// ---------------------------------------------------------------- shaders
const SW = 192, SH = 128, GROUND = 108, HORIZON = 86, WALLZ = -44, NL = 6;
const COMMON = `#version 300 es
precision highp float;
precision highp int;
const float SW = ${SW}.0, SH = ${SH}.0, GROUND = ${GROUND}.0, HORIZON = ${HORIZON}.0, WALLZ = ${WALLZ}.0;
uniform float uTime;
uniform int uScene;                 // 0 studio, 1 cave, 2 rain
uniform vec3 uLp[${NL}];            // point: scene x, scene y (down), z toward the viewer.  directional: x, y (up), z
uniform vec3 uLc[${NL}];
uniform float uLr[${NL}];           // reach in pixels; 0 or less means a distant light
float hash(float n){ return fract(sin(n*127.1)*43758.5453); }
float hash2(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7)))*43758.5453); }
float vnoise(vec2 p){ vec2 i = floor(p), f = fract(p); f = f*f*(3.0-2.0*f);
  return mix(mix(hash2(i), hash2(i+vec2(1,0)), f.x), mix(hash2(i+vec2(0,1)), hash2(i+vec2(1,1)), f.x), f.y); }
float fbm(vec2 p){ float a = 0.5, s = 0.0; for(int i=0;i<4;i++){ s += a*vnoise(p); p = p*2.03+7.1; a *= 0.5; } return s; }
float bayer(vec2 p){ ivec2 i = ivec2(mod(p, 4.0)); int b[16] = int[16](0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5); return (float(b[i.y*4+i.x])+0.5)/16.0; }
vec2 scenePos(){ return vec2(gl_FragCoord.x, SH - gl_FragCoord.y); }
vec4 fetchS(sampler2D t, vec2 p){ if(p.x<0.0||p.y<0.0||p.x>=SW||p.y>=SH) return vec4(0.0); return texelFetch(t, ivec2(int(p.x), int(SH) - 1 - int(p.y)), 0); }
float caust(vec2 q, float t){        // the net of bright lines sunlight makes under moving water
  vec2 a = q*0.085 + vec2(t*0.22, t*0.09);
  float v = sin(a.x*3.1 + sin(a.y*2.3 + t)) + sin(a.y*3.7 + sin(a.x*1.9 - t*0.8)) + 0.6*sin((a.x + a.y)*2.7 + t*1.3);
  return smoothstep(0.9, 2.0, v); }
float falloff(float dist, float r){ float q = dist/r; return 1.0/(1.0+q*q) * smoothstep(4.0, 2.0, q); }
`;

const VS_FULL = `#version 300 es
in vec2 aPos; void main(){ gl_Position = vec4(aPos*2.0-1.0, 0.0, 1.0); }`;

// -- 1. parts into two buffers: (color index, shade) and (surface direction, material, glow)
const VS_PART = `#version 300 es
in vec2 aPos;
uniform vec4 uRect; uniform vec2 uOff; uniform vec4 uM; uniform vec2 uT; uniform vec2 uOrigin;
out vec2 vUV;
void main(){
  vec2 l = uOff + aPos*uRect.zw;
  vec2 w = vec2(uM.x*l.x + uM.z*l.y, uM.y*l.x + uM.w*l.y) + uT + uOrigin;
  vUV = uRect.xy + aPos*uRect.zw;
  gl_Position = vec4(w.x/${SW}.0*2.0-1.0, 1.0 - w.y/${SH}.0*2.0, 0.0, 1.0);
}`;
const FS_PART = `#version 300 es
precision highp float;
uniform sampler2D uColor, uLight; uniform vec4 uRect; uniform mat2 uRot; uniform float uPart; uniform int uLayer;
in vec2 vUV; layout(location=0) out vec4 o0; layout(location=1) out vec4 o1;
void main(){
  ivec2 t = ivec2(clamp(floor(vUV), uRect.xy, uRect.xy + uRect.zw - 1.0));
  vec4 c = texelFetch(uColor, t, 0); if(c.a <= 0.0) discard;                  // the color sheet's last channel is the pixel's opacity
  // layer 0 keeps solid pixels. Layer 1 keeps the soft, partly see-through ones that sit in front of them:
  // a solid pixel drawn later wipes whatever soft pixel was there.
  bool solid = c.a > 0.98;
  if(uLayer == 0 && !solid) discard;
  if(uLayer == 1 && solid){ o0 = vec4(0.0); o1 = vec4(0.0); return; }
  vec4 l = texelFetch(uLight, t, 0);
  vec2 n = (l.xy*255.0 - 128.0)/127.0;
  vec2 r = uRot * vec2(n.x, -n.y);            // a part that turns or mirrors turns its surface with it
  // b: a fixed random number per texel (for fur and glitter), or for a soft pixel its opacity
  o0 = vec4(c.r, c.g, uLayer == 1 ? c.a : fract(sin(dot(vec2(t), vec2(12.9898, 78.233)))*43758.5453), (uPart + 1.0)/255.0);   // b: a fixed random number per texel, for fur
  o1 = vec4(vec2(r.x, -r.y)*0.5 + 0.5, l.b, l.a);
}`;

