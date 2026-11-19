// Capture the frames of the tour video from the built lab page: out/video/*.png and out/video/frames.json.
// Then `python video.py` adds visual labels and selectable captions and encodes out/spu-tour.mp4.
// The video builds one sprite up a layer at a time: the flat color it is stored as, its pieces and the animation that moves
// them, facing and the light that uses it, materials and glow. Then places, variants and views.
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const fs = require('fs');
const sharedSheetFrame = require('./video_looks.cjs');
const FPS = 30, WIPE = 0.7, FADE = 0.3;
const smooth = (x) => { x = Math.max(0, Math.min(1, x)); return x * x * (3 - 2 * x); };
const ramp = (u, from, to) => smooth((u - from) / (to - from));              // 0 until `from`, 1 after `to`
const line = (u, from, to) => Math.max(0, Math.min(1, (u - from) / (to - from)));    // the same at a steady rate, for what the page eases itself
const leave = (u, d) => ramp(u, d - 0.9, d - 0.1);                           // 0 to 1 over the end of a scene d seconds long: a side panel on its way out
// The light stays put unless its movement is part of the demonstration.
const STILL = { studio: [44, 28], cave: [44, 70], field: [150, 28], rain: [150, 58], snow: [150, 26], deep: [44, 58] };
const BASE = { tab: 'light', view: 'front', mode: 0, shiny: false, palBase: 'normal', looks: {}, animId: 'idle', blink: false, behind: false, flat: false, pure: false,
               glowLights: true, explode: 0, explodeAlways: true, plain: false, noGlow: false, sheet: 0 };
// Charizard's animation clock is set by hand, so it can come to rest on the pose the numbers are pinned to and leave it again.
// The `looks` tab stops the page blinking on its own clock; `blinks` closes the eyes at chosen moments instead.
const C = { sprite: 'charizard', place: 'studio', tab: 'looks' }, FLAT = { mode: 1, pure: true }, FACING = { mode: 2 };
const HOLD = 0.48, EASE = 0.9;                                               // the held pose, in animation seconds; how long it takes to stop or start
const settle = (u, end) => { const r = end - u; return r <= 0 ? HOLD : r < EASE ? HOLD - r * r / (2 * EASE) : HOLD - r + EASE / 2; };   // run, then slow to rest on the pose at `end`
const depart = (u, from) => { const x = u - from; return x <= 0 ? HOLD : x < EASE ? HOLD + x * x / (2 * EASE) : HOLD + x - EASE / 2; };  // rest on the pose until `from`, then run
const blinks = (u, ...at) => at.some((t) => u >= t && u < t + 0.13);
// One material from each of eight sprites, in a place that shows it: sprite, place, the material's name in that sprite's
// file, and the label. The last is Charizard's own wing, which the next scene explains; it stands 7 pixels right of center.
const MATS = [['klinklang', 'field', 'steel gears', 'Metal'], ['cryogonal', 'rain', 'ice', 'Ice'], ['arcanine', 'field', 'cream fur', 'Fur'], ['chandelure', 'cave', 'glass', 'Glass'],
              ['rotom', 'studio', 'plasma body', 'Plasma'], ['jellicent', 'deep', 'blue jelly', 'Jelly'], ['whimsicott', 'field', 'fluff', 'Fluff'], ['charizard', 'cave', 'wing membrane', 'Wing membrane', 7]];
const matState = (i) => MATS[i][0] === 'charizard' ? { ...C, place: 'cave', behind: true, lamp: [WING_LAMP_X, 44] } : { sprite: MATS[i][0], place: MATS[i][1] };
const PALS = [['normal', 'Normal'], ['fire', 'Fire'], ['water', 'Water'], ['grass', 'Grass'], ['electric', 'Electric'], ['ghost', 'Ghost'], ['ice', 'Ice'], ['dragon', 'Dragon']];
// The one light that moves: it rests, crosses the room and comes back to where it started.
const swing = (u, from, to) => { const w = Math.max(0, Math.min(1, (u - from) / (to - from))) * 2 * Math.PI;
  return [STILL.studio[0] + 53 * (1 - Math.cos(w)), STILL.studio[1] + 12 * Math.sin(w)]; };
