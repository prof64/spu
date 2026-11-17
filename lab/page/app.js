// ---------------------------------------------------------------- engine
const canvas = document.getElementById('view');
const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
const $ = (s) => document.querySelector(s);
function fail(msg){ const e = $('#fail'); e.hidden = false; e.textContent = msg; }
if(!gl) fail('This page needs WebGL 2, which this browser does not offer.');

function program(vs, fs){
  const p = gl.createProgram();
  for(const [type, src] of [[gl.VERTEX_SHADER, vs], [gl.FRAGMENT_SHADER, fs]]){
    const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s);
    if(!gl.getShaderParameter(s, gl.COMPILE_STATUS)){ const log = gl.getShaderInfoLog(s); console.error(log); fail('Shader error: ' + log); }
    gl.attachShader(p, s);
  }
  gl.bindAttribLocation(p, 0, 'aPos'); gl.linkProgram(p);
  if(!gl.getProgramParameter(p, gl.LINK_STATUS)) console.error(gl.getProgramInfoLog(p));
  const cache = {};
  return { p, u: (n) => cache[n] ?? (cache[n] = gl.getUniformLocation(p, n)) };
}
function texture(w, h, data, filter){
  const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, data);
  for(const k of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, k, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}
function target(w, h, n, filter){
  const fbo = gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
  const tex = []; const bufs = [];
  for(let i = 0; i < n; i++){ tex.push(texture(w, h, null, filter)); gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0 + i, gl.TEXTURE_2D, tex[i], 0); bufs.push(gl.COLOR_ATTACHMENT0 + i); }
  return { fbo, tex, w, h, bufs };
}
function into(t){
  gl.bindFramebuffer(gl.FRAMEBUFFER, t ? t.fbo : null);
  if(t){ gl.drawBuffers(t.bufs); gl.viewport(0, 0, t.w, t.h); } else gl.viewport(0, 0, canvas.width, canvas.height);
}
function bind(prog, name, tex, unit){ gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(prog.u(name), unit); }

let P, T;
if(gl){
  P = { part: program(VS_PART, FS_PART), bg: program(VS_FULL, FS_BG), lit: program(VS_FULL, FS_LIT), comp: program(VS_FULL, FS_COMP), blur: program(VS_FULL, FS_BLUR), out: program(VS_FULL, FS_OUT) };
  T = { g: target(SW, SH, 2, gl.NEAREST), g2: target(SW, SH, 2, gl.NEAREST), lit2: target(SW, SH, 2, gl.NEAREST), bg: target(SW, SH, 1, gl.NEAREST), lit: target(SW, SH, 2, gl.NEAREST), comp: target(SW, SH, 2, gl.NEAREST),
        b1a: target(SW, SH, 1, gl.LINEAR), b1b: target(SW, SH, 1, gl.LINEAR), b2a: target(SW/2, SH/2, 1, gl.LINEAR), b2b: target(SW/2, SH/2, 1, gl.LINEAR) };
  gl.bindTexture(gl.TEXTURE_2D, T.comp.tex[1]); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  const vb = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, vb);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  gl.disable(gl.BLEND); gl.disable(gl.DEPTH_TEST);
}
const quad = () => gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);