// -- 2. the room behind the sprite, lit by the same lights, with the sprite's shadow
const FS_BG = COMMON + `
uniform sampler2D uG0, uG1, uMat;
uniform vec3 uAmbSky, uAmbGnd;
uniform float uFlash, uGrid, uDay, uBackdrop;
uniform vec2 uSun;
out vec4 o;
vec2 occl(vec2 q){                  // x: how much of the light is blocked here, y: how much of that is see-through material
  vec2 r = vec2(0.0);
  for(int j=0;j<4;j++){ vec2 qq = q + (j == 0 ? vec2(0.0) : j == 1 ? vec2(1.5, 0.0) : j == 2 ? vec2(-1.5, 0.0) : vec2(0.0, 1.5));
    if(fetchS(uG0, qq).a > 0.0){ float body = texelFetch(uMat, ivec2(int(fetchS(uG1, qq).b*255.0 + 0.5), 0), 0).g*2.0 - 1.0;
      r += vec2(0.25, 0.25*max(0.0, 2.0*body - 1.0)*0.85); } }
  return r; }
// what stands between light i and a point of the room at (x, height above floor, depth)
vec2 shadowed(int i, vec3 w){
  if(i > 1 || uLr[i] <= 0.0) return vec2(0.0);
  vec3 L = vec3(uLp[i].x, GROUND - uLp[i].y, uLp[i].z);
  float den = L.z - w.z; if(abs(den) < 0.5) return vec2(0.0);
  float u = L.z/den; if(u <= 0.02 || u >= 0.999) return vec2(0.0);
  vec3 c = L + (w - L)*u;
  return occl(vec2(c.x, GROUND - c.y));
}
vec2 shadowedFar(vec3 d, vec3 w){   // the same, for a sun or moon
  if(abs(d.z) < 0.04) return vec2(0.0);
  float u = -w.z/d.z; if(u <= 0.0) return vec2(0.0);
  vec3 c = w + d*u; if(c.y < 0.0) return vec2(0.0);
  return occl(vec2(c.x, GROUND - c.y));
}
vec3 lightAt(vec3 w, vec3 n, float soft){
  vec3 e = vec3(0.0);
  for(int i=0;i<${NL};i++){
    if(uLc[i] == vec3(0.0)) continue;
    if(uLr[i] <= 0.0){ vec3 d = normalize(uLp[i]); vec2 sh = (i == 4 && uScene >= 3) ? shadowedFar(d, w) : vec2(0.0);
      e += uLc[i]*max(dot(n, d), 0.0)*0.8*(1.0 - 0.75*(sh.x - 0.8*sh.y)); continue; }
    vec3 d = vec3(uLp[i].x, GROUND - uLp[i].y, uLp[i].z) - w; float dist = length(d);
    float c = max(dot(n, d/dist), 0.0);
    vec2 sh = shadowed(i, w);
    e += uLc[i]*falloff(dist, uLr[i]*1.25)*mix(c, 1.0, soft)*(1.0 - 0.8*(sh.x - 0.8*sh.y) + 1.5*sh.y*smoothstep(0.7, 1.0, sh.x));
  }
  return e;
}
vec3 dith(vec3 c, vec2 p, float steps){ return floor(c*steps + bayer(p))/steps; }
void main(){
  vec2 p = scenePos(); float t = uTime;
  bool floor_ = p.y >= HORIZON;
  vec3 w, n; if(floor_){ w = vec3(p.x, 0.0, (p.y - GROUND)/0.5); n = vec3(0,1,0); } else { w = vec3(p.x, GROUND - p.y, WALLZ); n = vec3(0,0,1); }
  vec3 col; float emis = 0.0;
  if(uScene == 0){                                   // studio: a seamless paper sweep
    vec3 alb = mix(vec3(0.40, 0.43, 0.50), vec3(0.52, 0.52, 0.55), floor_ ? 1.0 : clamp(p.y/HORIZON, 0.0, 1.0));
    // behind a test ball the backdrop is chequered, so anything see-through has something to bend
    if(uGrid > 0.0) alb *= 1.0 + 0.16*(mod(floor(p.x/8.0) + floor((floor_ ? w.z : p.y)/8.0), 2.0) - 0.5);
    float crease = floor_ ? smoothstep(0.0, 14.0, p.y - HORIZON) : 1.0 - smoothstep(0.0, 30.0, HORIZON - p.y)*0.0;
    vec3 amb = mix(uAmbGnd, uAmbSky, floor_ ? 0.85 : 0.6)*0.55;
    if(!floor_) n = normalize(vec3(0.0, mix(0.9, 0.0, smoothstep(0.0, 26.0, HORIZON - p.y)), 1.0));
    col = alb*(amb + lightAt(w, n, 0.25)*0.9);
    col *= floor_ ? mix(0.9, 1.0, crease) : 1.0;
    col = dith(col, p, 72.0);
  } else if(uScene == 1){                            // cave: flowstone wall, flat rock floor
    vec3 alb, nn; vec3 amb = mix(uAmbGnd, uAmbSky, 0.6);
    if(!floor_){
      vec2 q = vec2(p.x*0.045, p.y*0.012) + 3.0;
      float rib = fbm(q), rib2 = fbm(q + vec2(0.07, 0.0));
      vec2 q2 = vec2(p.x*0.02, p.y*0.07) + 11.0;
      float led = fbm(q2), led2 = fbm(q2 + vec2(0.0, 0.1));
      float tone = floor((rib*0.7 + led*0.3)*6.0)/6.0;
      alb = mix(vec3(0.15, 0.14, 0.18), vec3(0.44, 0.39, 0.38), tone);
      nn = normalize(vec3((rib - rib2)*10.0, (led2 - led)*6.0, 1.0));
    } else {
      float f = fbm(vec2(p.x*0.05, w.z*0.08) + 7.0);
      float stones = step(0.74, vnoise(vec2(p.x*0.33, w.z*0.45)));
      alb = mix(vec3(0.14, 0.13, 0.16), vec3(0.33, 0.29, 0.29), floor(f*5.0)/5.0)*(1.0 + 0.3*stones);
      nn = vec3(0.0, 1.0, 0.0);
    }
    col = alb*(amb + lightAt(w, nn, 0.12));
    if(!floor_){
      // spikes growing up from the back of the floor, and hanging from the roof
      float ci = floor((p.x + 9.0)/31.0), cx = (ci + 0.25 + 0.5*hash(ci + 2.0))*31.0 - 9.0;
      float up = (4.0 + 20.0*pow(hash(ci + 6.0), 2.0))*(1.0 - abs(p.x - cx)/(3.0 + 5.0*hash(ci + 8.0)));
      if(p.y > HORIZON - up) col = alb*0.6*(amb + lightAt(vec3(w.x, w.y, WALLZ + 14.0), normalize(vec3(sign(p.x - cx)*0.7, 0.0, 0.7)), 0.0));
      float cj = floor(p.x/23.0), dx = (cj + 0.3 + 0.4*hash(cj + 12.0))*23.0;
      float down = (6.0 + 38.0*pow(hash(cj + 19.0), 2.2))*(1.0 - abs(p.x - dx)/(2.5 + 5.5*hash(cj + 14.0)));
      float roof = 7.0 + 5.0*vnoise(vec2(p.x*0.07, 1.0));
      if(p.y < max(roof, down)) col = vec3(0.028, 0.028, 0.042) + alb*lightAt(vec3(w.x, w.y, -16.0), normalize(vec3(sign(p.x - dx)*0.6, -0.3, 0.7)), 0.0)*0.10;
    }
    col = dith(col, p, 48.0);
  } else if(uScene == 2){                            // rain: dusk, far hills, standing water
    float sky = clamp(p.y/HORIZON, 0.0, 1.0);
    vec3 skyc = mix(vec3(0.07, 0.09, 0.16), vec3(0.27, 0.30, 0.38), pow(sky, 1.6));
    skyc += vec3(0.10, 0.09, 0.07)*smoothstep(0.75, 1.0, sky);
    float cl = fbm(vec2(p.x*0.025 + t*0.02, p.y*0.06)); skyc *= 0.8 + 0.35*cl;
    skyc += vec3(0.55, 0.62, 0.75)*uFlash*(0.5 + 0.6*cl);
    if(!floor_){
      col = skyc;
      float r1 = HORIZON - 16.0 - 22.0*fbm(vec2(p.x*0.018, 1.0));
      float r2 = HORIZON - 5.0 - 13.0*fbm(vec2(p.x*0.04, 4.0)) - 3.0*step(0.55, hash(floor(p.x/2.0)))*hash(floor(p.x/2.0)+3.0);
      if(p.y > r1) col = mix(vec3(0.13, 0.16, 0.23), skyc, 0.35) + vec3(0.2,0.24,0.3)*uFlash*0.4;
      if(p.y > r2) col = vec3(0.06, 0.08, 0.11) + vec3(0.1,0.12,0.16)*uFlash*0.3;
    } else {
      float d = (p.y - HORIZON)/(SH - HORIZON);
      vec3 base = mix(vec3(0.09, 0.11, 0.15), vec3(0.05, 0.065, 0.085), d);
      float rip = vnoise(vec2(p.x*0.08, p.y*0.9 - t*0.6));
      vec3 mir = mix(vec3(0.27, 0.30, 0.38), vec3(0.10, 0.12, 0.18), d) * (0.7 + 0.5*rip) + vec3(0.5,0.56,0.7)*uFlash*0.5;
      col = mix(base, mir, 0.5 - 0.25*d);
      col += vec3(0.42, 0.46, 0.52)*lightAt(w, n, 0.0)*0.5;
      // the lamp's own streak of reflection on the water
      for(int i=0;i<4;i++){ if(uLr[i] <= 0.0 || uLc[i] == vec3(0.0)) continue;
        float hh = GROUND - uLp[i].y; float ry = GROUND + (hh*0.5 + uLp[i].z*0.5)*0.55;
        float sx = abs(p.x - uLp[i].x + (rip-0.5)*5.0); float sy = abs(p.y - ry);
        col += uLc[i]*0.35*smoothstep(4.0+sy*0.2, 0.0, sx)*smoothstep(26.0, 0.0, sy)*(0.5+0.8*rip); }
      // rings where drops land
      vec2 cell = floor(vec2(p.x, w.z)/vec2(22.0, 26.0)); float ph = fract(t*0.9 + hash2(cell)*7.0);
      vec2 cc = (cell + vec2(0.3 + 0.4*hash2(cell+1.0), 0.3 + 0.4*hash2(cell+2.0)))*vec2(22.0, 26.0);
      float rr = length((vec2(p.x, w.z) - cc)*vec2(1.0, 1.0)); float ring = abs(rr - ph*9.0);
      col += vec3(0.25, 0.29, 0.36)*step(ring, 0.7)*(1.0 - ph)*step(hash2(cell + floor(t*0.9 + hash2(cell)*7.0)), 0.6);
    }
    col = dith(col, p, 44.0);
  } else if(uScene == 3){                            // field: the sky follows the sun down
    float up = smoothstep(-0.12, 0.08, uDay), warm = smoothstep(0.0, 0.5, uDay);
    vec3 zen = mix(mix(vec3(0.03, 0.04, 0.11), vec3(0.20, 0.20, 0.42), up), vec3(0.20, 0.48, 0.90), warm);
    vec3 hor = mix(mix(vec3(0.08, 0.09, 0.20), vec3(1.0, 0.50, 0.24), up), vec3(0.70, 0.87, 0.98), warm);
    if(!floor_){
      float h = clamp(p.y/HORIZON, 0.0, 1.0);
      col = mix(zen, hor, pow(h, 1.6));
      float sd = length(p - uSun);
      col += mix(vec3(1.0, 0.55, 0.25), vec3(1.0, 0.95, 0.8), warm)*smoothstep(26.0, 0.0, sd)*0.4*up;
      if(sd < 5.5) col = mix(vec3(1.0, 0.72, 0.40), vec3(1.0, 1.0, 0.92), warm);
      float st = hash2(floor(p)); if(st > 0.992) col += vec3(0.8, 0.85, 1.0)*(1.0 - up)*(0.5 + 0.5*sin(t*3.0 + st*90.0));
      float cl = fbm(vec2(p.x*0.028 + t*0.012, p.y*0.10));
      float cm = smoothstep(0.50, 0.60, cl)*smoothstep(70.0, 44.0, p.y)*smoothstep(4.0, 14.0, p.y);
      col = mix(col, mix(hor*1.1 + 0.08, vec3(1.0), warm*0.85)*(0.3 + 0.7*up), cm*0.8);
      float r1 = HORIZON - 9.0 - 13.0*fbm(vec2(p.x*0.02, 2.0)), r2 = HORIZON - 2.0 - 8.0*fbm(vec2(p.x*0.045, 6.0));
      if(p.y > r1) col = mix(vec3(0.26, 0.42, 0.44)*(0.2 + 0.8*up), hor, 0.42);
      if(p.y > r2) col = vec3(0.15, 0.35, 0.19)*(0.16 + 0.84*up)*mix(vec3(1.25, 0.8, 0.6), vec3(1.0), warm);
    } else {
      float f = fbm(vec2(p.x*0.07, w.z*0.12) + 5.0);
      vec3 alb = mix(vec3(0.18, 0.42, 0.16), vec3(0.42, 0.62, 0.22), floor(f*4.0)/4.0);
      alb *= 1.0 + 0.25*step(0.8, hash2(floor(vec2(p.x, p.y*0.5))));
      if(hash2(floor(vec2(p.x/2.0, w.z/4.0))) > 0.988) alb = mix(alb, vec3(0.95, 0.9, 0.5), 0.8);
      col = alb*(mix(uAmbGnd, uAmbSky, 0.75) + lightAt(w, n, 0.0));
    }
    col = dith(col, p, 56.0);
  } else if(uScene == 4){                            // snow: a moon, an aurora, pines
    if(!floor_){
      float h = clamp(p.y/HORIZON, 0.0, 1.0);
      col = mix(vec3(0.02, 0.03, 0.09), vec3(0.10, 0.16, 0.30), pow(h, 1.4));
      float st = hash2(floor(p)); if(st > 0.988) col += vec3(0.7, 0.8, 1.0)*(0.4 + 0.6*sin(t*2.5 + st*80.0))*0.8;
      float cur = fbm(vec2(p.x*0.035 + 0.6*sin(p.y*0.06 + t*0.3), t*0.08));
      float band = smoothstep(0.42, 0.68, cur)*smoothstep(6.0, 20.0, p.y)*smoothstep(66.0, 30.0, p.y);
      col += mix(vec3(0.10, 0.75, 0.45), vec3(0.45, 0.25, 0.75), smoothstep(20.0, 55.0, p.y))*band*(0.6 + 0.4*sin(p.x*1.3 + sin(t*0.7 + p.x*0.2)*2.0))*0.6;
      float md = length(p - uSun); col += vec3(0.5, 0.6, 0.9)*smoothstep(20.0, 0.0, md)*0.3;
      if(md < 5.0) col = vec3(0.92, 0.95, 1.0) - 0.12*step(0.55, vnoise(p*0.6));
      float r1 = HORIZON - 10.0 - 14.0*fbm(vec2(p.x*0.02, 9.0)); if(p.y > r1) col = vec3(0.15, 0.20, 0.33);
      float cell = floor(p.x/11.0), th = 8.0 + 13.0*hash(cell + 4.0), tx = p.x - (cell*11.0 + 3.0 + 5.0*hash(cell)), ty = HORIZON - p.y;
      if(ty < th && abs(tx) < (0.8 + (th - ty)*0.27)*(0.55 + 0.45*step(0.5, fract(ty*0.33)))) col = vec3(0.04, 0.07, 0.11);
    } else {
      float f = fbm(vec2(p.x*0.05, w.z*0.07) + 2.0);
      vec3 alb = mix(vec3(0.62, 0.70, 0.88), vec3(0.90, 0.94, 1.0), floor(f*4.0)/4.0);
      col = alb*(mix(uAmbGnd, uAmbSky, 0.8)*1.1 + lightAt(w, normalize(vec3((f - 0.5)*0.6, 1.0, 0.0)), 0.0));
      float gr = hash2(floor(p)); if(gr > 0.985 && fract(gr*57.0 + uLp[4].x*2.0 + t*0.2) < 0.4) col += vec3(0.5, 0.55, 0.65);
    }
    col = dith(col, p, 56.0);
  } else {                                           // deep: light comes down through moving water
    if(!floor_){
      float h = clamp(p.y/HORIZON, 0.0, 1.0);
      col = mix(vec3(0.10, 0.45, 0.60), vec3(0.02, 0.10, 0.22), pow(h, 0.8));
      col += vec3(0.25, 0.5, 0.5)*smoothstep(0.5, 0.8, fbm(vec2((p.x + p.y*0.45)*0.05, t*0.15)))*(1.0 - h)*0.7;
      float r1 = HORIZON - 6.0 - 22.0*pow(fbm(vec2(p.x*0.03, 12.0)), 2.0); if(p.y > r1) col = mix(vec3(0.03, 0.13, 0.20), col, 0.35);
      float kx = floor(p.x/9.0), up = HORIZON - p.y, kh = (10.0 + 26.0*hash(kx + 2.0))*step(0.45, hash(kx));
      if(up < kh && abs(mod(p.x, 9.0) - 4.5 - sin(t*1.2 + kx + up*0.15)*2.0*up/30.0) < 1.4) col = vec3(0.02, 0.12, 0.12);
    } else {
      float f = fbm(vec2(p.x*0.06, w.z*0.09) + 8.0);
      vec3 alb = mix(vec3(0.34, 0.36, 0.30), vec3(0.62, 0.60, 0.46), floor(f*4.0)/4.0);
      col = alb*(mix(uAmbGnd, uAmbSky, 0.7) + lightAt(w, n, 0.0)*(0.5 + 1.4*caust(vec2(p.x, w.z), t)));
    }
    col = dith(col, p, 56.0);
  }
  // a plain backdrop for when the pieces are laid out as a sheet
  if(uBackdrop > 0.0) col = mix(col, vec3(0.070, 0.080, 0.110) + 0.012*mod(floor(p.x/8.0) + floor(p.y/8.0), 2.0), uBackdrop);
  o = vec4(col, emis);
}`;

