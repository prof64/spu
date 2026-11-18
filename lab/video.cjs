// Capture the frames of the tour video from the built lab page: out/video/*.png and out/video/frames.json.
// Then `python video.py` adds visual labels and selectable captions and encodes ../docs/spu-tour.mp4.
// The video takes one sprite apart and puts it back: the flat color it is stored as, the sheet its pieces live on,
// the facing sheet, the animation that assembles it, and the shading a light makes of them. Then materials, glow,
// places and looks.
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const fs = require('fs');
const sharedSheetFrame = require('./video_looks.cjs');
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
// Each scene: its chapter, how long it lasts, what the page shows at local time u, a visible note and an optional longer caption.
// A scene wipes in over the one before unless it cuts.
//   s2:  a comparison. `s` on the left and `s2` on the right, under `labels`.
//   sum: flat color and facing on the left, and what a light makes of them (`s`) on the right.
//   grid: the same sprite in several places at once, each with a label.
//   ov:  what to draw on top (the palette, a legend), filled in from the sprite itself.
const SCENES = [
  { ch: 'The sprite', d: 2.8, s: () => C, note: 'One file holds this\nwhole animation.', cap: 'This Charizard is one .spu file.', ov: { kind: 'file' } },
  { ch: 'Color', d: 4.6, s: () => ({ ...C, ...FLAT, tab: 'looks', anim: 0.48 }), note: 'Each pixel’s number\npicks a palette color.', cap: 'Each color on the flat sprite has a number in the palette.', ov: { kind: 'palette' } },
  { ch: 'Pieces', d: 5.0, s: (u) => ({ ...C, ...FLAT, sheet: smooth(u / 2.0) }), note: 'The pieces share one sheet.', ov: { kind: 'sheet' } },
  { ch: 'Facing', d: 4.4, s: () => ({ ...C, ...FACING, sheet: 1 }), note: 'A second sheet records which way each pixel faces.', ov: { kind: 'facing' } },
  { ch: 'Motion', d: 6.0, s: (u) => ({ ...C, mode: 5, sheet: 1 - smooth((u - 1.2) / 2.0) }), note: 'The animation moves\nand turns the pieces.', cap: 'An animation puts the pieces in place and moves them. Each can slide, turn, stretch or switch pictures.', ov: { kind: 'motion' } },
  { ch: 'Light', d: 6.0, sum: true, s: () => ({ ...C, pure: true }), note: 'Shading follows the light.', cap: 'The light uses flat color and facing to shade the sprite. Move the light and the shading follows.', lamp: SWING },
  { ch: 'Materials', d: 4.8, s: () => ({ ...C, mode: 3, tab: 'looks', anim: 0.48 }), note: 'Each pixel also has\na material number.', cap: 'Each pixel also has a material number. Its material controls how it responds to light.', ov: { kind: 'materials' } },
  { ch: 'Materials', d: 4.6, s: () => ({ ...C, tab: 'looks', anim: 0.48 }), ov: { kind: 'surface' },
    cap: 'The numbers stay on the same pixels when Charizard is lit. Each close-up shows one of its seven materials.' },
  { ch: 'Materials', d: 4.8, s: () => ({ ...C, tab: 'looks', anim: 0.48 }), ov: { kind: 'surface', samples: [1, 2],
    notes: [['Light catches the', 'orange scales.'], ['Light scatters inside', 'the waxy skin.']] },
    cap: 'The scales have a satin finish. Light scatters beneath the waxy skin of the underbelly.' },
  { ch: 'Materials', d: 4.8, s: () => ({ ...C, tab: 'looks', anim: 0.48 }), ov: { kind: 'surface', samples: [4, 6],
    notes: [['Hard points catch', 'a pale highlight.'], ['The polished surface', 'has a sharper shine.']] },
    cap: 'The claws and teeth catch pale highlights. The mouth has a more polished finish.' },
  { ch: 'Materials', d: 5.0, s: () => ({ ...C, place: 'cave', tab: 'looks', anim: 0.48, behind: true, lamp: [128, 44] }),
    ov: { kind: 'surface', wing: true }, cap: 'A lamp behind the wing lights the translucent membrane. An opaque wing blocks that light.' },
  // the torch is far off to the side here, so that the glow is what lights the sprite
  { ch: 'Glow', d: 4.8, s: () => ({ sprite: 'charizard', place: 'cave', noGlow: true, glowLights: false, lamp: [-36, 62] }), s2: () => ({ sprite: 'charizard', place: 'cave', lamp: [-36, 62] }),
    labels: ['Glow off', 'Glow on'], ov: { kind: 'glow', material: 5 }, note: 'The tail flame lights the cave.', cap: 'Pixels can also glow. Glow lights the sprite and whatever is near it.' },
  { ch: 'Places', d: 6.0, grid: [['studio', 'Studio'], ['cave', 'Cave'], ['rain', 'Rain'], ['field', 'Field'], ['snow', 'Snow'], ['deep', 'Underwater']], s: (u, place) => ({ sprite: 'charizard', place: place || 'studio' }),
    note: 'The same sprite takes on the light of each place.' },
  { ch: 'Looks', d: 4.8, grid: PALS, s: (u, palBase) => ({ sprite: 'arceus', place: 'studio', tab: 'looks', palBase }),
    ov: { kind: 'typePalettes' }, note: 'All 34 palettes use the same Arceus sprite.', cap: 'Arceus carries 34 palettes. Each of its 17 types has a shiny palette too.' },
  { ch: 'Looks', d: 6.6, s: () => ({ sprite: 'jellicent', place: 'studio', tab: 'looks' }), ov: { kind: 'sharedSheet', assembled: 'The same animation moves both forms.' }, note: 'Both Jellicent forms take their pieces from one sheet.',
    cap: 'Male and female Jellicent use different pieces from one sheet. Both play the same animation.' },
  { ch: '', d: 2.8, s: () => ({ sprite: 'charizard', place: 'field' }), cap: 'Universal Sprite Format (SPU)', ov: { kind: 'end' } },
];

