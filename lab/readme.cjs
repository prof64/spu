// Capture the pictures the repository README uses, from the built lab page: out/readme-shots.json.
// Then `python readme.py` lays them out into ../docs/images/.
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const fs = require('fs');
(async () => {
  const b = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined, args: ['--use-angle=metal', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: 1240, height: 900 }, deviceScaleFactor: 2 });
  const errs = []; p.on('pageerror', (e) => errs.push(e.message)); p.on('console', (m) => { if (m.type() === 'error') errs.push(m.text()); });
  await p.goto('file://' + __dirname + '/out/html/spu-format-lab.html'); await p.waitForTimeout(1800);
  const base = { tab: 'light', view: 'front', shiny: false, palBase: 'normal', looks: {}, animId: 'idle', blink: false, behind: false, flat: false, pure: false, byMat: false, glowOnly: true, glowLights: true, explode: 0 };
  const shot = async (o) => { const s = { ...base, ...o }; await p.evaluate((s) => window.lab.set(s), s); await p.waitForTimeout(25); return p.evaluate((s) => window.lab.set(s), s); };
  const out = {};
  const group = async (name, list) => { out[name] = []; for (const o of list) out[name].push(await shot(o)); };
  const studio = { place: 'studio', t: 1.2, lamp: [40, 28] };
  // the same moving light, twelve frames a second, for two short loops
  const loop = async (name, n, o, lamp) => { out[name] = []; for (let i = 0; i < n; i++) out[name].push(await shot({ ...o, t: 2 + i / 12, lamp: lamp(i / n * Math.PI * 2) })); };
  await loop('hero', 48, { sprite: 'charizard', place: 'cave' }, (a) => [96 - 74 * Math.cos(a), 60 - 46 * Math.sin(a)]);      // the torch circles the sprite
  await loop('gears', 40, { sprite: 'klinklang', place: 'studio' }, (a) => [96 + 50 * Math.cos(a), 30 + 8 * Math.sin(a)]);
  await group('places', [
    { sprite: 'klinklang', place: 'studio', t: 1.2, lamp: [44, 26] }, { sprite: 'charizard', place: 'cave', t: 1.2, lamp: [40, 72] },
    { sprite: 'kyurem', place: 'rain', t: 1.2, lamp: [150, 58] }, { sprite: 'arcanine', place: 'field', t: 1.2, lamp: [150, 66] },
    { sprite: 'corsoreef', place: 'snow', t: 1.2, lamp: [150, 24] }, { sprite: 'jellicent', place: 'deep', t: 1.2, lamp: [44, 58] }]);
  await group('pixel', [{ sprite: 'charizard', ...studio, flat: true }, { sprite: 'charizard', ...studio, tab: 'surface' }, { sprite: 'charizard', ...studio, tab: 'materials', byMat: true }, { sprite: 'charizard', ...studio, tab: 'glow' }]);
  await group('light', [{ sprite: 'charizard', ...studio }, { sprite: 'charizard', ...studio, lamp: [152, 28] }, { sprite: 'charizard', place: 'cave', t: 1.2, lamp: [128, 44], behind: true }, { sprite: 'charizard', place: 'rain', t: 6.9, lamp: [44, 60] }]);
  await group('alone', [{ sprite: 'kyurem', ...studio, flat: true }, { sprite: 'kyurem', ...studio, pure: true }, { sprite: 'kyurem', ...studio, pure: true, lamp: [152, 28] }, { sprite: 'kyurem', ...studio, pure: true, lamp: [150, 88] }]);
  await group('palettes', ['normal', 'fire', 'water', 'grass', 'electric', 'ghost', 'ice', 'dragon'].map((k) => ({ sprite: 'arceus', ...studio, tab: 'looks', palBase: k })));
  await group('looks', [{ sprite: 'jellicent', ...studio, tab: 'looks' }, { sprite: 'jellicent', ...studio, tab: 'looks', looks: { female: true } }, { sprite: 'jellicent', ...studio, tab: 'looks', looks: { female: true }, blink: true }, { sprite: 'jellicent', ...studio, tab: 'looks', looks: { female: true }, shiny: true }]);
  await group('parts', [{ sprite: 'charizard', ...studio }, { sprite: 'charizard', ...studio, tab: 'parts' }, { sprite: 'charizard', ...studio, tab: 'parts', explode: 0.5 }]);
  await group('views', [{ sprite: 'charizard', ...studio }, { sprite: 'charizard', ...studio, view: 'back' }, { sprite: 'arceus', ...studio }, { sprite: 'arceus', ...studio, view: 'back' }]);
  await group('soft', [{ sprite: 'titanice', ...studio, flat: true }, { sprite: 'titanice', place: 'rain', t: 1.2, lamp: [150, 50] }, { sprite: 'corsoreef', ...studio, flat: true }, { sprite: 'corsoreef', place: 'cave', t: 1.2, lamp: [150, 60] }]);
  // the two-dial material map, and the sheets as stored
  await p.evaluate((s) => window.lab.set(s), { ...base, sprite: 'charizard', tab: 'materials', place: 'studio' }); await p.waitForTimeout(300);
  await (await p.$('#map')).screenshot({ path: __dirname + '/out/readme-map.png' });
  await p.evaluate((s) => window.lab.set(s), { ...base, sprite: 'charizard', tab: 'file', place: 'studio' }); await p.waitForTimeout(300);
  out.sheets = await p.evaluate(() => ['sheet-color', 'sheet-light', 'sheet-glow'].map((id) => document.getElementById(id).toDataURL('image/png')));
  fs.writeFileSync(__dirname + '/out/readme-shots.json', JSON.stringify(out));
  console.log(Object.entries(out).map(([k, v]) => k + ':' + v.length).join(' '), 'errors', JSON.stringify(errs)); await b.close();
})();
