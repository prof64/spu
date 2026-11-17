// ---------------------------------------------------------------- two dials, and a test ball to try them on
// finish: -1 fuzzy .. 0 matte .. +1 polished.   body: -1 metal .. 0 solid .. 0.5 translucent .. +1 clear.
// The lighting derives everything else from these two (see the sprite shader).
const FINISH = [[1, 'Polished'], [.5, 'Satin'], [0, 'Matte'], [-1, 'Fuzzy']];
const BODY = [[-1, 'Metal'], [0, 'Solid'], [.25, 'Waxy'], [.5, 'Translucent'], [.8, 'Cloudy'], [1, 'Clear']];
const finishWord = (f) => f < -.3 ? 'fuzzy' : f < .3 ? 'matte' : f < .75 ? 'satin' : 'polished';
const bodyWord = (b) => b < -.5 ? 'metal' : b < .12 ? 'solid' : b < .4 ? 'waxy' : b < .65 ? 'translucent' : b < .92 ? 'cloudy' : 'clear';
const MONO = ['#6b3a31', '#a5634a', '#d69473', '#f7c9a5'];     // one clay color for the whole map, so only the dials differ
const NAMED = [
  { name: 'Scales',        on: 'Charizard',  c: ['#843119', '#ce5242', '#ef8429', '#efb55a'], f: .5, b: .05 },
  { name: 'Soft skin',     on: 'Charizard',  c: ['#b5736b', '#de9c8c', '#f7c5ad', '#ffe6d6'], f: .4, b: .25 },
  { name: 'Wing membrane', on: 'Charizard',  c: ['#084252', '#217394', '#219cc5', '#6bcee6'], f: .45, b: .5 },
  { name: 'Horn',          on: 'Charizard',  c: ['#6b6352', '#a59c84', '#d6cead', '#f7efd6'], f: .7, b: .05 },
  { name: 'Wet eye',       on: 'Magneton',   c: ['#101831', '#29427b', '#4273c5', '#8cb5f7'], f: .9, b: 0 },
  { name: 'Steel',         on: 'Magneton',   c: ['#4a525a', '#84949c', '#b5c5ce', '#e6eff7'], f: .7, b: -1 },
  { name: 'Dark iron',     on: 'Chandelure', c: ['#212129', '#42424a', '#6b6b73', '#9c9ca5'], f: .45, b: -1 },
  { name: 'Glossy paint',  on: 'Magneton',   c: ['#6b1010', '#b52121', '#ef4231', '#ff8c73'], f: .8, b: 0 },
  { name: 'Ice',           on: 'Cryogonal',  c: ['#3142a5', '#5263c5', '#8cadff', '#cedeff'], f: .94, b: .82 },
  { name: 'Glass',         on: 'Chandelure', c: ['#7b8c8c', '#a5b5b5', '#c5d6d6', '#ffffff'], f: .97, b: 1 },
  { name: 'Fur',           on: 'Arcanine',   c: ['#6b5229', '#9c7b4a', '#c5a573', '#e6ce9c'], f: -.7, b: .05 },
  { name: 'Plasma',        on: 'Rotom',      c: ['#7b3a21', '#c54a19', '#ef7329', '#ffad84'], f: .75, b: .3, glow: [0, .12, .25, .6] },
  { name: 'Chrome',        c: ['#39424a', '#7b8c94', '#bdced6', '#ffffff'], f: .95, b: -1 },
  { name: 'Gold',          c: ['#7b5208', '#bd8c10', '#efc531', '#fff39c'], f: .8, b: -1 },
  { name: 'Water',         c: ['#104a7b', '#2173b5', '#4aa5e6', '#a5e0ff'], f: .98, b: 1 },
  { name: 'Slime',         c: ['#1a6b31', '#31a542', '#6bd65a', '#c5f79c'], f: .9, b: .45 },
  { name: 'Snow',          c: ['#8494b5', '#b5c5de', '#dee6f7', '#ffffff'], f: .4, b: .58 },
  { name: 'Wax',           c: ['#a56b31', '#d69c52', '#f7ce84', '#fff0c5'], f: .5, b: .35 },
  { name: 'Leaf',          c: ['#1a4a19', '#317b29', '#5aad42', '#9cd66b'], f: .4, b: .4 },
  { name: 'Velvet',        c: ['#3a104a', '#6b218c', '#9c42c5', '#c57bef'], f: -1, b: 0 },
  { name: 'Plastic',       c: ['#846b00', '#c5a500', '#f7d621', '#fff78c'], f: .65, b: 0 },
  { name: 'Wet rock',      c: ['#292931', '#4a4a52', '#73737b', '#a5a5ad'], f: .75, b: 0 },
  { name: 'Stone',         c: ['#524a42', '#7b7363', '#a59c8c', '#cec5b5'], f: .1, b: 0 },
  { name: 'Ghost gas',     c: ['#31195a', '#5a31a5', '#8c5ae6', '#c5a5ff'], f: .7, b: .95, glow: [.2, .35, .5, .7] },
];
const MAP = FINISH.flatMap(([f, fn]) => BODY.map(([b, bn]) => ({ name: fn + ' ' + bn.toLowerCase(), c: MONO, f, b })));
const BR = 23, BW = 48;
const BALL = { id: 'ball', title: '', atlas: [64, 64], drawings: [[1, 1, BW, BW, -BW / 2, -BW]], bbox: [-BW / 2, -BW, BW / 2, 0], ground: 0, glowZ: 8,
               starts: [0], offsets: [0], length: 1, tps: 60, buf: new Float32Array(24), parts: ['ball'], names: ['ball'], blink: null, looks: [], palettes: { normal: [] }, glowCols: [{ normal: [1, 1, 1] }], rows: [], pal: null, ready: false };