(async () => {
  const dir = __dirname + '/out/video'; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const b = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined, args: ['--use-angle=metal', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 520, height: 700 } });
  const errs = []; p.on('pageerror', (e) => errs.push(e.message));
  await p.goto('file://' + __dirname + '/out/html/spu-format-lab.html'); await p.waitForTimeout(1800);
  let n = 0, last = '';
  const shot = async (state, T, lamp, sheetTime) => {
    const o = { ...BASE, ...state, t: T, lamp: state.lamp || (lamp ? lamp(T) : STILL[state.place]) };
    if (o.sprite !== last) { await p.evaluate((o) => window.lab.set(o), o); await p.waitForTimeout(60); last = o.sprite; }
    const url = sheetTime === undefined ? await p.evaluate((o) => window.lab.set(o), o) : await p.evaluate(sharedSheetFrame, sheetTime);
    const name = String(n++).padStart(5, '0') + '.png';
    fs.writeFileSync(dir + '/' + name, Buffer.from(url.split(',')[1], 'base64')); return name;
  };
  // every capture one scene needs at local time u
  const captures = async (sc, u, T) => {
    if (sc.grid) { const cells = []; for (const [key] of sc.grid) cells.push(await shot(sc.s(u, key), T)); return { cells }; }
    const c = { a: await shot(sc.s(u), T, sc.lamp, sc.ov?.kind === 'sharedSheet' ? u : undefined) };
    if (sc.s2) c.c = await shot(sc.s2(u), T, sc.lamp);
    if (sc.sum) { c.flat = await shot({ ...sc.s(u), ...FLAT }, T, sc.lamp); c.facing = await shot({ ...sc.s(u), ...FACING }, T, sc.lamp); }
    return c;
  };
  // what the sprite on stage holds, for the overlays: its palette and its materials
  const info = (ov = {}) => p.evaluate((ov) => { const kind = ov.kind; const v = current(), pal = v.palettes[palId(v)];
    // Pin each number to an actual sprite pixel, before the room, reflections and tone mapping.
    // The flat-color pass contains the displayed palette color. The second geometry attachment holds the material ID.
    let indices;
    if (kind === 'palette' || kind === 'materials' || kind === 'surface') {
      const read = (target, attachment) => {
        gl.bindFramebuffer(gl.READ_FRAMEBUFFER, target.fbo);
        gl.readBuffer(gl.COLOR_ATTACHMENT0 + attachment);
        const pixels = new Uint8Array(SW * SH * 4);
        gl.readPixels(0, 0, SW, SH, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
        return pixels;
      };
      const lit = read(T.lit, 0), materials = kind !== 'palette' ? read(T.g, 1) : null;
      gl.bindFramebuffer(gl.READ_FRAMEBUFFER, null);
      const ids = new Uint8Array(SW * SH).fill(255);
      for (let y = 0; y < SH; y++) for (let x = 0; x < SW; x++) {
        const i = ((SH - 1 - y) * SW + x) * 4;
        if (!lit[i + 3]) continue;
        // Palette slot zero is transparent. It can share its RGB with visible black.
        const id = materials ? materials[i + 2] : pal.findIndex((c, j) => j > 0 && c[0] === lit[i] && c[1] === lit[i + 1] && c[2] === lit[i + 2]);
        if (id < 0) throw Error('Color outside the indexed palette on the flat sprite');
        ids[y * SW + x] = id;
      }
      indices = btoa(String.fromCharCode(...ids));
    }
    // Show each finish and body on the lab's test sphere, using the colors of the same SPU row.
    // The backlit comparison instead uses two close-ups of the actual wing.
    const swatch = (x, y, w, h) => {
      const shot = document.createElement('canvas'), k = canvas.width / SW;
      shot.width = w; shot.height = h;
      const ctx = shot.getContext('2d'); ctx.imageSmoothingEnabled = false;
      ctx.drawImage(canvas, x * k, y * k, w * k, h * k, 0, 0, w, h);
      return shot.toDataURL('image/png').split(',')[1];
    };
    let samples;
    if (ov.samples) {
      samples = ov.samples.map(id => {
        const row = v.rows[id];
        const colors = Array.from({length: 4}, (_, k) => {
          const rgb = pal[row.ramp[Math.round(k * (row.ramp.length - 1) / 3)]];
          return '#' + rgb.slice(0, 3).map(c => c.toString(16).padStart(2, '0')).join('');
        });
        S.ball = { name: row.name, c: colors, f: row.f, b: row.b };
        setBall(S.ball); draw();
        return swatch(SW / 2 - 25, GROUND - 49, 50, 50);
      });
      S.ball = null; draw();
    } else if (ov.wing) {
      const wing = () => swatch(98, 37, 33, 32);
      draw();
      const backlit = wing();
      // Change only the membrane's body so both close-ups keep the same finish, pose and lamp.
      const membrane = v.rows[3], body = membrane.b;
      membrane.b = 0; draw();
      samples = [wing(), backlit];
      membrane.b = body; draw();
    }
    return { indices, samples, palette: pal.slice(1, 16).map((c) => c.slice(0, 3)),
             rows: v.rows.map((r) => ({ name: r.name, f: r.f, b: r.b, words: r.name === 'outline' || !r.ramp.length ? 'line art' : finishWord(r.f) + ', ' + bodyWord(r.b), ramp: r.ramp.map((c) => pal[c].slice(0, 3)) })) }; }, ov);
  const frames = [], scenes = []; let T = 0, total = 0;
  for (let i = 0; i < SCENES.length; i++) {
    const sc = SCENES[i], prev = SCENES[i - 1], count = Math.round(sc.d * FPS);
    for (let f = 0; f < count; f++, T += 1 / FPS) {
      const u = f / FPS, wipe = prev && !sc.cut && u < WIPE ? smooth(u / WIPE) : 1;
      const frame = { scene: i, seconds: u, wipe, caps: await captures(sc, u, T) };
      if (f === 0) {
        const swatches = sc.ov?.kind === 'typePalettes'
          ? await p.evaluate((keys) => keys.map(key => [11, 9, 8].map(i => current().palettes[key][i].slice(0, 3))), sc.grid.map(g => g[0]))
          : undefined;
        scenes.push({ cap: sc.cap || sc.note, note: sc.note, chapter: sc.ch, labels: sc.labels, sum: !!sc.sum, grid: sc.grid?.map(g => g[1]), swatches, ov: sc.ov, info: await info(sc.ov) });
      }
      if (wipe < 1) { frame.prevSeconds = prev.d + u; frame.prev = await captures(prev, prev.d + u, T); }       // the scene the wipe is leaving behind
      frames.push(frame);
    }
    total += sc.d;
  }
  fs.writeFileSync(dir + '/frames.json', JSON.stringify({ fps: FPS, scenes, frames }));
  console.log(frames.length, 'frames,', n, 'captures,', total.toFixed(1), 'seconds, errors', JSON.stringify(errs)); await b.close();
})();