const WING_LAMP_X = 128, OFFSTAGE_X = 280;                  // a torch behind the wing; the same torch out of the picture
const cavePose = (lampX, anim = HOLD) => ({ ...C, place: 'cave', anim, behind: true, lamp: [lampX, 44] });
const glowPose = (u) => cavePose(OFFSTAGE_X, depart(u, 0.9));
// Each scene: its chapter, how long it lasts, what the page shows at local time u, a visible note and an optional longer caption.
// How a scene begins says how it relates to the one before:
//   a wipe (the default) sweeps the new picture in over the old one, which keeps running underneath. Where both hold the
//     same pose, only the layer under the line changes: lit to flat color, flat color to facing, facing to lit, lit to the
//     material map and back. The sprite may keep moving under the wipe if both scenes count down to the same pose.
//   join: 'flow' is one continuous shot. The scene starts on the exact picture the last one ended on, and only the labels
//     fade. The scene before sets `show: [null, 0]` to clear its labels first.
//   s2:   a comparison. `s` on the left and `s2` on the right, under `labels`.
//   sum:  flat color and facing on the left, and what a light makes of them (`s`) on the right.
//   grid: the same sprite in several places at once, each with a label.
//   grow: a grid that gains one cell at each time in `at`, and re-fits itself to hold them. `s` is handed the cell's number.
//   ov:   what to draw on top (the palette, a legend), filled in from the sprite itself. `from` is when its numbers set off.
//   open: 0 to 1 at local time u. How far a side panel is in: the title's and the end card's text, the stored sheets beside
//     the light, the "Glow off" half. At 0 the picture fills the frame, ready for a wipe on the same pose.
//   show: [from, end]. The labels fade in at `from` and are gone by `end`; null keeps them on. An `end` of 0 or less
//     counts back from the end of the scene.
// `s` and `open` are also handed the scene's length, so what happens at its end moves with `d`.
// The tour builds one sprite up a layer at a time, and nothing is shown before the layer that explains it:
// flat color, the pieces, the animation, facing and the light, materials and glow. Then what one file can hold more of:
// places it can stand in, variants, views. Each thing is shown once.
const FACING_D = 3.4, LIGHT_D = 7.0;               // Charizard keeps moving from the pieces coming together until the light has swung
const SCENES = [
  { ch: 'The sprite', d: 4.8, s: (u, d) => ({ ...C, anim: settle(u, d), blink: blinks(u, 1.4, 3.0), lamp: [34 + 10 * leave(u, d), 28] }),
    open: (u, d) => 1 - leave(u, d), // the list is in the order the video shows them
    ov: { kind: 'intro', heading: 'A file format for 2D sprites', lead: 'One file holds a sprite’s', items: ['Palettes', 'Pieces', 'Animations', 'Surface facing', 'Materials', 'Variants', 'Views'] },
    cap: 'An SPU file stores a sprite’s palettes, pieces and animations, which way its surfaces face and how they take light, and its variants and views.' },
  // 1. Flat color
  { ch: 'Color', d: 5.8, s: () => ({ ...C, ...FLAT, anim: HOLD }), note: 'A palette is a list of colors.\nEach pixel stores a number.',
    cap: 'The file stores only flat color. Shading is not stored: it comes later, from the light. A palette is a numbered list of colors, and each pixel stores a number.',
    ov: { kind: 'palette', lead: { heading: 'Only flat color is stored.', note: 'Shading comes later,\nfrom the light.' }, key: 1.9, from: 2.8 }, show: [null, 0] },
  // 2. Pieces and the animation, as one shot and still in flat color: the sprite moves, comes apart onto its sheet, and is put back together
  { ch: 'Pieces', join: 'flow', d: 3.9, s: (u) => ({ ...C, ...FLAT, anim: depart(u, 0), sheet: line(u, 1.2, 3.0) }), note: 'The sprite is built from pieces that share one sheet.', ov: { kind: 'sheet' }, show: [0.2, 0] },
  { ch: 'Motion', join: 'flow', d: 5.2, s: (u, d) => ({ ...C, ...FLAT, sheet: 1 - line(u, 0.7, 2.5), anim: settle(u, d + FACING_D + LIGHT_D), blink: blinks(u, 3.6) }),
    note: 'An animation puts the pieces together and moves them.', cap: 'An animation puts the pieces in place and moves them. Each can slide, turn, stretch or switch pictures.', ov: { kind: 'sheet' }, show: [0.2, null] },
  // 3. Facing, then the light that uses it. The sprite keeps moving under both wipes, and has no materials or glow yet.
  { ch: 'Facing', d: FACING_D, s: (u, d) => ({ ...C, ...FACING, anim: settle(u, d + LIGHT_D) }), note: 'Each pixel also records which way it faces.',
    cap: 'Each pixel also records which way its surface faces: up, right, or toward the viewer.', ov: { kind: 'facing' } },
  { ch: 'Light', d: LIGHT_D, sum: true, s: (u, d) => ({ ...C, pure: true, plain: true, noGlow: true, glowLights: false, anim: settle(u, d), blink: blinks(u, 3.2), lamp: swing(u, 1.9, d - 1.2) }),
    open: (u, d) => ramp(u, 0.8, 1.6) * (1 - leave(u, d)),
    note: 'Shading follows the light.', cap: 'A light uses flat color and facing to shade the sprite. Move the light and the shading follows.', show: [1.3, -0.8] },
  // 4. Materials: the numbers, what they do to Charizard, then the materials of other sprites, ending back on Charizard's wing
  { ch: 'Materials', d: 4.4, s: () => ({ ...C, mode: 3, anim: HOLD }), note: 'Each pixel also has\na material number.', cap: 'Each pixel also has a material number. Its material controls how it responds to light.',
    ov: { kind: 'materials', from: 1.4 } },
  { ch: 'Materials', d: 4.0, s: () => ({ ...C, anim: HOLD }), ov: { kind: 'surface' },
    note: 'Materials vary in shine\nand translucency.', cap: 'These materials differ in how they reflect light and how much light passes through them.' },
  // The grid grows by one material at a time. Each cell has a sample ball and the material's two dials.
  // Every new cell gets the same second and a half to itself; the full grid needs no long hold after that.
  { ch: 'Materials', d: 12.6, grow: { at: [0, 2.1, 3.6, 5.1, 6.6, 8.1, 9.6, 11.1], cells: MATS.map(([, , row, label, shift = 0]) => ({ row, label, shift })) }, s: (u, d, i) => matState(i),
    cap: 'Every sprite lists its own materials. Each has a finish, from fuzzy to polished, and a body, from metal to clear.' },
  // 5. Glow, in the cave the wing was just shown in, with its torch out of the picture
  { ch: 'Glow', d: 2.8, s: () => cavePose(OFFSTAGE_X), ov: { kind: 'glow', material: 5, title: 'Tail flame' }, note: 'Pixels can also glow.\nThe flame makes its own light.',
    cap: 'Pixels can also glow. The tail flame makes its own light.', show: [0.8, 0] },
  { ch: 'Glow', join: 'flow', d: 4.4, s: (u) => ({ ...glowPose(u), noGlow: true, glowLights: false }), s2: glowPose, open: (u) => ramp(u, 0.1, 0.9),
    labels: ['Glow off', 'Glow on'], ov: { kind: 'glow', material: 5 }, note: 'The flame lights the nearby rock.', cap: 'The glowing pixels cast light on Charizard and the rock around its tail.', show: [0.9, null] },
  // The finished sprite, then what else one file holds
  { ch: 'Places', d: 4.8, grid: [['studio', 'Studio'], ['cave', 'Cave'], ['rain', 'Rain'], ['field', 'Field'], ['snow', 'Snow'], ['deep', 'Underwater']], s: (u, d, place) => ({ sprite: 'charizard', place }),
    note: 'The same sprite takes on the light of each place.' },
  { ch: 'Variants', d: 4.4, grid: PALS, s: (u, d, palBase) => ({ sprite: 'arceus', place: 'studio', tab: 'looks', palBase }),
    ov: { kind: 'typePalettes' }, note: 'Swap the palette and the same pixels change color. Arceus has 34: 17 types × 2 (normal and shiny).',
    cap: 'A sprite can have several palettes. Swapping one recolors the same pixels. Arceus has 34: one for each of its 17 types, times two for normal and shiny.' },
  { ch: 'Variants', d: 6.2, s: () => ({ sprite: 'jellicent', place: 'studio', tab: 'looks' }), ov: { kind: 'sharedSheet', assembled: 'One animation moves both variants.' }, note: 'Variants swap pieces. Jellicent’s two share one sheet.',
    cap: 'A sprite can have variants that swap pieces. Male and female Jellicent take different pieces from one sheet, and one animation moves both.' },
  { ch: 'Views', d: 4.0, s: () => ({ sprite: 'charizard', place: 'field' }), s2: () => ({ sprite: 'charizard', place: 'field', view: 'back' }),
    labels: ['Front', 'Back'], note: 'One file can hold\nseveral views.', cap: 'A sprite can have several views, such as front and back.' },
  { ch: '', d: 4.4, s: () => ({ sprite: 'charizard', place: 'field' }), open: (u) => ramp(u, 0.9, 1.7), cap: 'Universal Sprite Format (SPU)', ov: { kind: 'end' } },
];
// Run inside the page, after reading the ID map from the posed sprite.
function materialSamples(ov) {
  const view = current(), palette = view.palettes[palId(view)];
  const samples = ov.samples || view.rows.map((_, id) => ({ id }));
  const size = BW + 8, x = (SW - size) / 2, y = GROUND - BW - 4;
  const tile = document.createElement('canvas');
  tile.width = tile.height = size;
  const ctx = tile.getContext('2d'), scale = canvas.width / SW;
  ctx.imageSmoothingEnabled = false;

  function renderSample({id, row: name}) {
    if (name) id = view.rows.findIndex((r) => r.name === name);       // a material can be asked for by name
    const row = view.rows[id];
    // Glow belongs to the pixels. Use their mean emission at each palette shade.
    const coverage = new Float64Array(256), emission = new Float64Array(256);
    if (row.glow || !row.ramp.length) {
      for (let i = 0; i < view.rawColor.length; i += 4) {
        const alpha = view.rawColor[i + 3];
        if (!alpha || view.rawLight[i + 2] !== id) continue;
        const shade = view.rawColor[i];
        coverage[shade] += alpha;
        emission[shade] += alpha * view.rawLight[i + 3] / 255;
      }
    }
    // A row without a shade list uses the color painted on its pixels.
    const shades = row.ramp.length ? row.ramp : [coverage.indexOf(Math.max(...coverage))];
    S.ball = {
      name: row.name, f: row.f, b: row.b, base: row.base, line: id === 0 || !row.ramp.length,
      c: shades.map(slot => '#' + palette[slot].slice(0, 3).map(c => c.toString(16).padStart(2, '0')).join('')),
      glow: row.glow ? shades.map(slot => coverage[slot] ? emission[slot] / coverage[slot] : 0) : null,
    };
    setBall(S.ball);
    draw();
    // The whole sphere and a little space for its shadow and glow.
    ctx.drawImage(canvas, x * scale, y * scale, size * scale, size * scale, 0, 0, size, size);
    return tile.toDataURL('image/png').split(',')[1];
  }

  const mode = S.mode;
  S.mode = 0;                                        // lit spheres, whichever view the scene shows the sprite in
  try {
    return samples.map(renderSample);
  } finally {
    S.ball = null; S.mode = mode;
    draw();
  }
}

