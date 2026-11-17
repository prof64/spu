// Capture the frames of the tour video from the built lab page: out/video/*.png and out/video/frames.json.
// Then `python video.py` adds captions and encodes ../docs/spu-tour.mp4.
// The video builds one sprite up a layer at a time (parts, flat color, facing, shading, materials, glow),
// then shows other materials, other places and other looks.
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const fs = require('fs');
const FPS = 30, WIPE = 0.7;
const smooth = (x) => { x = Math.max(0, Math.min(1, x)); return x * x * (3 - 2 * x); };
const LAMP = {                                   // where the light is at time T, per place
  studio: (T) => [96 + 60 * Math.cos(T * 0.9), 32 + 12 * Math.sin(T * 1.3)],
  cave: (T) => [96 - 64 * Math.cos(T * 0.8), 68 + 12 * Math.sin(T * 1.1)],
  field: (T) => [96 + 62 * Math.sin(T * 0.6), 30 + 6 * Math.cos(T * 0.6)],
  rain: (T) => [96 + 60 * Math.sin(T * 0.8), 70 + 6 * Math.cos(T * 1.1)],
  snow: (T) => [96 + 60 * Math.sin(T * 0.6), 27],
  deep: (T) => [96 + 60 * Math.sin(T * 0.8), 62 + 8 * Math.cos(T)],
};
const BASE = { tab: 'light', view: 'front', mode: 0, shiny: false, palBase: 'normal', looks: {}, animId: 'idle', blink: false, behind: false, flat: false, pure: false,
               glowLights: true, explode: 0, explodeAlways: true, plain: false, noGlow: false };
const K = { sprite: 'klinklang', place: 'studio' };
const PALS = ['normal', 'fire', 'water', 'grass', 'electric', 'ghost', 'ice', 'dragon'];
// Each scene: how long, what the page shows at local time u, and its caption. Scenes wipe in over the one before unless cut.
const SCENES = [
  { d: 2.6, s: () => K, cap: 'One sprite, one .spu file.', sub: 'Klinklang: 86 KB holds its drawings, its motion and how every pixel takes light.', cut: true },
  { d: 2.4, s: (u) => ({ ...K, explode: 0.9 * smooth(u / 1.1) }), cap: 'It is built from parts.', sub: 'Eighteen here. Each part shows one drawing at a time.', cut: true },
  { d: 2.4, s: () => ({ ...K, explode: 0.9, mode: 5 }), cap: 'Each part moves, turns and swaps drawings on its own.', sub: 'Every part in its own color.' },
  { d: 1.6, s: (u) => ({ ...K, explode: 0.9 * (1 - smooth(u / 1.1)), mode: 5 }), cap: 'Each part moves, turns and swaps drawings on its own.', sub: 'Every part in its own color.', cut: true },
  { d: 3.0, s: () => ({ ...K, mode: 1, pure: true }), cap: 'What the artist draws: flat color.', sub: 'One palette number per pixel.' },
  { d: 3.0, s: () => ({ ...K, mode: 2 }), cap: 'Which way every pixel faces.', sub: 'Two more numbers per pixel. Green faces up, red faces right.' },
  { d: 4.0, s: () => ({ ...K, pure: true, plain: true, noGlow: true, glowLights: false }), cap: 'Flat color + facing + a light = shading.', sub: 'Light only moves a pixel between its material’s own shades, so it stays pixel art.' },
  { d: 2.8, s: () => ({ ...K, mode: 3 }), cap: 'Every pixel also names a material.', sub: 'Steel gears, a dark iron gear, a ring, and two glowing cores.' },
  { d: 4.0, s: () => ({ ...K, place: 'field', pure: true, noGlow: true, glowLights: false }), cap: 'A material is two dials: finish and body.', sub: 'Polished metal mirrors the place it stands in.' },
  { d: 3.6, s: () => ({ ...K, place: 'cave' }), cap: 'Glow: pixels that give off light.', sub: 'They light the sprite and the room around it.' },
  ...[['cryogonal', 'rain', 'Ice: clear, and it glitters.'], ['arcanine', 'field', 'Fur: fuzzy and matte.'], ['charizard', 'cave', 'Wing membrane: light from behind comes through.', { behind: true }],
      ['chandelure', 'cave', 'Glass, iron and flame.'], ['jellicent', 'deep', 'Jelly: light sinks in and glows back out.']]
    .map(([sprite, place, sub, more]) => ({ d: 2.4, s: () => ({ sprite, place, ...more }), cap: 'The same two dials make every material.', sub })),
  ...[['studio', 'A studio.'], ['field', 'A field.'], ['rain', 'Rain. Whatever faces up gets wet.'], ['snow', 'Snow. It settles on whatever faces up.'], ['deep', 'Deep water.']]
    .map(([place, sub]) => ({ d: 1.8, s: () => ({ sprite: 'charizard', place }), cap: 'The file never changes. The place supplies the light.', sub })),
  ...PALS.map((palBase, i) => ({ d: 0.45, s: () => ({ sprite: 'arceus', place: 'studio', tab: 'looks', palBase }), cap: 'Looks reuse the same animation.', sub: 'This one file carries 34 palettes.', cut: i > 0 })),
  { d: 3.2, s: () => ({ sprite: 'charizard', place: 'field' }), cap: 'SPU · Sprite Universal', sub: 'One file per sprite: parts, motion, looks and light.' },
];

(async () => {
  const dir = __dirname + '/out/video'; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const b = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined, args: ['--use-angle=metal', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 520, height: 700 } });
  const errs = []; p.on('pageerror', (e) => errs.push(e.message));
  await p.goto('file://' + __dirname + '/out/html/spu-format-lab.html'); await p.waitForTimeout(1800);
  let n = 0, last = '';
  const shot = async (state, T) => {
    const o = { ...BASE, ...state, t: T, lamp: LAMP[state.place](T) };
    if (o.sprite !== last) { await p.evaluate((o) => window.lab.set(o), o); await p.waitForTimeout(60); last = o.sprite; }
    const url = await p.evaluate((o) => window.lab.set(o), o), name = String(n++).padStart(5, '0') + '.png';
    fs.writeFileSync(dir + '/' + name, Buffer.from(url.split(',')[1], 'base64')); return name;
  };
  const frames = []; let T = 0, start = 0;
  for (let i = 0; i < SCENES.length; i++) {
    const sc = SCENES[i], prev = SCENES[i - 1], count = Math.round(sc.d * FPS);
    for (let f = 0; f < count; f++, T += 1 / FPS) {
      const u = f / FPS, wipe = prev && !sc.cut && u < WIPE ? smooth(u / WIPE) : 1;
      const frame = { cap: sc.cap, sub: sc.sub, wipe, a: await shot(sc.s(u), T) };
      if (wipe < 1) frame.b = await shot(prev.s(prev.d + u), T);          // what the wipe is leaving behind
      frames.push(frame);
    }
    start += sc.d;
  }
  fs.writeFileSync(dir + '/frames.json', JSON.stringify({ fps: FPS, frames }));
  console.log(frames.length, 'frames,', n, 'captures,', start.toFixed(1), 'seconds, errors', JSON.stringify(errs)); await b.close();
})();
