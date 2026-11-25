// ---------------------------------------------------------------- reading .spu files
// A reader for the format in docs/spec.md, independent of the Rust library: it shares no code with it.
const SPU = (() => {
  const MAGIC = [0x89, 0x53, 0x50, 0x55, 0x0d, 0x0a, 0x1a, 0x0a];
  async function inflate(bytes){
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('deflate'));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }
  const table = new Uint32Array(256).map((_, n) => { let c = n; for(let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1; return c >>> 0; });
  function crc32(...parts){ let c = 0xffffffff; for(const p of parts) for(let i = 0; i < p.length; i++) c = table[(c ^ p[i]) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; }

  // A PNG with four bytes per pixel (8-bit RGBA, or 16-bit gray and alpha), not interlaced.
  async function png(bytes){
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength); let at = 8, width = 0, height = 0; const data = [];
    while(at < bytes.length){
      const length = view.getUint32(at), type = String.fromCharCode(...bytes.subarray(at + 4, at + 8));
      if(type === 'IHDR'){ width = view.getUint32(at + 8); height = view.getUint32(at + 12); }
      if(type === 'IDAT') data.push(bytes.subarray(at + 8, at + 8 + length));
      at += 12 + length;
    }
    const raw = await inflate(await new Blob(data).arrayBuffer().then((b) => new Uint8Array(b))), stride = width * 4, out = new Uint8Array(stride * height);
    for(let y = 0; y < height; y++){
      const filter = raw[y * (stride + 1)], row = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)), o = y * stride;
      for(let x = 0; x < stride; x++){
        const a = x >= 4 ? out[o + x - 4] : 0, b = y ? out[o + x - stride] : 0, c = x >= 4 && y ? out[o + x - stride - 4] : 0;
        const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
        out[o + x] = row[x] + (filter === 1 ? a : filter === 2 ? b : filter === 3 ? (a + b) >> 1 : filter === 4 ? (pa <= pb && pa <= pc ? a : pb <= pc ? b : c) : 0);
      }
    }
    return { width, height, data: out };
  }

  // A cursor over one chunk's data.
  function cursor(data){
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength); let at = 0;
    const step = (n) => { at += n; if(at > data.length) throw new Error('A chunk ends before its contents do'); return at - n; };
    const c = {
      u8: () => view.getUint8(step(1)), u16: () => view.getUint16(step(2), true), u32: () => view.getUint32(step(4), true), i32: () => view.getInt32(step(4), true),
      f32: () => view.getFloat32(step(4), true), f64: () => view.getFloat64(step(8), true),
      index: () => { const v = c.u16(); return v === 65535 ? null : v; },
      text: () => { const n = c.u16(); return new TextDecoder().decode(data.subarray(step(n), at)); },
      align: (to) => { while(at % to) at++; }, rest: () => data.subarray(at),
    };
    return c;
  }

  async function read(file){
    if(file.length < 16 || MAGIC.some((b, i) => file[i] !== b)) throw new Error('Not an SPU file');
    const head = new DataView(file.buffer, file.byteOffset, file.byteLength);
    const version = head.getUint16(8, true);
    if(version !== 2) throw new Error(`The header's version is ${version}; this reader reads version 2`);
    if(head.getUint32(12, true) !== file.length) throw new Error('The file is not the length its header says');
    const s = { views: [], parts: [], drawings: [], palettes: [], materials: [], variants: [], animations: [], extras: [], bytes: file.length };
    let counts = [];
    for(let at = 16; at < file.length;){
      const type = String.fromCharCode(...file.subarray(at, at + 4)), length = head.getUint32(at + 4, true), sum = head.getUint32(at + 8, true), flags = head.getUint32(at + 12, true);
      let data = file.subarray(at + 16, at + 16 + length);
      if(crc32(file.subarray(at, at + 4), data) !== sum) throw new Error('Chunk ' + type + ' is damaged');
      if(flags & 1) data = await inflate(data);
      at += 16 + length; while(at % 8) at++;
      const c = cursor(data);
      if(type === 'HEAD'){ s.name = c.text(); s.author = c.text(); s.version = c.u32(); s.ticksPerSecond = c.f32(); counts = Array.from({ length: 7 }, c.u16); }
      else if(type === 'VIEW') for(let i = 0; i < counts[0]; i++){
        const v = { id: c.text(), name: c.text(), x: c.i32(), y: c.i32(), width: c.u32(), height: c.u32(), parts: [], color: null, light: null };
        for(let n = c.u16(); n > 0; n--) v.parts.push({ part: c.u16(), parent: c.index() });
        s.views.push(v);
      }
      else if(type === 'PART') for(let i = 0; i < counts[1]; i++) s.parts.push({ id: c.text(), name: c.text(), extra: !!c.u8(), depth: c.f32() });
      else if(type === 'DRAW') for(let i = 0; i < counts[2]; i++){
        const d = { id: c.text(), name: c.text(), places: [] };
        for(let v = 0; v < counts[0]; v++) d.places.push(c.u8() ? { x: c.u16(), y: c.u16(), width: c.u16(), height: c.u16(), offsetX: c.f32(), offsetY: c.f32() } : null);
        s.drawings.push(d);
      }
      else if(type === 'PALS'){ const n = c.u32(); for(let i = 0; i < counts[3]; i++){ const p = { id: c.text(), name: c.text(), colors: [] }; for(let k = 0; k < n; k++) p.colors.push([c.u8(), c.u8(), c.u8(), c.u8()]); s.palettes.push(p); } }
      else if(type === 'MATS') for(let i = 0; i < counts[4]; i++){ const m = { name: c.text(), finish: c.f32(), body: c.f32(), base: c.u16(), shades: [] }; for(let n = c.u16(); n > 0; n--) m.shades.push(c.u16()); s.materials.push(m); }
      else if(type === 'VARS') for(let i = 0; i < counts[5]; i++){
        const v = { id: c.text(), name: c.text(), palette: c.index(), replace: [], show: [], hide: [] };
        for(let n = c.u16(); n > 0; n--) v.replace.push({ view: c.u16(), from: c.u16(), to: c.u16() });
        for(let n = c.u16(); n > 0; n--) v.show.push(c.u16());
        for(let n = c.u16(); n > 0; n--) v.hide.push(c.u16());
        s.variants.push(v);
      }
      else if(type === 'ANIM'){
        const a = { id: c.text(), name: c.text(), views: [] };
        for(let v = 0; v < counts[0]; v++){
          if(!c.u8()){ a.views.push(null); continue; }
          const m = { length: c.u32(), looping: !!c.u8(), tracks: [] };
          for(let n = c.u16(); n > 0; n--){
            const t = { part: c.u16(), image: [], numbers: [] }, k = c.u32(); c.align(4);
            const ticks = Array.from({ length: k }, c.u32); for(const tick of ticks) t.image.push([tick, c.index()]);
            for(let p = 0; p < 8; p++){
              c.align(4); const k = c.u32(), ticks = Array.from({ length: k }, c.u32); c.align(8);
              const values = Array.from({ length: k }, c.f64); t.numbers.push(ticks.map((tick, i) => ({ tick, value: values[i], ease: 0 })));
              for(const key of t.numbers[p]) key.ease = c.u8();
            }
            m.tracks.push(t);
          }
          a.views.push(m);
        }
        s.animations.push(a);
      }
      else if(type === 'SCOL'){ const v = c.u16(); s.views[v].color = await png(c.rest()); }
      else if(type === 'SLIT'){ const v = c.u16(); s.views[v].light = await png(c.rest()); }
      else if(type === 'xtra') s.extras.push({ path: c.text(), bytes: c.rest() });
      else if(type === 'IEND') break;
      else if(type[0] === type[0].toUpperCase()) throw new Error('This file needs something this reader does not know: chunk ' + type);
    }
    return s;
  }

  // ---- where a view's parts are at a tick (the reference picture's first three steps)
  const DEFAULTS = [0, 0, 0, 1, 1, 0, 1, 1], EASE = [(t) => 0, (t) => t, (t) => t * t, (t) => t * (2 - t), (t) => t * t * (3 - 2 * t)];
  function number(keys, tick, fallback){
    let i = 0; while(i < keys.length && keys[i].tick <= tick) i++;
    if(i === 0) return keys.length ? keys[0].value : fallback;
    const a = keys[i - 1], b = keys[i];
    return b && a.ease ? a.value + (b.value - a.value) * EASE[a.ease]((tick - a.tick) / (b.tick - a.tick)) : a.value;
  }
  function image(keys, tick){ let i = 0; while(i < keys.length && keys[i][0] <= tick) i++; return keys.length ? keys[Math.max(i - 1, 0)][1] : null; }
  const matrix = ([x, y, r, sx, sy]) => { const s = Math.sin(r * Math.PI / 180), c = Math.cos(r * Math.PI / 180); return [c * sx, s * sx, -s * sy, c * sy, x, y]; };
  const tidy = (v) => { const snapped = Math.round(v * 1048576) / 1048576; return Math.abs(snapped - v) < 1e-9 ? snapped : v; };
  function join(p, l){
    const m = matrix(p), turn = (p[3] < 0) !== (p[4] < 0) ? -l[2] : l[2];
    return [m[0] * l[0] + m[2] * l[1] + m[4], m[1] * l[0] + m[3] * l[1] + m[5], p[2] + turn, p[3] * l[3], p[4] * l[4]].map(tidy);
  }
  /// Back to front: { part (position in the view's list), drawing, opacity, matrix }. No looks applied.
  function placed(s, view, motion, tick){
    const v = s.views[view]; tick = motion.looping ? ((tick % motion.length) + motion.length) % motion.length : Math.max(0, Math.min(tick, motion.length - 1));
    const tracks = v.parts.map((p) => motion.tracks.find((t) => t.part === p.part) || null);
    const local = (i) => [0, 1, 2, 3, 4].map((k) => tracks[i] ? number(tracks[i].numbers[k], tick, DEFAULTS[k]) : DEFAULTS[k]);
    const list = [];
    v.parts.forEach((part, i) => {
      const t = tracks[i]; if(!t || s.parts[part.part].extra) return;
      const drawing = image(t.image, tick); if(drawing === null || !s.drawings[drawing].places[view]) return;
      const chain = [i]; while(v.parts[chain[chain.length - 1]].parent !== null) chain.push(v.parts[chain[chain.length - 1]].parent);
      let world = local(chain[chain.length - 1]); for(let k = chain.length - 2; k >= 0; k--) world = join(world, local(chain[k]));
      list.push({ z: number(t.numbers[5], tick, 0), part: i, drawing, opacity: Math.max(0, Math.min(1, number(t.numbers[6], tick, 1))), matrix: matrix(world) });
    });
    return list.sort((a, b) => a.z - b.z || a.part - b.part);
  }

  // ---- what this page draws from: one sprite, laid out the way the renderer wants it
  function model(s){
    const first = s.palettes[0].colors, palettes = Object.fromEntries(s.palettes.map((p) => [p.id, p.colors]));
    const depth = s.parts.map((p) => p.depth).find((d) => d) || 0, views = {};
    s.views.forEach((v, vi) => {
      const w = v.color.width, h = v.color.height, n = w * h, light = v.light ? v.light.data : new Uint8Array(n * 4), color = new Uint8Array(n * 4);
      const level = s.materials.map((m) => new Map(m.shades.map((c, i) => [c, i]))), used = new Set(), glows = s.materials.map(() => false); let soft = false;
      for(let i = 0; i < n; i++){
        const k = (v.color.data[i * 4] << 8) | v.color.data[i * 4 + 1], o = Math.round(v.color.data[i * 4 + 2] * first[k][3] / 255);    // a palette color's own opacity multiplies in
        if(!o) continue;
        if(!v.light){ light[i * 4] = light[i * 4 + 1] = 128; }
        const row = light[i * 4 + 2]; used.add(k); if(o < 250) soft = true; if(light[i * 4 + 3] && row < glows.length) glows[row] = true;
        color.set([Math.min(k, 255), level[row] && level[row].get(k) || 0, 0, o], i * 4);
      }
      // per drawing: where it sits, its opaque bounds, and where and what color its glow is
      const drawings = [], glowCols = [], bounds = [], spots = [];
      s.drawings.forEach((d) => {
        const p = d.places[vi]; if(!p){ drawings.push([0, 0, 0, 0, 0, 0]); glowCols.push(null); bounds.push(null); spots.push(null); return; }
        drawings.push([p.x, p.y, p.width, p.height, p.offsetX, p.offsetY]);
        let x0 = 1e9, y0 = 1e9, x1 = -1e9, y1 = -1e9, e = 0, gx = 0, gy = 0; const sums = s.palettes.map(() => [0, 0, 0]);
        for(let y = 0; y < p.height; y++) for(let x = 0; x < p.width; x++){
          const i = (p.y + y) * w + p.x + x; if(!color[i * 4 + 3]) continue;
          x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x + 1); y1 = Math.max(y1, y + 1);
          const g = light[i * 4 + 3] / 255; if(!g) continue;
          e += g; gx += x * g; gy += y * g; const k = (v.color.data[i * 4] << 8) | v.color.data[i * 4 + 1];
          s.palettes.forEach((pal, j) => { for(let c = 0; c < 3; c++) sums[j][c] += pal.colors[k][c] / 255 * g; });
        }
        bounds.push(x1 > x0 ? [p.offsetX + x0, p.offsetY + y0, p.offsetX + x1, p.offsetY + y1] : null);
        spots.push(e ? [gx / e + .5 + p.offsetX, gy / e + .5 + p.offsetY, e] : null);
        glowCols.push(e ? Object.fromEntries(s.palettes.map((pal, j) => [pal.id, sums[j].map((c) => c / e)])) : null);
      });
      // every distinct pose of every animation this view has: the glow lights, then each part's drawing and place
      const anims = [];
      for(const a of s.animations){
        const m = a.views[vi]; if(!m) continue;
        const starts = [], offsets = [], buf = []; let last = '';
        for(let t = 0; t < m.length; t++){
          const list = placed(s, vi, m, t).filter((p) => p.opacity > 0), key = JSON.stringify(list.map((p) => [p.drawing, p.part, p.matrix.map((x) => Math.round(x * 1e6))]));
          if(key === last) continue;
          last = key; starts.push(t); offsets.push(buf.length);
          const lights = [];
          for(const p of list){ const g = spots[p.drawing]; if(g) lights.push([p.matrix[0] * g[0] + p.matrix[2] * g[1] + p.matrix[4], p.matrix[1] * g[0] + p.matrix[3] * g[1] + p.matrix[5], g[2], p.drawing]); }
          lights.sort((a, b) => b[2] - a[2]); const merged = [];
          for(const l of lights){
            const q = merged.find((q) => (q[0] - l[0]) ** 2 + (q[1] - l[1]) ** 2 < 324);
            if(q){ const sum = q[2] + l[2]; q[0] = (q[0] * q[2] + l[0] * l[2]) / sum; q[1] = (q[1] * q[2] + l[1] * l[2]) / sum; q[2] = sum; } else merged.push(l.slice());
          }
          merged.sort((a, b) => b[2] - a[2]); for(let j = 0; j < 3; j++) buf.push(...(merged[j] || [0, 0, 0, 0]));
          buf.push(list.length); for(const p of list) buf.push(p.drawing, ...p.matrix, p.part);
        }
        const keys = m.tracks.reduce((sum, t) => sum + t.image.length + t.numbers.reduce((x, k) => x + k.length, 0), 0);
        anims.push({ id: a.id, name: a.name, length: m.length, keys, starts, offsets, buf: new Float32Array(buf) });
      }
      // where the sprite stands at the first tick, for centering it
      const bbox = [1e9, 1e9, -1e9, -1e9], m0 = s.animations.map((a) => a.views[vi]).find(Boolean);
      for(const p of m0 ? placed(s, vi, m0, 0) : []){
        const b = bounds[p.drawing]; if(!b) continue;
        for(const [x, y] of [[b[0], b[1]], [b[2], b[1]], [b[0], b[3]], [b[2], b[3]]]){
          const sx = p.matrix[0] * x + p.matrix[2] * y + p.matrix[4], sy = p.matrix[1] * x + p.matrix[3] * y + p.matrix[5];
          bbox[0] = Math.min(bbox[0], Math.floor(sx)); bbox[1] = Math.min(bbox[1], Math.floor(sy)); bbox[2] = Math.max(bbox[2], Math.ceil(sx)); bbox[3] = Math.max(bbox[3], Math.ceil(sy));
        }
      }
      const swaps = (variant) => Object.fromEntries(variant.replace.filter((r) => r.view === vi).map((r) => [r.from, r.to]));
      const closed = s.variants.find((x) => x.id === 'eyes-closed');
      views[v.id] = {
        canvas: [v.x, v.y, v.width, v.height], colors: used.size, soft, bbox, atlas: [w, h], rawColor: color, rawLight: light, lit: !!v.light, drawings, glowCols, anims,
        names: s.drawings.map((d) => d.id), parts: v.parts.map((p) => s.parts[p.part].id), blink: closed ? swaps(closed) : {},
        looks: s.variants.filter((x) => x.id !== 'eyes-closed').map((x) => ({ id: x.id, name: x.name, palette: x.palette === null ? null : s.palettes[x.palette].id, swaps: swaps(x) })).filter((l) => l.palette || Object.keys(l.swaps).length),
        rows: s.materials.length ? s.materials.map((m, i) => ({ name: m.name, lib: m.name, ramp: m.shades, base: m.base, glow: glows[i], f: m.finish, b: m.body })) : [{ name: 'plain', lib: 'plain', ramp: [], base: 0, glow: false, f: 0, b: 0 }],
        tps: s.ticksPerSecond, bytes: s.bytes,
      };
    });
    return { title: s.name, author: s.author, palettes, views, ground: 0, glowZ: depth, fileParts: s.parts.length, fileDrawings: s.drawings.length };
  }
  return { read, model, placed };
})();
