// Runs inside the lab page. Assemble both front looks from their positions on the file's actual color sheet.
module.exports = function sharedSheetFrame(seconds) {
  const view = current(), [w, h] = view.atlas;
  if (!view.tourSheet) {
    const sheet = document.createElement('canvas');
    sheet.width = w; sheet.height = h;
    const ctx = sheet.getContext('2d'), pixels = ctx.createImageData(w, h);
    for (let i = 0; i < pixels.data.length; i += 4) {
      pixels.data.set(view.palettes.normal[view.rawColor[i]], i);
      pixels.data[i + 3] = view.rawColor[i + 3];
    }
    ctx.putImageData(pixels, 0, 0);
    view.tourSheet = sheet;
  }

  const stage = document.createElement('canvas');
  stage.width = 384; stage.height = 256;
  const ctx = stage.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.fillStyle = '#11151c';
  ctx.fillRect(0, 0, stage.width, stage.height);

  const clamp = x => Math.min(1, Math.max(0, x));
  const ease = x => { x = clamp(x); return x * x * (3 - 2 * x); };
  const travel = (seconds - 1.65) / 2.1;
  const sheetX = Math.floor((stage.width - w) / 2), sheetY = Math.floor((stage.height - h) / 2);
  const blend = (a, b, k) => a + (b - a) * k;

  // Start with the intact sheet, including alternate eyes. Unused pieces fade as the two sets leave.
  ctx.globalAlpha = 1 - ease(travel * 2.4);
  for (let y = 0; y < h; y += 8) for (let x = 0; x < w; x += 8) {
    ctx.fillStyle = (x + y) % 16 ? '#242c37' : '#202731';
    ctx.fillRect(sheetX + x, sheetY + y, Math.min(8, w - x), Math.min(8, h - y));
  }
  const animation = view.anims.find(a => a.id === 'idle');
  const tick = Math.floor(Math.max(0, seconds - 3.1) * view.tps) % animation.length;
  const frame = animation.starts.findLastIndex(start => start <= tick);
  const buf = animation.buf, offset = animation.offsets[frame], count = buf[offset + 12];
  const active = new Set();
  const female = view.looks.find(l => l.id === 'female').swaps;
  const pieces = [];
  for (const [side, replace] of [{}, female].entries()) {
    for (let i = 0; i < count; i++) {
      const o = offset + 13 + i * 8, id = replace[buf[o]] ?? buf[o];
      active.add(id);
      pieces.push({ id, side, i, o });
    }
  }
  for (const [id, [x, y, dw, dh]] of view.drawings.entries()) {
    if (!active.has(id) && dw && dh) ctx.drawImage(view.tourSheet, x, y, dw, dh, sheetX + x, sheetY + y, dw, dh);
  }
  ctx.globalAlpha = 1;
  const cx = (view.bbox[0] + view.bbox[2]) / 2, cy = (view.bbox[1] + view.bbox[3]) / 2;
  for (const { id, side, i, o } of pieces) {
    const [x, y, dw, dh, dx, dy] = view.drawings[id];
    const [a, b, c, d, tx, ty] = buf.subarray(o + 1, o + 7);
    const k = ease(travel * 1.18 - .18 * i / count);
    const px = (side ? 286 : 98) + 2 * (a * dx + c * dy + tx - cx);
    const py = 137 + 2 * (b * dx + d * dy + ty - cy);
    ctx.setTransform(blend(1, 2 * a, k), 2 * b * k, 2 * c * k, blend(1, 2 * d, k),
                     Math.round(blend(sheetX + x, px, k)), Math.round(blend(sheetY + y, py, k)));
    ctx.drawImage(view.tourSheet, x, y, dw, dh, 0, 0, dw, dh);
  }
  return stage.toDataURL('image/png');
};
