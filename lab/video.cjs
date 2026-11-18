// Capture the frames of the tour video from the built lab page: out/video/*.png and out/video/frames.json.
// Then `python video.py` adds captions and encodes ../docs/spu-tour.mp4.
// The video takes one sprite apart and puts it back: the flat color it is stored as, the sheet its pieces live on,
// the facing sheet, the animation that assembles it, and the shading a light makes of them. Then materials, glow,
// places and looks.
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const fs = require('fs');
const FPS = 30, WIPE = 0.7;
const smooth = (x) => { x = Math.max(0, Math.min(1, x)); return x * x * (3 - 2 * x); };
// The light stays put, so that only the thing a scene is about changes. A scene marked `move` is about the light.
const LAMP = {                                   // where a moving light is at time T, per place
  studio: (T) => [96 + 60 * Math.cos(T * 0.9), 32 + 12 * Math.sin(T * 1.3)],
  cave: (T) => [96 - 64 * Math.cos(T * 0.8), 68 + 12 * Math.sin(T * 1.1)],
  field: (T) => [96 + 62 * Math.sin(T * 0.6), 30 + 6 * Math.cos(T * 0.6)],
  rain: (T) => [96 + 60 * Math.sin(T * 0.8), 70 + 6 * Math.cos(T * 1.1)],
  snow: (T) => [96 + 60 * Math.sin(T * 0.6), 27],
  deep: (T) => [96 + 60 * Math.sin(T * 0.8), 62 + 8 * Math.cos(T)],
};
const STILL = { studio: [44, 28], cave: [44, 70], field: [150, 28], rain: [150, 58], snow: [150, 26], deep: [44, 58] };   // where it rests otherwise
const BASE = { tab: 'light', view: 'front', mode: 0, shiny: false, palBase: 'normal', looks: {}, animId: 'idle', blink: false, behind: false, flat: false, pure: false,
               glowLights: true, explode: 0, explodeAlways: true, plain: false, noGlow: false, sheet: 0 };