const ballLevel = new Uint8Array(BW * BW), ballLight = new Uint8Array(64 * 64 * 4);
function buildBall(){
  // a ball painted the way a pixel artist would, four shades, lit from the upper left
  const color = new Uint8Array(64 * 64 * 4), L = [-0.5099, 0.6119, 0.6323];
  for(let v = 0; v < BW; v++) for(let u = 0; u < BW; u++){
    const dx = u + .5 - BW / 2, dy = v + .5 - BW / 2, d = Math.hypot(dx, dy); if(d > BR + .2) continue;
    const o = ((v + 1) * 64 + u + 1) * 4, line = d > BR - .9;
    const nx = Math.min(1, dx / BR), ny = Math.max(-1, Math.min(1, -dy / BR)), nz = Math.sqrt(Math.max(0, 1 - nx * nx - ny * ny));
    const lit = nx * L[0] + ny * L[1] + nz * L[2], k = line ? 0 : lit > .93 ? 3 : lit > .45 ? 2 : lit > .08 ? 1 : 0;
    ballLevel[v * BW + u] = line ? 255 : k;
    color.set([line ? 1 : 2 + k, k, 0, 255], o);
    ballLight.set([Math.round(nx * 127 + 128), Math.round(ny * 127 + 128), line ? 0 : 1, 0], o);
  }
  BALL.texColor = texture(64, 64, color, gl.NEAREST); BALL.texLight = texture(64, 64, ballLight, gl.NEAREST);
  BALL.texRamp = texture(RAMP_W, ROWS, null, gl.NEAREST); BALL.texPal = texture(256, 1, null, gl.NEAREST); BALL.texMat = texture(ROWS, 1, null, gl.NEAREST);
  BALL.buf.set([0, 1, 0, 0, 1, 0, 0, 0], 13); BALL.buf[12] = 1; BALL.ready = true;
}
const rgb = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
function setBall(m){
  if(!BALL.ready) buildBall();
  const shades = m.c.map(rgb), pal = Array.from({ length: 16 }, () => [0, 0, 0]);
  pal[1] = shades[0].map((v) => Math.round(v * .3)); shades.forEach((c, k) => { pal[2 + k] = c; });
  BALL.palettes.normal = pal; BALL.pal = null; BALL.title = m.name;
  BALL.rows = [{ name: 'outline', lib: 'outline', ramp: [1], base: 0, glow: false, f: .1, b: 0 },
               { name: m.name, lib: m.name, ramp: [2, 3, 4, 5], base: 2, glow: !!m.glow, f: m.f, b: m.b }];
  let e = 0; const sum = [0, 0, 0];
  for(let v = 0; v < BW; v++) for(let u = 0; u < BW; u++){
    const k = ballLevel[v * BW + u], g = m.glow && k < 4 ? m.glow[k] : 0;
    ballLight[((v + 1) * 64 + u + 1) * 4 + 3] = Math.round(g * 255);
    if(g > 0){ e += g; shades[k].forEach((c, j) => { sum[j] += c / 255 * g; }); }
  }
  gl.bindTexture(gl.TEXTURE_2D, BALL.texLight); gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 64, 64, gl.RGBA, gl.UNSIGNED_BYTE, ballLight);
  BALL.buf.set([0, -BW / 2, e * .25, 0], 0); BALL.glowCols = [{ normal: e ? sum.map((v) => v / e) : [1, 1, 1] }];
}
// one small still of each ball, drawn by the same lighting as everything else
function ballThumbs(list){
  const keep = { ...S }, shot = document.createElement('canvas'), k = canvas.width / SW; shot.width = shot.height = 96;
  const ctx = shot.getContext('2d'); ctx.imageSmoothingEnabled = false;
  Object.assign(S, { place: 'studio', tab: 'light', mode: 0, behind: false, lamp: [58, 30], t: 0 });
  const out = list.map((m) => { S.ball = m; setBall(m); draw(); ctx.drawImage(canvas, (SW / 2 - 30) * k, (GROUND - 53) * k, 60 * k, 60 * k, 0, 0, 96, 96); return shot.toDataURL('image/png'); });
  Object.assign(S, keep); if(S.ball) setBall(S.ball);
  return out;
}