(async () => {
  const dir = __dirname + '/out/video'; fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
  const b = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined, args: ['--use-angle=metal', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 520, height: 700 } });
  const errs = []; p.on('pageerror', (e) => errs.push(e.message));
  await p.goto('file://' + __dirname + '/out/html/spu-format-lab.html'); await p.waitForTimeout(1800);
  let n = 0, last = '', loop = 1; const seen = new Set();
  const shot = async (state, T, sheetTime) => {
    const o = { ...BASE, ...state, t: T, lamp: state.lamp || STILL[state.place] };
    if (o.sprite !== last) {                                                  // a sprite needs a moment the first time it is put on stage
      await p.evaluate((o) => window.lab.set(o), o); if (!seen.has(o.sprite)) await p.waitForTimeout(60); seen.add(o.sprite); last = o.sprite; loop = await p.evaluate(() => current().length / current().tps); }
    if (o.anim !== undefined) o.anim = ((o.anim % loop) + loop) % loop;       // a clock counting down to the held pose starts below zero
    const url = sheetTime === undefined ? await p.evaluate((o) => window.lab.set(o), o) : await p.evaluate(sharedSheetFrame, sheetTime);
    const name = String(n++).padStart(5, '0') + '.png';
    fs.writeFileSync(dir + '/' + name, Buffer.from(url.split(',')[1], 'base64')); return name;
  };
  // every capture one scene needs at local time u
  const captures = async (sc, u, T) => {
    if (sc.grow) { const cells = []; for (const [i, at] of sc.grow.at.entries()) cells.push(u >= at ? await shot(sc.s(u, sc.d, i), T) : null); return { cells }; }
    if (sc.grid) { const cells = []; for (const [key] of sc.grid) cells.push(await shot(sc.s(u, sc.d, key), T)); return { cells }; }
    const state = sc.s(u, sc.d), c = { a: await shot(state, T, sc.ov?.kind === 'sharedSheet' ? u : undefined), open: sc.open?.(u, sc.d) };
    if (sc.s2) c.c = await shot(sc.s2(u, sc.d), T);
    if (sc.sum) { c.flat = await shot({ ...state, ...FLAT }, T); c.facing = await shot({ ...state, ...FACING }, T); }
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
    return { indices, palette: pal.slice(1, 16).map((c) => c.slice(0, 3)),
             rows: v.rows.map((r) => ({ name: r.name, f: r.f, b: r.b, words: r.name === 'outline' || !r.ramp.length ? 'line art' : finishWord(r.f) + ', ' + bodyWord(r.b), ramp: r.ramp.map((c) => pal[c].slice(0, 3)) })) }; }, ov);
  const frames = [], scenes = []; let T = 0, total = 0;
  for (let i = 0; i < SCENES.length; i++) {
    const sc = SCENES[i], prev = SCENES[i - 1], count = Math.round(sc.d * FPS);
    for (let f = 0; f < count; f++, T += 1 / FPS) {
      const u = f / FPS, wipe = prev && sc.join !== 'flow' && u < WIPE ? smooth(u / WIPE) : 1;
      const frame = { scene: i, seconds: u, wipe, caps: await captures(sc, u, T) };
      if (f === 0) {
        const swatches = sc.ov?.kind === 'typePalettes'
          ? await p.evaluate((keys) => keys.map(key => [11, 9, 8].map(i => current().palettes[key][i].slice(0, 3))), sc.grid.map(g => g[0]))
          : undefined;
        const spriteInfo = await info(sc.ov);
        if (sc.ov?.kind === 'surface' || sc.ov?.kind === 'materials') spriteInfo.samples = await p.evaluate(materialSamples, sc.ov);
        if (sc.grow) {                                         // each cell's material: its two dials and a sample ball lit in the cell's own place
          spriteInfo.cards = [];
          for (const [i, { row }] of sc.grow.cells.entries()) {
            await shot(sc.s(0, sc.d, i), T);
            const dials = await p.evaluate((row) => { const r = current().rows.find((r) => r.name === row); return { f: r.f, b: r.b }; }, row);
            spriteInfo.cards.push({ ...dials, ball: (await p.evaluate(materialSamples, { samples: [{ row }] }))[0] });
          }
        }
        scenes.push({ cap: sc.cap || sc.note, note: sc.note, chapter: sc.ch, labels: sc.labels, show: sc.show && [sc.show[0], sc.show[1] !== null && sc.show[1] <= 0 ? sc.d + sc.show[1] : sc.show[1]], fade: FADE, sum: !!sc.sum, grid: sc.grid?.map(g => g[1]), grow: sc.grow && { at: sc.grow.at, labels: sc.grow.cells.map(c => c.label), shifts: sc.grow.cells.map(c => c.shift) }, swatches, ov: sc.ov, info: spriteInfo });
      }
      if (wipe < 1) { frame.prevSeconds = prev.d + u; frame.prev = await captures(prev, prev.d + u, T); }       // the scene the wipe is leaving behind
      frames.push(frame);
    }
    total += sc.d;
  }
  fs.writeFileSync(dir + '/frames.json', JSON.stringify({ fps: FPS, scenes, frames }));
  console.log(frames.length, 'frames,', n, 'captures,', total.toFixed(1), 'seconds, errors', JSON.stringify(errs)); await b.close();
})();