// ---------------------------------------------------------------- data
const hex = (c) => [c[0] / 255, c[1] / 255, c[2] / 255];
const RAMP_W = 260, ROWS = 64;      // room for a material's shades (plus one darker, one lighter) and for the rows
// Read a sprite's .spu file (see spu.js) and make its textures.
async function load(sp){
  if(sp.ready) return sp;
  const bytes = sp.bytes || Uint8Array.from(atob(sp.file), (c) => c.charCodeAt(0));
  const model = SPU.model(await SPU.read(bytes)), wanted = sp.views;
  Object.assign(sp, model, { title: sp.title || model.title });
  if(wanted) sp.views = Object.fromEntries(Object.entries(model.views).filter(([id]) => wanted.includes(id)));
  for(const v of Object.values(sp.views)){
    v.texColor = texture(v.atlas[0], v.atlas[1], v.rawColor, gl.NEAREST); v.texLight = texture(v.atlas[0], v.atlas[1], v.rawLight, gl.NEAREST);
    v.texRamp = texture(RAMP_W, ROWS, null, gl.NEAREST); v.texPal = texture(256, 1, null, gl.NEAREST); v.texMat = texture(ROWS, 1, null, gl.NEAREST);
    Object.assign(v, { palettes: sp.palettes, title: sp.title, ground: sp.ground, glowZ: sp.glowZ, pal: null, ready: true });
    useAnim(v, 0);
  }
  sp.ready = true; return sp;
}
// a view plays one of its animations at a time
function useAnim(v, i){ const an = v.anims[i]; Object.assign(v, { animIndex: i, buf: an.buf, starts: an.starts, offsets: an.offsets, length: an.length, keys: an.keys }); }
function syncAnim(){ const v = current(); if(!v.anims) return; const i = v.anims.findIndex((an) => an.id === S.animId); useAnim(v, i < 0 ? 0 : i); }
// the palette in use: a base one chosen by a look, or its shiny twin
function palId(v){
  const base = S.palBase && v.palettes[S.palBase] ? S.palBase : 'normal'; if(!S.shiny) return base;
  const twin = base === 'normal' ? 'shiny' : base + '-shiny'; return v.palettes[twin] ? twin : base;
}
function rampColors(sp, palette){
  const pal = sp.palettes[palette].map(hex);
  return sp.rows.map((r) => {
    const c = r.ramp.map((i) => pal[i]); if(!c.length) return [];          // a material with no shades is only ever tinted
    const under = c.length === 1 ? [c[0][0] * 0.62, c[0][1] * 0.60, c[0][2] * 0.72] : c[0].map((v, j) => Math.max(0, Math.min(1, v - (c[1][j] - v) * 0.8)) * 0.9);
    const top = c[c.length - 1], over = top.map((v) => v + (1 - v) * 0.35);
    return [under, ...c, over];
  });
}
function setPalette(sp, palette){
  if(sp.pal === palette) return; sp.pal = palette;
  const pal = new Uint8Array(256 * 4); sp.palettes[palette].slice(0, 256).forEach((c, i) => pal.set([c[0], c[1], c[2], 255], i * 4));
  gl.bindTexture(gl.TEXTURE_2D, sp.texPal); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 256, 1, gl.RGBA, gl.UNSIGNED_BYTE, pal);
  const ramp = new Uint8Array(RAMP_W * ROWS * 4);
  rampColors(sp, palette).forEach((cols, r) => cols.forEach((c, j) => ramp.set([c[0] * 255, c[1] * 255, c[2] * 255, 255], (r * RAMP_W + j) * 4)));
  gl.bindTexture(gl.TEXTURE_2D, sp.texRamp); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, RAMP_W, ROWS, gl.RGBA, gl.UNSIGNED_BYTE, ramp);
}
// the material table as the lighting reads it: finish, body and how many shades, one texel per row
function setMaterials(sp){
  const m = new Uint8Array(ROWS * 4);
  // S.plain: every material matte and solid, to show shading before materials
  sp.rows.forEach((r, i) => m.set([Math.round(((S.plain ? 0 : r.f) * .5 + .5) * 255), Math.round(((S.plain ? 0 : r.b) * .5 + .5) * 255), r.ramp.length, r.base], i * 4));
  gl.bindTexture(gl.TEXTURE_2D, sp.texMat); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, ROWS, 1, gl.RGBA, gl.UNSIGNED_BYTE, m);
}

