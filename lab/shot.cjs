// node shot.cjs out.png '[{"sprite":"charizard","place":"cave","t":1.2,"lamp":[60,90]}, ...]'  -> a grid of stage captures
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const fs = require('fs');
(async () => {
  const b = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined, args: ['--use-angle=metal', '--enable-unsafe-swiftshader'] });
  const p = await b.newPage({ viewport: { width: Number(process.env.VW || 1240), height: 900 } });
  const errs = []; p.on('pageerror', (e) => errs.push(e.message)); p.on('console', (m) => { if (m.type() === 'error') errs.push(m.text()); });
  const html = fs.readFileSync(__dirname + '/out/spu-material-lab.html', 'utf8');
  await p.setContent('<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"></head><body style="margin:0">' + html + '</body></html>');
  await p.waitForTimeout(1200);
  const shots = JSON.parse(process.argv[3] || '[]');
  if (process.argv[4] === 'page') { await p.screenshot({ path: process.argv[2], fullPage: true }); }
  else {
    const urls = [];
    for (const s of shots) { await p.evaluate((o) => window.lab.set(o), s); await p.waitForTimeout(40); urls.push(await p.evaluate((o) => window.lab.set(o), s)); }
    const cols = Number(process.argv[5] || 2); const iw = Number(process.argv[6] || 480);
    await p.setContent('<body style="margin:0;background:#111"><div style="display:grid;grid-template-columns:repeat(' + cols + ',1fr);gap:4px;width:' + (cols * (iw + 4)) + 'px">' + urls.map((u) => '<img style="width:' + iw + 'px;image-rendering:pixelated" src="' + u + '">').join('') + '</div></body>');
    await p.waitForTimeout(200);
    const el = await p.$('div'); await el.screenshot({ path: process.argv[2] });
  }
  console.log('errors', JSON.stringify(errs).slice(0, 1500)); await b.close();
})();