const C = { sprite: 'charizard', place: 'studio' }, FLAT = { mode: 1, pure: true }, FACING = { mode: 2 };
const PALS = [['normal', 'Normal'], ['fire', 'Fire'], ['water', 'Water'], ['grass', 'Grass'], ['electric', 'Electric'], ['ghost', 'Ghost'], ['ice', 'Ice'], ['dragon', 'Dragon']];
const SWING = (T) => [96 + 50 * Math.cos(T * 1.0), 30 + 10 * Math.sin(T * 1.5)];       // the one light that moves
// Each scene: its chapter, how long it lasts, what the page shows at local time u, and one caption.
// A scene wipes in over the one before unless it cuts.
//   s2:  a comparison. `s` on the left and `s2` on the right, under `labels`.
//   sum: flat color and facing on the left, and what a light makes of them (`s`) on the right.
//   ov:  what to draw on top (the palette, a legend), filled in from the sprite itself.
const SCENES = [
  { ch: 'The sprite', d: 2.8, s: () => C, cap: 'This Charizard is one .spu file.', cut: true },
  { ch: 'Color', d: 4.6, s: () => ({ ...C, ...FLAT }), cap: 'The file does not store shading. It stores flat color: each pixel is a number into the palette.', ov: { kind: 'palette' } },
  { ch: 'Pieces', d: 2.6, s: (u) => ({ ...C, ...FLAT, sheet: smooth(u / 2.0) }), cap: 'The sprite is stored as separate pieces, laid out on a sheet.', cut: true },
  { ch: 'Pieces', d: 3.2, s: () => ({ ...C, ...FLAT, sheet: 1 }), cap: 'Pieces this pose is not using stay on the sheet: two more flames, and closed eyes.', cut: true },
  { ch: 'Facing', d: 4.4, s: () => ({ ...C, ...FACING, sheet: 1 }), cap: 'A second sheet stores which way each pixel’s surface faces.', ov: { kind: 'facing', at: 'corner' } },
  { ch: 'Motion', d: 6.0, s: (u) => ({ ...C, mode: 5, sheet: 1 - smooth((u - 1.2) / 2.0) }), cap: 'An animation puts the pieces in place and moves them. Each can slide, turn, stretch or switch pictures.' },
  { ch: 'Light', d: 4.4, sum: true, s: () => ({ ...C, pure: true }), cap: 'Shading is not stored. A light works it out from the flat color and the facing.', cut: true, lamp: SWING },
  { ch: 'Light', d: 4.4, sum: true, s: () => ({ ...C, pure: true }), cap: 'Move the light and the shading follows. Every color it uses is already in the palette.', cut: true, lamp: SWING },
  { ch: 'Materials', d: 4.8, s: () => ({ ...C, mode: 3 }), cap: 'Each pixel also names a material, which says how its surface takes light.', ov: { kind: 'materials' }, cut: true },
  { ch: 'Materials', d: 5.0, s: () => ({ sprite: 'klinklang', place: 'field', plain: true }), s2: () => ({ sprite: 'klinklang', place: 'field' }), labels: ['Materials off', 'Materials on'],
    cap: 'Metal, for example, reflects what is around it: here, sky and grass.', cut: true },
  ...[['cryogonal', 'rain', 'ice', 'Ice lets the scene show through, and glitters.'], ['arcanine', 'field', 'cream fur', 'Fur is fuzzy, so it shades in tufts.'],
      ['charizard', 'cave', 'wing membrane', 'A wing membrane is translucent: light from behind comes through.', { behind: true, lamp: [128, 44] }],
      ['chandelure', 'cave', 'glass', 'Glass, around a flame.'], ['jellicent', 'deep', 'blue jelly', 'Jelly: light sinks in and glows back out.']]
    .map(([sprite, place, row, cap, more], i) => ({ ch: 'Materials', d: 2.8, s: () => ({ sprite, place, ...more }), cap, ov: { kind: 'dials', row }, cut: i === 0 })),
  { ch: 'Glow', d: 4.6, s: () => ({ sprite: 'charizard', place: 'cave', noGlow: true, glowLights: false }), s2: () => ({ sprite: 'charizard', place: 'cave' }), labels: ['Glow off', 'Glow on'],
    cap: 'Pixels can also glow. Glow lights the sprite and whatever is near it.', cut: true },
  ...[['studio', 'in a studio'], ['field', 'in a field'], ['rain', 'in rain'], ['snow', 'in snow'], ['deep', 'under water']]
    .map(([place, where], i) => ({ ch: 'Places', d: 1.9, s: () => ({ sprite: 'charizard', place }), cap: 'Nothing in the file changes between places. The same Charizard ' + where + '.', cut: i === 0 })),
  ...PALS.map(([palBase, name], i) => ({ ch: 'Looks', d: 0.55, s: () => ({ sprite: 'arceus', place: 'studio', tab: 'looks', palBase }), cap: 'A file can carry several palettes. Arceus has 34: one per type, and a shiny of each.', ov: { kind: 'palette', name }, cut: i > 0 })),
  { ch: 'Looks', d: 1.6, s: () => ({ sprite: 'jellicent', place: 'deep', tab: 'looks' }), cap: 'A look can also swap pieces. Jellicent’s male form...' },
  { ch: 'Looks', d: 2.4, s: () => ({ sprite: 'jellicent', place: 'deep', tab: 'looks', looks: { female: true } }), cap: '...and its female form, from the same file, with the same animation.' },
  { ch: '', d: 2.6, s: () => ({ sprite: 'charizard', place: 'field' }), cap: 'SPU: Sprite Universal' },
];

(async () => {
  const dir = __dirname + '/out/video'; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const b = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined, args: ['--use-angle=metal', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 520, height: 700 } });
  const errs = []; p.on('pageerror', (e) => errs.push(e.message));
  await p.goto('file://' + __dirname + '/out/html/spu-format-lab.html'); await p.waitForTimeout(1800);
  let n = 0, last = '';
  const shot = async (state, T, lamp) => {
    const o = { ...BASE, ...state, t: T, lamp: state.lamp || (lamp ? lamp(T) : STILL[state.place]) };
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
      const u = f / FPS, wipe = prev && !sc.cut && !prev.s2 && !prev.sum && u < WIPE ? smooth(u / WIPE) : 1;
      const frame = { scene: i, wipe, a: await shot(sc.s(u), T, sc.lamp) };
      if (f === 0) scenes.push({ cap: sc.cap, chapter: sc.ch, labels: sc.labels, sum: !!sc.sum, ov: sc.ov, info: await info() });
      if (sc.s2) frame.c = await shot(sc.s2(u), T, sc.lamp);
      if (sc.sum) { frame.flat = await shot({ ...sc.s(u), ...FLAT }, T, sc.lamp); frame.facing = await shot({ ...sc.s(u), ...FACING }, T, sc.lamp); }                        // the right-hand side of a comparison
      if (wipe < 1) frame.b = await shot(prev.s(prev.d + u), T, prev.lamp);          // what the wipe is leaving behind
      frames.push(frame);
    }
    start += sc.d;
  }
  fs.writeFileSync(dir + '/frames.json', JSON.stringify({ fps: FPS, scenes, frames }));
  console.log(frames.length, 'frames,', n, 'captures,', start.toFixed(1), 'seconds, errors', JSON.stringify(errs)); await b.close();
})();