// ---------------------------------------------------------------- places
const mix3 = (a, b, k) => a.map((v, i) => v + (b[i] - v) * k), sstep = (a, b, x) => { const t = Math.max(0, Math.min(1, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
const norm3 = (v) => { const l = Math.hypot(...v); return v.map((x) => x / l); };
const PLACES = {
  studio: { id: 0, name: 'Studio', hint: 'Move the lamp.',
            ambSky: [.50, .52, .56], ambGnd: [.33, .32, .36], envTop: [.95, .97, 1.0], envHor: [.60, .62, .66], envGnd: [.40, .39, .40],
            lamp: { c: [1, .96, .9], k: .9, r: 115, z: 60 }, glow: .55, glowGain: 1.1, reflect: .16, bloom: .5,
            path: (t) => [96 + 68 * Math.cos(t * .45), 46 + 24 * Math.sin(t * .6)] },
  cave:   { id: 1, name: 'Cave', hint: 'Carry the torch. Anything that glows lights the rock too.',
            ambSky: [.085, .105, .18], ambGnd: [.055, .05, .07], envTop: [.05, .06, .10], envHor: [.12, .09, .08], envGnd: [.05, .045, .045],
            lamp: { c: [1, .62, .30], k: 1.5, r: 76, z: 34, flicker: true }, glow: 1.15, glowGain: 1.2, reflect: .05, bloom: .8,
            path: (t) => [96 - 70 * Math.cos(t * .33), 72 + 10 * Math.sin(t * .8)] },
  rain:   { id: 2, name: 'Rain', hint: 'Rain wets whatever faces up. Lightning strikes every few seconds.',
            ambSky: [.36, .42, .52], ambGnd: [.17, .19, .24], envTop: [.50, .57, .68], envHor: [.26, .30, .37], envGnd: [.09, .11, .14],
            lamp: { c: [1, .78, .46], k: 1.1, r: 64, z: 30 }, far: () => ({ dir: [.1, .9, -.3], c: [.26, .31, .40] }), storm: true, glow: 1.0, glowGain: 1.15, wet: 1, reflect: .6, bloom: .7,
            path: (t) => [96 + 66 * Math.sin(t * .3), 74 + 7 * Math.sin(t * .7)] },
  field:  { id: 3, name: 'Field', hint: 'Drag the sun. Take it down to the horizon for dusk.',
            glow: .8, glowGain: 1.1, reflect: 0, bloom: .55,
            path: (t) => [96 - 82 * Math.cos(t * .2), 46 - 44 * Math.sin(t * .2)],
            live(){                                      // the whole place follows how high the sun is
              const day = Math.max(-.2, Math.min(1, (HORIZON - S.lamp[1]) / 72)), up = sstep(-.12, .08, day), warm = sstep(0, .5, day);
              const blend = (night, dusk, noon) => mix3(mix3(night, dusk, up), noon, warm);
              return { day, ambSky: blend([.10, .12, .22], [.42, .36, .46], [.55, .62, .75]), ambGnd: blend([.06, .07, .12], [.25, .20, .22], [.36, .40, .30]),
                       envTop: blend([.06, .08, .18], [.35, .30, .50], [.45, .65, .95]), envHor: blend([.10, .10, .20], [1, .55, .30], [.80, .90, 1]), envGnd: blend([.04, .06, .05], [.25, .20, .12], [.30, .45, .20]),
                       far: { dir: norm3([(S.lamp[0] - 96) / 80, .12 + Math.max(day, 0) * 1.2, S.behind ? -.6 : .35]), c: mix3([1, .5, .22], [1, .97, .9], warm).map((v) => v * .9 * up) } };
            } },
  snow:   { id: 4, name: 'Snow', hint: 'Drag the moon. Snow settles on whatever faces up.',
            ambSky: [.17, .22, .36], ambGnd: [.12, .15, .24], envTop: [.12, .18, .36], envHor: [.25, .40, .55], envGnd: [.55, .62, .80],
            snow: 1, glow: 1.0, glowGain: 1.15, reflect: .08, bloom: .7,
            path: (t) => [96 + 74 * Math.sin(t * .22), 30 + 12 * Math.cos(t * .31)],
            live(){ return { far: { dir: norm3([(S.lamp[0] - 96) / 80, .25 + Math.max(0, Math.min(1, (HORIZON - S.lamp[1]) / 72)) * 1.1, S.behind ? -.6 : .4]), c: [.48, .60, .92] } }; } },
  deep:   { id: 5, name: 'Deep', hint: 'Light ripples down through the water. Carry the diver’s lamp.',
            ambSky: [.16, .34, .44], ambGnd: [.07, .14, .20], envTop: [.35, .70, .80], envHor: [.08, .30, .42], envGnd: [.20, .26, .24],
            lamp: { c: [1, .95, .8], k: 1.0, r: 60, z: 30 }, far: () => ({ dir: [.15, 1, .2], c: [.34, .60, .68] }), caustic: 1, glow: 1.0, glowGain: 1.15, reflect: 0, bloom: .7,
            path: (t) => [96 + 70 * Math.sin(t * .28), 66 + 14 * Math.sin(t * .6)] },
};
const noise1 = (x) => { const i = Math.floor(x), f = x - i, h = (n) => { const s = Math.sin(n * 127.1) * 43758.5453; return s - Math.floor(s); }; return h(i) + (h(i + 1) - h(i)) * f * f * (3 - 2 * f); };

// ---------------------------------------------------------------- state and drawing
// t: the place's clock (rain, flicker).  anim: where the sprite is in its animation, in seconds.
const S = { ball: null, sprite: 0, view: 'front', place: 'cave', tab: 'light', mode: 0, shiny: false, palBase: 'normal', looks: {}, animId: 'idle', blink: false, paused: false, speed: 1, behind: false, glowLights: true, explode: 0, explodeAlways: false, plain: false, noGlow: false,
            t: 0, anim: 0, lamp: [40, 70], pointer: null, pointerAt: -1e9, fixed: null, calm: false };
const Lp = new Float32Array(NL * 3), Lc = new Float32Array(NL * 3), Lr = new Float32Array(NL);
const current = () => S.ball ? BALL : (DATA.sprites[S.sprite].views[S.view] || Object.values(DATA.sprites[S.sprite].views)[0]);

function lights(sp, place, t, off, ox, oy){
  Lp.fill(0); Lc.fill(0); Lr.fill(0);
  const L = place.lamp;
  if(L){ const fl = L.flicker ? 0.82 + 0.3 * noise1(Math.floor(t * 14) * 0.7) : 1;
    Lp.set([S.lamp[0], S.lamp[1], S.behind ? -L.z * .7 : L.z], 0); Lc.set(L.c.map((v) => v * L.k * fl), 0); Lr[0] = L.r; }
  else Lp.set([S.lamp[0], S.lamp[1], 1], 0);                     // no lamp here: the pointer moves the sun or moon
  const pal = palId(sp);
  if(S.glowLights) for(let j = 0; j < 3; j++){
    const x = sp.buf[off + j * 4], y = sp.buf[off + j * 4 + 1], e = sp.buf[off + j * 4 + 2], d = sp.buf[off + j * 4 + 3];
    if(!(e > 0)) continue;
    const c = sp.glowCols[d][pal], m = Math.max(c[0], c[1], c[2], .01);
    const I = Math.max(.3, Math.min(1.3, Math.sqrt(e / 80))) * place.glow * (0.86 + 0.22 * noise1(Math.floor(t * 12) * 0.9 + j * 5));
    Lp.set([ox + x, oy + y, sp.glowZ], (j + 1) * 3); Lc.set(c.map((v) => (v / m * .72 + .28) * I), (j + 1) * 3); Lr[j + 1] = 30 + 26 * I;
  }
  if(place.far){ Lp.set(place.far.dir, 12); Lc.set(place.far.c, 12); }
  let flash = 0;
  if(place.storm){
    const tau = (t + 2.2) % 9; flash = tau < .07 ? 1 : tau < .15 ? .15 : tau < .34 ? .75 * (1 - (tau - .15) / .19) : 0;
    Lp.set([.7, .45, -.55], 15); Lc.set([.75 * 2.4 * flash, .85 * 2.4 * flash, 2.4 * flash], 15);
  }
  return flash;
}
function common(prog, place, t){
  gl.uniform1f(prog.u('uTime'), t); gl.uniform1i(prog.u('uScene'), place.id);
  gl.uniform3fv(prog.u('uLp'), Lp); gl.uniform3fv(prog.u('uLc'), Lc); gl.uniform1fv(prog.u('uLr'), Lr);
}
function tickNow(sp){ return Math.floor(S.anim * sp.tps) % sp.length; }
function draw(){
  const sp = current(); if(!sp.ready) return;
  const base = PLACES[S.place], live = base.live ? base.live() : {}, place = { ...base, ...live }, t = S.t;
  if(typeof place.far === 'function') place.far = place.far();
  setPalette(sp, palId(sp));
  // which drawing each part shows, and where, at this tick
  const tick = tickNow(sp);
  let f = 0; for(let lo = 0, hi = sp.starts.length - 1; lo <= hi;){ const m = (lo + hi) >> 1; if(sp.starts[m] <= tick){ f = m; lo = m + 1; } else hi = m - 1; }
  const off = sp.offsets[f], count = sp.buf[off + 12];
  const ox = Math.round(SW / 2 - (sp.bbox[0] + sp.bbox[2]) / 2), oy = GROUND - sp.ground;
  const cx = (sp.bbox[0] + sp.bbox[2]) / 2, cy = (sp.bbox[1] + sp.bbox[3]) / 2, apart = S.tab === 'parts' || S.explodeAlways ? S.explode : 0;
  const swap = {}; for(const l of sp.looks || []) if(S.looks[l.id]) Object.assign(swap, l.swaps);      // looks that swap drawings
  const shut = S.blink || (S.tab !== 'looks' && sp.blink && (S.t % 3.7) < .13);          // the eyes-closed look, used as a blink
  gl.useProgram(P.part.p); bind(P.part, 'uColor', sp.texColor, 0); bind(P.part, 'uLight', sp.texLight, 1);
  gl.uniform2f(P.part.u('uOrigin'), ox, oy);
  for(const layer of sp.soft ? [0, 1] : [0]){                      // solid pixels, then soft see-through ones if the sprite has any
    into(layer ? T.g2 : T.g); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); gl.uniform1i(P.part.u('uLayer'), layer);
    for(let i = 0; i < count; i++){
      const o = off + 13 + i * 8; let di = sp.buf[o]; if(swap[di] !== undefined) di = swap[di]; if(shut && sp.blink && sp.blink[di] !== undefined) di = sp.blink[di];
      const d = sp.drawings[di], a = sp.buf[o + 1], b = sp.buf[o + 2], c = sp.buf[o + 3], e = sp.buf[o + 4], s = Math.sqrt(Math.abs(a * e - b * c)) || 1;
      let tx = sp.buf[o + 5], ty = sp.buf[o + 6];
      if(apart){ const lx = d[4] + d[2] / 2, ly = d[5] + d[3] / 2; tx += (a * lx + c * ly + tx - cx) * apart * .75; ty += (b * lx + e * ly + ty - cy) * apart * .75; }
      gl.uniform4f(P.part.u('uRect'), d[0], d[1], d[2], d[3]); gl.uniform2f(P.part.u('uOff'), d[4], d[5]);
      gl.uniform4f(P.part.u('uM'), a, b, c, e); gl.uniform2f(P.part.u('uT'), apart ? Math.round(tx) : tx, apart ? Math.round(ty) : ty);
      gl.uniform1f(P.part.u('uPart'), sp.buf[o + 7]);
      gl.uniformMatrix2fv(P.part.u('uRot'), false, [a / s, b / s, c / s, e / s]);
      quad();
    }
  }
  const flash = lights(sp, place, t, off, ox, oy);
  setMaterials(sp);
  // the room
  into(T.bg); gl.useProgram(P.bg.p); common(P.bg, place, t); bind(P.bg, 'uG0', T.g.tex[0], 0); bind(P.bg, 'uG1', T.g.tex[1], 1); bind(P.bg, 'uMat', sp.texMat, 2);
  gl.uniform3fv(P.bg.u('uAmbSky'), place.ambSky); gl.uniform3fv(P.bg.u('uAmbGnd'), place.ambGnd); gl.uniform1f(P.bg.u('uFlash'), flash); gl.uniform1f(P.bg.u('uGrid'), S.ball ? 1 : 0);
  gl.uniform1f(P.bg.u('uDay'), place.day ?? 1); gl.uniform2f(P.bg.u('uSun'), S.lamp[0], S.lamp[1]); quad();
  // the sprite: its solid pixels, then its soft ones
  gl.useProgram(P.lit.p); common(P.lit, place, t);
  bind(P.lit, 'uBg', T.bg.tex[0], 2); bind(P.lit, 'uRamp', sp.texRamp, 3); bind(P.lit, 'uPal', sp.texPal, 4); bind(P.lit, 'uMat', sp.texMat, 5);
  for(const k of ['ambSky', 'ambGnd', 'envTop', 'envHor', 'envGnd']) gl.uniform3fv(P.lit.u('u' + k[0].toUpperCase() + k.slice(1)), place[k]);
  gl.uniform1f(P.lit.u('uWet'), place.wet || 0); gl.uniform1f(P.lit.u('uSnow'), place.snow || 0); gl.uniform1f(P.lit.u('uCaustic'), place.caustic || 0);
  gl.uniform1f(P.lit.u('uPure'), S.pure ? 1 : 0); gl.uniform1f(P.lit.u('uNoGlow'), S.noGlow ? 1 : 0); gl.uniform1f(P.lit.u('uGain'), 1.0); gl.uniform1f(P.lit.u('uGlowGain'), place.glowGain); gl.uniform1i(P.lit.u('uMode'), S.mode);
  for(const layer of sp.soft ? [0, 1] : [0]){
    const g = layer ? T.g2 : T.g; into(layer ? T.lit2 : T.lit); gl.useProgram(P.lit.p);
    bind(P.lit, 'uG0', g.tex[0], 0); bind(P.lit, 'uG1', g.tex[1], 1); gl.uniform1f(P.lit.u('uSoft'), layer); quad();
  }
  // together
  into(T.comp); gl.useProgram(P.comp.p); common(P.comp, place, t);
  bind(P.comp, 'uBg', T.bg.tex[0], 0); bind(P.comp, 'uLit', T.lit.tex[0], 1); bind(P.comp, 'uEmis', T.lit.tex[1], 2);
  bind(P.comp, 'uLit2', T.lit2.tex[0], 3); bind(P.comp, 'uEmis2', T.lit2.tex[1], 4); gl.uniform1f(P.comp.u('uSoft'), sp.soft ? 1 : 0);
  gl.uniform1f(P.comp.u('uReflect'), place.reflect); gl.uniform1f(P.comp.u('uFlash'), flash); gl.uniform1f(P.comp.u('uWet'), place.wet || 0); gl.uniform1f(P.comp.u('uShowLamp'), S.mode === 0 && place.lamp ? 1 : 0); quad();
  // glow spreading into the air
  const blur = (src, dst, dx, dy) => { into(dst); gl.useProgram(P.blur.p); bind(P.blur, 'uTex', src, 0); gl.uniform2f(P.blur.u('uInv'), 1 / dst.w, 1 / dst.h); gl.uniform2f(P.blur.u('uStep'), dx, dy); quad(); };
  blur(T.comp.tex[1], T.b1a, 1 / SW, 0); blur(T.b1a.tex[0], T.b1b, 0, 1 / SH);
  blur(T.b1b.tex[0], T.b2a, 2.5 / SW, 0); blur(T.b2a.tex[0], T.b2b, 0, 2.5 / SH);
  into(null); gl.useProgram(P.out.p); bind(P.out, 'uScene', T.comp.tex[0], 0); bind(P.out, 'uB1', T.b1b.tex[0], 1); bind(P.out, 'uB2', T.b2b.tex[0], 2);
  gl.uniform2f(P.out.u('uRes'), canvas.width, canvas.height); gl.uniform1f(P.out.u('uBloom'), S.mode === 0 || S.mode === 4 ? place.bloom : S.mode === 3 ? 0.3 : 0); quad();
}

let last = 0, onFrame = () => {};
function frame(now){
  const dt = Math.min(0.05, (now - last) / 1000); last = now;
  if(S.fixed === null){
    S.t += dt; if(!S.paused) S.anim += dt * S.speed;
    const idle = now - S.pointerAt > 3500 && !S.pointer && !S.calm;
    const want = S.pointer || (idle ? PLACES[S.place].path(S.t) : S.lamp), k = S.pointer ? 0.5 : 0.04;
    S.lamp = [S.lamp[0] + (want[0] - S.lamp[0]) * k, S.lamp[1] + (want[1] - S.lamp[1]) * k];
  }
  draw(); onFrame();
  requestAnimationFrame(frame);
}
function resize(){
  const w = canvas.clientWidth, dpr = window.devicePixelRatio || 1;
  const scale = Math.max(1, Math.round(w * dpr / SW));
  if(canvas.width !== SW * scale){ canvas.width = SW * scale; canvas.height = SH * scale; }
}
