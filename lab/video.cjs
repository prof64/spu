// Capture the frames of the tour video from the built lab page: out/video/*.png and out/video/frames.json.
// Then `python video.py` adds captions and encodes ../docs/spu-tour.mp4.
// The video builds one sprite up a layer at a time (parts, flat color, facing, shading, materials, glow), with the
// palette, legends and side-by-side comparisons on screen, then shows other materials, places and looks.
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
const PALS = [['normal', 'Normal'], ['fire', 'Fire'], ['water', 'Water'], ['grass', 'Grass'], ['electric', 'Electric'], ['ghost', 'Ghost'], ['ice', 'Ice'], ['dragon', 'Dragon']];
// Each scene: how long, what the page shows at local time u, and one caption. A scene wipes in over the one before unless
// it cuts. `s2` makes it a comparison: `s` on the left, `s2` on the right, under `labels`. `ov` is what to draw on top
// (the palette, a legend), filled in from the sprite itself.
const SCENES = [
  { d: 2.6, s: () => K, cap: 'This is Klinklang, stored as one .spu file.', cut: true },
  { d: 2.6, s: (u) => ({ ...K, explode: 0.9 * smooth(u / 1.1) }), cap: 'The sprite is made of parts. This one has 18.', cut: true },
  { d: 2.6, s: () => ({ ...K, explode: 0.9, mode: 5 }), cap: 'Here each part has its own color. Parts move, turn and switch drawings independently.' },
  { d: 1.6, s: (u) => ({ ...K, explode: 0.9 * (1 - smooth(u / 1.1)), mode: 5 }), cap: 'Here each part has its own color. Parts move, turn and switch drawings independently.', cut: true },
  { d: 4.0, s: () => ({ ...K, mode: 1, pure: true }), cap: 'The drawings are flat color. Each pixel is one of the colors in the palette.', ov: { kind: 'palette' } },
  { d: 4.0, s: () => ({ ...K, mode: 2 }), cap: 'Each pixel also stores which way its surface faces.', ov: { kind: 'facing' } },
  { d: 5.0, s: () => ({ ...K, mode: 1, pure: true }), s2: () => ({ ...K, pure: true, plain: true, noGlow: true, glowLights: false }), labels: ['Flat color', 'Flat color + facing + a light'],
    cap: 'A light shades each pixel by how much it faces the light, using only that material’s shades.', ov: { kind: 'shades', row: 'steel gears' }, cut: true },
  { d: 4.2, s: () => ({ ...K, mode: 3 }), cap: 'Each pixel names a material. A material is two settings: its finish and its body.', ov: { kind: 'materials' }, cut: true },
  { d: 5.0, s: () => ({ ...K, place: 'field', plain: true }), s2: () => ({ ...K, place: 'field' }), labels: ['Materials off', 'Materials on'], cap: 'With materials on, the metal gears reflect the sky and the grass.', cut: true },
  { d: 4.5, s: () => ({ ...K, place: 'cave', noGlow: true, glowLights: false }), s2: () => ({ ...K, place: 'cave' }), labels: ['Glow off', 'Glow on'], cap: 'Pixels can glow. Glowing pixels light the sprite and the room.', cut: true },
  ...[['cryogonal', 'rain', 'ice', 'Ice: the scene shows through it, and it glitters.'], ['arcanine', 'field', 'cream fur', 'Fur: fuzzy, so it shades in tufts.'],
      ['charizard', 'cave', 'wing membrane', 'A wing membrane: translucent, so light from behind comes through.', { behind: true }],
      ['chandelure', 'cave', 'glass', 'Glass around a flame.'], ['jellicent', 'deep', 'blue jelly', 'Jelly: light sinks in and glows back out.']]
    .map(([sprite, place, row, cap, more], i) => ({ d: 2.8, s: () => ({ sprite, place, ...more }), cap, ov: { kind: 'dials', row }, cut: i === 0 })),
  ...[['studio', 'in a studio'], ['field', 'in a field'], ['rain', 'in rain'], ['snow', 'in snow'], ['deep', 'under water']]
    .map(([place, where]) => ({ d: 1.9, s: () => ({ sprite: 'charizard', place }), cap: 'The file does not change between places. This is the same Charizard ' + where + '.' })),
  ...PALS.map(([palBase, name], i) => ({ d: 0.55, s: () => ({ sprite: 'arceus', place: 'studio', tab: 'looks', palBase }), cap: 'A file can carry several palettes. Arceus has 34, one per type and a shiny of each.', ov: { kind: 'palette', name }, cut: i > 0 })),
  { d: 1.5, s: () => ({ sprite: 'jellicent', place: 'deep', tab: 'looks' }), cap: 'A look can also swap drawings. This is Jellicent’s male form...' },
  { d: 2.2, s: () => ({ sprite: 'jellicent', place: 'deep', tab: 'looks', looks: { female: true } }), cap: '...and its female form, in the same file with the same animation.' },
  { d: 2.6, s: () => ({ sprite: 'charizard', place: 'field' }), cap: 'SPU: Sprite Universal.' },
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
  // what the sprite on stage holds, for the overlays: its palette and its materials
  const info = () => p.evaluate(() => { const v = current(), pal = v.palettes[palId(v)];
    return { parts: v.parts.length, palette: pal.slice(1, 16).map((c) => c.slice(0, 3)),
             rows: v.rows.map((r) => ({ name: r.name, f: r.f, b: r.b, words: r.name === 'outline' || !r.ramp.length ? 'line art' : finishWord(r.f) + ', ' + bodyWord(r.b) + (r.glow ? ', glows' : ''), ramp: r.ramp.map((c) => pal[c].slice(0, 3)) })) }; });
  const frames = [], scenes = []; let T = 0, start = 0;
  for (let i = 0; i < SCENES.length; i++) {
    const sc = SCENES[i], prev = SCENES[i - 1], count = Math.round(sc.d * FPS);
    for (let f = 0; f < count; f++, T += 1 / FPS) {
      const u = f / FPS, wipe = prev && !sc.cut && !prev.s2 && u < WIPE ? smooth(u / WIPE) : 1;
      const frame = { scene: i, wipe, a: await shot(sc.s(u), T) };
      if (f === 0) scenes.push({ cap: sc.cap, labels: sc.labels, ov: sc.ov, info: await info() });
      if (sc.s2) frame.c = await shot(sc.s2(u), T);                        // the right-hand side of a comparison
      if (wipe < 1) frame.b = await shot(prev.s(prev.d + u), T);          // what the wipe is leaving behind
      frames.push(frame);
    }
    start += sc.d;
  }
  fs.writeFileSync(dir + '/frames.json', JSON.stringify({ fps: FPS, scenes, frames }));
  console.log(frames.length, 'frames,', n, 'captures,', start.toFixed(1), 'seconds, errors', JSON.stringify(errs)); await b.close();
})();