// -- 3. light the sprite
const FS_LIT = COMMON + `
uniform sampler2D uG0, uG1, uBg, uRamp, uPal, uMat;
uniform float uSoft;
uniform vec3 uAmbSky, uAmbGnd, uEnvTop, uEnvHor, uEnvGnd;
uniform float uWet, uGain, uGlowGain, uSnow, uCaustic, uPure, uNoGlow;
uniform int uMode;
layout(location=0) out vec4 o0; layout(location=1) out vec4 o1;
const vec3 L0 = vec3(-0.5099, 0.6119, 0.6323);       // the light the art was painted under
vec3 shade(int row, float k, float n){ int j = clamp(int(floor(k + 0.5)), -1, int(n)) + 1; return texelFetch(uRamp, ivec2(j, row), 0).rgb; }
vec3 env(vec3 R){ float y = R.y;     // what a mirror facing that way would show: sky, a dark horizon, the floor
  vec3 c = y > 0.6 ? uEnvTop : y > 0.22 ? mix(uEnvTop, uEnvHor, 0.55) : y > 0.0 ? uEnvHor*0.8 : y > -0.2 ? uEnvHor*0.14 : y > -0.6 ? uEnvGnd*0.8 : uEnvGnd*0.45;
  return c*(0.88 + 0.24*step(0.0, R.x)); }
vec3 hue(float h){ return clamp(abs(fract(h + vec3(0.0, 0.667, 0.333))*6.0 - 3.0) - 1.0, 0.0, 1.0); }
void main(){
  ivec2 ip = ivec2(gl_FragCoord.xy); vec2 p = scenePos();
  vec4 g0 = texelFetch(uG0, ip, 0);
  if(g0.a <= 0.0){ o0 = vec4(0.0); o1 = vec4(0.0); return; }
  vec4 g1 = texelFetch(uG1, ip, 0);
  int idx = int(g0.r*255.0 + 0.5); float k = floor(g0.g*255.0 + 0.5);
  int row = int(g1.b*255.0 + 0.5); float glow = g1.a*(1.0 - uNoGlow);
  vec2 nxy = (g1.rg*255.0 - 128.0)/127.0; vec3 N = vec3(nxy, sqrt(max(0.0, 1.0 - dot(nxy, nxy))));
  vec4 pc = texelFetch(uPal, ivec2(idx, 0), 0); vec3 painted = pc.rgb;
  float opacity = uSoft > 0.0 ? g0.b : 1.0, rnd = uSoft > 0.0 ? hash2(floor(p)) : g0.b;   // a soft pixel carries its own opacity
  if(uMode == 1 && uPure <= 0.0){ o0 = vec4(painted, opacity); o1 = vec4(0.0); return; }
  if(uMode == 2){ o0 = vec4(N*0.5 + 0.5, 1.0); o1 = vec4(0.0); return; }
  // a material is two dials; everything the lighting needs is worked out from them
  vec4 mt = texelFetch(uMat, ivec2(row, 0), 0);
  float finish = mt.r*2.0 - 1.0, body = mt.g*2.0 - 1.0, n = floor(mt.b*255.0 + 0.5), cl = max(0.0, body);
  if(uMode == 4){ o0 = vec4(painted*(0.07 + 0.93*glow), 1.0); o1 = vec4(painted*glow, 1.0); return; }
  if(uMode == 5){ o0 = vec4(hue((g0.a*255.0 - 1.0)*0.618 + 0.05)*(0.35 + 0.8*dot(painted, vec3(0.3, 0.5, 0.2))), 1.0); o1 = vec4(0.0); return; }
  if(uMode == 3){ vec3 c = row == 0 ? vec3(0.08) : mix(vec3(0.25), hue(float(row)*0.618 + 0.05), 0.75)*(0.55 + 0.45*(k + 1.0)/(n + 0.5)); o0 = vec4(c, 1.0); o1 = vec4(c*glow*0.6, 1.0); return; }
  // "light alone": forget the color the artist painted. Every pixel starts from its material's base shade
  // and light decides how light or dark it is, from the facing data. If the facing data is right, this
  // gives the same picture as starting from the painted color.
  bool pure = uPure > 0.0 && row != 0 && glow < 0.05;
  if(pure){ k = floor(mt.a*255.0 + 0.5); painted = shade(row, k, n); }
  if(uMode == 1){ o0 = vec4(painted, opacity); o1 = vec4(0.0); return; }      // lighting off: the art itself, or its flat base colors
  float rough = 1.0 - max(0.0, finish), fuzz = max(0.0, -finish), metal = max(0.0, -body);
  float trans = max(0.0, 2.0*cl - 1.0)*0.85, ss = cl > 0.0 ? 1.0 - abs(2.0*cl - 1.0) : 0.0, coat = 0.0, ior = 1.4, depth = 0.5, rough0 = rough;
  bool line = row == 0 || n < 0.5;          // outlines, and any material that lists no shades: tinted by light, never re-shaded
  // rain soaks whatever faces the sky
  float wet = uWet*smoothstep(-0.2, 0.5, N.y);
  float soak = mix(1.0, 0.80, uWet*smoothstep(0.5, 0.9, rough0));
  coat = max(coat, wet*0.85); rough = mix(rough, rough*0.5, wet);
  // how many shades the light may move a pixel: all of a short list, about half of a long one
  float G = line ? 0.0 : uGain*max(2.0, (n - 1.0)*mix(1.0, 0.45, smoothstep(3.0, 9.0, n)))*(1.0 - 0.45*ss);
  // A surface facing the viewer head-on shows its base shade. Turning toward the light climbs to the
  // lightest shade, turning away falls to the darkest, and every shade has room on the way.
  float base = floor(mt.a*255.0 + 0.5);
  float Gup = line ? 0.0 : max(G, (n - 0.5 - base)/0.32), Gdn = line ? 0.0 : max(G, (base + 0.5)/1.2);
  // fuzz: the pile catches light edge-on and swallows it face-on, the reverse of a smooth surface
  float pile = fuzz*(N.z < 0.5 ? 1.5 : N.z < 0.72 ? 0.7 : N.z > 0.9 ? -0.7 : 0.0);
  float e0 = dot(N, L0) - 0.6, art = e0*(e0 > 0.0 ? Gup : Gdn), lev0 = pure ? 0.0 : art;   // art: shades the art's own light adds here
  float kFill = pure ? k + art : k;                             // fill light comes from where the art's light did
  if(pure) painted = shade(row, kFill, n);
  // fur: break the edges of each band so it reads as tufts
  float tuft = (rnd - 0.5)*fuzz*1.3;
  vec3 amb = mix(uAmbGnd, uAmbSky, N.y*0.5 + 0.5);
  vec3 dif = (fuzz > 0.0 ? shade(row, kFill + pile*0.6 + tuft*0.5, n) : painted)*amb, spec = vec3(0.0), sss = vec3(0.0), sheen = vec3(0.0), inner = vec3(0.0), flat_ = amb;
  vec3 tint = shade(row, n - 1.0, n);
  float f0 = pow((ior - 1.0)/(ior + 1.0), 2.0);
  vec3 R = vec3(2.0*N.z*N.x, 2.0*N.z*N.y, 2.0*N.z*N.z - 1.0), mirror = vec3(0.0);
  for(int i=0;i<${NL};i++){
    if(uLc[i] == vec3(0.0)) continue;
    vec3 Lv; float att = 1.0; float behind = 0.4;
    if(uLr[i] <= 0.0){ Lv = normalize(uLp[i]); behind = Lv.z < 0.0 ? 1.0 : 0.4; }
    else { vec3 d = vec3(uLp[i].x - p.x, p.y - uLp[i].y, uLp[i].z); float dist = length(d); Lv = d/dist; att = falloff(dist, uLr[i]); behind = uLp[i].z < 0.0 ? 1.0 : 0.4; }
    vec3 lc = uLc[i]*att*((i >= 1 && i <= 3) ? 0.6 : 1.0);   // its own glow is already painted into the art
    if(uCaustic > 0.0 && i == 4) lc *= mix(1.0, 0.45 + 1.4*caust(vec2(p.x, p.y*1.6), uTime), smoothstep(-0.3, 0.4, N.y));
    vec3 ld = mix(vec3(dot(lc, vec3(0.3, 0.5, 0.2))), lc, 0.7);
    flat_ += ld*0.5;
    if(line) continue;
    float D = dot(N, Lv);
    float wrap = max(ss*0.8, fuzz*0.4), Dw = (D + wrap)/(1.0 + wrap);
    float amt = Dw <= 0.02 ? 0.0 : Dw < 0.30 ? 0.36 : Dw < 0.62 ? 0.70 : 1.0;
    float e1 = D - 0.6;
    dif += shade(row, k + e1*(e1 > 0.0 ? Gup : Gdn) - lev0 + tuft + pile, n)*ld*amt;
    vec3 H = normalize(Lv + vec3(0.0, 0.0, 1.0)); float nh = max(dot(N, H), 0.0);
    // roughness: a small hard highlight when smooth, a wide faint one in between, none when rough
    float s = pow(nh, mix(300.0, 4.0, sqrt(rough))), str = pow(1.0 - rough, 1.5)*(fuzz > 0.3 ? 0.0 : 1.0);
    spec += lc*(s > 0.6 ? 1.0 : s > 0.25 ? 0.4 : 0.0)*str*mix(vec3(0.9), tint*1.5, metal);
    // metal: a mirror image of each lamp, sharp when polished, a broad sheen when dull
    if(metal > 0.0){ float rl = pow(max(dot(R, Lv), 0.0), mix(50.0, 2.5, rough));
      mirror += lc*((rl > 0.6 ? 1.4 : rl > 0.25 ? 0.6 : rl > 0.06 ? 0.25 : 0.0)*(1.0 - 0.5*rough) + (D < 0.3 ? 0.0 : D < 0.6 ? 0.25 : 0.55)*(0.3 + 0.7*rough)); }
    // coat: its own crisp white highlight on top, whatever is underneath
    float sc = pow(nh, 170.0); spec += lc*coat*(sc > 0.5 ? 1.2 : sc > 0.15 ? 0.35 : 0.0);
    // light-through, seen from the lit side: the edge of the shadow and the shadow itself glow from inside
    sss += lc*ss*(D > 0.3 ? 0.0 : D > -0.15 ? 0.6 : D > -0.55 ? 0.35 : 0.15);
    float back = clamp(0.35 - D*0.75, 0.0, 1.0);
    back = back < 0.25 ? 0.0 : back < 0.6 ? 0.5 : 1.0;
    sss += lc*ss*back*behind;
    if(trans > 0.0){
      // light that enters clear material comes out again on the side facing away from the lamp
      float far = clamp(0.1 - D*1.2, 0.0, 1.0);
      inner += lc*trans*(0.2 + (far < 0.3 ? 0.0 : far < 0.65 ? 0.45 : 0.9));
      // clear and cloudy inside at once means grains: single ones flash, different ones as the lamp or the part
      // moves. Glass (nothing inside) gets none, and a wet coat smooths them over. No value of its own is needed.
      if(fract(rnd*61.0 + dot(Lv.xy, vec2(2.3, 3.7))) < 0.1*ss*sqrt(trans)*(1.0 - rough)*(1.0 - coat)*(1.0 - coat)) spec += lc*1.5;
    }
    float rim = pow(1.0 - N.z, 1.5);
    sheen += lc*fuzz*(rim > 0.45 ? 0.6 : rim > 0.25 ? 0.25 : 0.0)*step(-0.3, D);
  }
  vec3 col;
  if(line){ col = painted*flat_ + flat_*0.03; }
  else {
    // light through flesh comes out deeper and warmer than the surface color
    vec3 deep = mix(painted/max(max(painted.r, painted.g), max(painted.b, 0.05)), vec3(1.0, 0.5, 0.3), 0.3)*0.85;
    col = dif*soak + sss*deep + sheen*mix(painted, vec3(1.0), 0.4);
    vec3 e = env(R);
    // a polished or wet surface shows a deeper color than a matte one
    float deepen = max(coat, smoothstep(0.55, 0.95, 1.0 - rough)*(1.0 - metal)*(1.0 - trans));
    col = mix(vec3(dot(col, vec3(0.3, 0.5, 0.2))), col, 1.0 + 0.3*deepen)*mix(1.0, 0.82, deepen);
    if(metal > 0.0){
      // the room itself, as a mirror ball would show it; duller metal blurs it
      vec2 rp = p + vec2(R.x, -R.y)*64.0; vec3 room = vec3(0.0);
      for(int j=0;j<5;j++){ vec2 o = (j == 0 ? vec2(0.0) : j == 1 ? vec2(1.0, 0.4) : j == 2 ? vec2(-1.0, -0.4) : j == 3 ? vec2(0.4, -1.0) : vec2(-0.4, 1.0))*rough*18.0;
        room += fetchS(uBg, clamp(rp + o, vec2(0.5), vec2(SW - 0.5, SH - 0.5))).rgb; }
      col = mix(col, (e*0.62 + room*0.11)*tint*(1.1 - 0.35*rough) + mirror*tint*0.85 + dif*(0.12 + 0.3*rough), metal*0.92);
    }
    float fres = pow(1.0 - N.z, 2.5);
    if(trans > 0.0){
      // the room seen through it: bent by the surface, each color bent a little differently, tinted by depth
      vec2 off = N.xy*vec2(1.0, -1.0)*(ior - 1.0)*(30.0 + 70.0*depth);
      vec2 lo = vec2(0.5), hi = vec2(SW - 0.5, SH - 0.5);
      vec3 through = vec3(fetchS(uBg, clamp(p - off*0.7, lo, hi)).r, fetchS(uBg, clamp(p - off, lo, hi)).g, fetchS(uBg, clamp(p - off*1.35, lo, hi)).b);
      vec3 tn = mix(vec3(1.0), painted*1.2, 0.45 + 0.4*depth);
      col = mix(col, through*tn + col*0.55, trans*(1.0 - 0.6*fres));
      col += inner*mix(painted/max(max(painted.r, painted.g), max(painted.b, 0.05)), vec3(1.0), 0.4)*0.8;
      // edges turned away from the eye mirror the sky and the lamps
      col += trans*(fres > 0.5 ? 1.0 : fres > 0.22 ? 0.4 : 0.0)*(e*0.7 + (flat_ - amb)*0.9)*(1.0 - rough);
    }
    float edge = fres > 0.5 ? 1.0 : fres > 0.22 ? 0.4 : 0.0;                    // smooth and coated things mirror the room at their edges
    col += coat*edge*e*0.6 + (1.0 - metal)*(1.0 - trans)*edge*e*0.6*(1.0 - rough)*(1.0 - rough);
    col += spec;
    // raindrops bursting on whatever faces up
    if(uWet > 0.0 && N.y > 0.25 && fract(rnd*53.0 + uTime*1.7) < 0.03 && fract(rnd*17.0) < 0.5) col += vec3(0.32, 0.37, 0.45);
  }
  // snow settles on whatever faces up
  if(uSnow > 0.0 && N.y > 0.5 && rnd > 0.22 && glow < 0.2) col = mix(col, vec3(0.80, 0.87, 1.0)*min(amb*1.5 + (flat_ - amb)*1.3 + 0.12, vec3(1.15)), 0.8);
  vec3 emit = painted*glow;
  col = col*(1.0 - 0.9*glow) + emit*uGlowGain;
  o0 = vec4(col, opacity);
  o1 = vec4(emit*uGlowGain*0.8 + max(col - 1.0, 0.0)*0.4, 1.0);
}`;

// -- 4. put it together: reflection, sprite, weather in front
const FS_COMP = COMMON + `
uniform sampler2D uBg, uLit, uEmis, uLit2, uEmis2;
uniform float uReflect, uFlash, uWet, uShowLamp, uSoft;
layout(location=0) out vec4 o0; layout(location=1) out vec4 o1;
float streak(vec2 p, float t, float layer){
  float cw = 3.0 + layer; vec2 q = vec2(p.x + p.y*0.26 + layer*31.0, p.y);
  float c = floor(q.x/cw); float h = hash(c + layer*17.0);
  float sp = 190.0 + 110.0*h + layer*60.0, per = 70.0 + 90.0*hash(c + 5.0);
  float y = mod(q.y - t*sp + h*300.0, per);
  return (fract(q.x/cw) < 1.0/cw && y < 5.0 + 7.0*h + layer*3.0) ? 1.0 : 0.0; }
void main(){
  vec2 p = scenePos(); float t = uTime;
  vec4 bg = fetchS(uBg, p); vec3 col = bg.rgb; vec3 em = vec3(0.0);
  if(uReflect > 0.0 && p.y >= GROUND - 1.0){
    float wob = uScene == 2 ? sin(p.y*1.7 + t*4.0)*1.3 + (vnoise(vec2(p.x*0.2, p.y*0.8 - t*1.5)) - 0.5)*2.5 : 0.0;
    vec2 m = vec2(p.x + wob, 2.0*GROUND - p.y - 1.0);
    vec4 r = fetchS(uLit, m); vec3 re = fetchS(uEmis, m).rgb;
    float fade = uReflect*smoothstep(46.0, 0.0, p.y - GROUND);
    col = mix(col, r.rgb*0.75, r.a*fade); em += re*r.a*fade*0.7;
  }
  if(uScene == 2) col = mix(col, vec3(0.50, 0.57, 0.68), 0.16*streak(p, t, 2.0));
  vec4 s = fetchS(uLit, p); vec3 se = fetchS(uEmis, p).rgb;
  if(uSoft > 0.0){ vec4 s2 = fetchS(uLit2, p); vec3 e2 = fetchS(uEmis2, p).rgb;      // soft pixels blend over the solid ones beneath
    float oa = s2.a + s.a*(1.0 - s2.a); s = vec4(oa > 0.0 ? (s2.rgb*s2.a + s.rgb*s.a*(1.0 - s2.a))/oa : vec3(0.0), oa); se = mix(se, e2, s2.a); }
  if(uLp[0].z >= 0.0){ col = mix(col, s.rgb, s.a); em += se*s.a; }
  if(uShowLamp > 0.0){
    vec2 q = vec2(p.x - uLp[0].x, uLp[0].y - p.y);
    if(uScene == 1){                                  // a torch flame you carry around
      float tq = floor(t*14.0)/14.0; float yy = q.y + 5.0;
      float w = 4.2*(1.0 - yy/15.0)*smoothstep(-1.0, 3.0, yy);
      float wob = (vnoise(vec2(yy*0.4 - tq*7.0, tq*3.0)) - 0.5)*4.0*clamp(yy/15.0, 0.0, 1.0);
      float v = (yy > 0.0 && yy < 15.0) ? 1.0 - abs(q.x - wob)/max(w, 0.4) : -1.0;
      if(v > 0.0){ vec3 f = v > 0.55 ? vec3(1.0, 0.93, 0.62) : v > 0.25 ? vec3(1.0, 0.60, 0.14) : vec3(0.86, 0.26, 0.06); col = f; em += f*0.9; }
    } else if(uScene == 2){                           // a storm lamp
      if(abs(q.x) < 1.5 && abs(q.y) < 2.0){ col = vec3(1.0, 0.92, 0.7); em += vec3(1.0, 0.8, 0.5); }
      else if(abs(q.x) < 2.5 && abs(q.y) < 3.0){ col = vec3(0.16, 0.13, 0.11); }
    } else if(uScene == 0 || uScene == 5){            // a studio lamp, a diver's lamp
      if((abs(q.x) < 0.6 && abs(q.y) < 2.6) || (abs(q.y) < 0.6 && abs(q.x) < 2.6) || (abs(q.x) < 1.6 && abs(q.y) < 1.6)){ col = vec3(1.0, 0.98, 0.94); em += vec3(0.5); }
    }
  }
  if(uLp[0].z < 0.0){ col = mix(col, s.rgb, s.a); em = em*(1.0 - s.a) + se*s.a; }
  if(uScene == 1){                                    // sparks rising off the flames
    for(int i=0;i<4;i++){ if(uLr[i] <= 0.0 || uLc[i].r < uLc[i].b*1.2) continue;
      for(int j=0;j<3;j++){ float h = hash(float(i*7 + j)); float ph = fract(t*(0.35 + 0.2*h) + h);
        vec2 e = vec2(uLp[i].x + (h - 0.5)*14.0 + sin(ph*9.0 + h*6.0)*3.0, uLp[i].y - 6.0 - ph*34.0);
        if(floor(p) == floor(e) && ph < 0.85){ vec3 f = vec3(1.0, 0.6, 0.2)*(1.0 - ph); col += f; em += f*0.6; } } }
  }
  if(uScene == 2){
    float a = streak(p, t, 0.0)*0.38 + streak(p, t, 1.0)*0.26;
    col = mix(col, vec3(0.62, 0.70, 0.82), a);
    if(p.y > HORIZON){ vec2 c = floor(p/vec2(5.0, 3.0)); float ph = fract(t*2.3 + hash2(c)*9.0);
      if(hash2(c + floor(t*2.3 + hash2(c)*9.0)) < 0.07 && ph < 0.35 && mod(p.x, 5.0) < 1.0 + step(0.18, ph)*2.0 && mod(p.y, 3.0) < 1.0) col = mix(col, vec3(0.7, 0.78, 0.9), 0.55); }
    col += vec3(0.25, 0.28, 0.34)*uFlash*0.35;
  }
  if(uScene == 4){                                    // snowfall, two layers
    for(int j=0;j<2;j++){ float lj = float(j); vec2 cs = vec2(9.0 - lj*2.0, 11.0);
      vec2 q = vec2(p.x + sin(p.y*0.05 + t*0.6 + lj*3.0)*4.0, p.y - t*(14.0 + lj*10.0)); vec2 c = floor(q/cs); float h = hash2(c + lj*7.0);
      if(h > 0.35 && floor(q - c*cs) == floor(vec2(h, fract(h*13.7))*(cs - 2.0) + 1.0)) col = mix(col, vec3(0.9, 0.94, 1.0), 0.8 - 0.25*lj); }
  }
  if(uScene == 5){                                    // everything is seen through water; bubbles rise
    col = col*vec3(0.62, 0.86, 1.0) + vec3(0.0, 0.02, 0.05);
    for(int j=0;j<7;j++){ float h = hash(float(j)*3.7 + 1.0), ph = fract(t*(0.06 + 0.05*h) + h), r = 1.0 + 1.5*fract(h*7.3);
      float d = length(p - vec2(16.0 + h*160.0 + sin(ph*14.0 + h*9.0)*3.0, SH - ph*(SH + 10.0)));
      if(d < r + 0.5 && d > r - 0.6) col = mix(col, vec3(0.7, 0.95, 1.0), 0.6); }
  }
  float vg = smoothstep(0.55, 1.05, length((p/vec2(SW, SH) - 0.5)*vec2(1.25, 1.5)));
  col *= 1.0 - 0.45*vg*(uScene == 0 ? 0.6 : 1.0);
  o0 = vec4(col, 1.0); o1 = vec4(em + max(col - 0.92, 0.0)*0.6, 1.0);
}`;

const FS_BLUR = `#version 300 es
precision highp float; uniform sampler2D uTex; uniform vec2 uInv, uStep; out vec4 o;
void main(){ vec2 uv = gl_FragCoord.xy*uInv;
  vec4 c = texture(uTex, uv)*0.227;
  c += (texture(uTex, uv + uStep*1.385) + texture(uTex, uv - uStep*1.385))*0.316;
  c += (texture(uTex, uv + uStep*3.231) + texture(uTex, uv - uStep*3.231))*0.070;
  o = c; }`;

const FS_OUT = `#version 300 es
precision highp float; uniform sampler2D uScene, uB1, uB2; uniform vec2 uRes; uniform float uBloom; out vec4 o;
void main(){ vec2 uv = gl_FragCoord.xy/uRes;
  vec3 c = texture(uScene, uv).rgb;
  vec3 b = texture(uB1, uv).rgb*0.75 + texture(uB2, uv).rgb*0.9;
  c += b*uBloom; c = c/(1.0 + max(c - 0.85, 0.0)*0.6);
  o = vec4(c, 1.0); }`;
