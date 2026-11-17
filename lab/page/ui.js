// ---------------------------------------------------------------- page
const TABS = [['light', 'Light'], ['materials', 'Materials'], ['surface', 'Surface'], ['glow', 'Glow'], ['parts', 'Parts'], ['motion', 'Motion'], ['views', 'Views'], ['looks', 'Looks'], ['file', 'File']];
const fmt = (v) => (v > 0 ? '+' : '') + String(Math.round(v * 100) / 100);
const partHue = (i) => `hsl(${Math.round(((i * .618 + .05) % 1) * 360)} 100% 50%)`;
const sprite = () => DATA.sprites[S.sprite];
function seg(el, items, get, set){
  el.innerHTML = '';
  items.forEach(([value, label, off]) => {
    const b = document.createElement('button'); b.type = 'button'; b.textContent = label; b.disabled = !!off; b.setAttribute('aria-pressed', String(get() === value));
    b.onclick = () => { set(value); [...el.children].forEach((c, i) => c.setAttribute('aria-pressed', String(items[i][0] === get()))); };
    el.appendChild(b);
  });
}
function tog(id, get, set){ const b = $('#' + id); b.setAttribute('aria-pressed', String(get())); b.onclick = () => { set(!get()); b.setAttribute('aria-pressed', String(get())); }; }
function setMode(){
  S.mode = S.tab === 'light' ? (S.flat ? 1 : 0) : S.tab === 'materials' ? (S.byMat ? 3 : 0) : S.tab === 'surface' ? 2 : S.tab === 'glow' ? (S.glowOnly ? 4 : 0) : S.tab === 'parts' ? 5 : 0;
}
function traits(r){
  if(!r.ramp.length || r.lib === 'outline') return ['line art', 'takes light flat'];
  return [finishWord(r.f), bodyWord(r.b)].concat(r.glow ? ['glows'] : []);
}
function materialList(){
  if(S.ball){
    const m = S.ball;
    const say = () => { $('#mat-title').textContent = m.name; $('#words').textContent = finishWord(m.f) + ', ' + bodyWord(m.b) + (m.glow ? ', glows' : ''); };
    const dial = (id, label, ends, v) => `<li class="dial"><label for="${id}">${label}</label><span class="n" id="n${id}">${fmt(v)}</span><input id="${id}" type="range" min="-1" max="1" step="0.05" value="${v}"><span class="ends">${ends.map(([e, at]) => `<i style="${at === 0 ? 'left:0' : at === 100 ? 'right:0' : 'left:' + at + '%;transform:translateX(-50%)'}">${e}</i>`).join('')}</span></li>`;
    $('#mats').innerHTML = `<li class="val"><span>Shades</span><span class="shades">${m.c.map((c) => `<i style="background:${c}"></i>`).join('')}</span><span></span></li>`
      + dial('df', 'Finish', [['fuzzy', 0], ['matte', 50], ['polished', 100]], m.f) + dial('db', 'Body', [['metal', 0], ['solid', 50], ['translucent', 75], ['clear', 100]], m.b)
      + `<li class="note"><b id="words"></b>. Drag either dial. ${m.glow ? 'Glow is painted per pixel. ' : ''}${m.on ? 'This setting is used on ' + m.on + '.' : ''}</li>`;
    say();
    for(const [id, key] of [['df', 'f'], ['db', 'b']]) $('#' + id).oninput = (e) => { m[key] = Number(e.target.value); BALL.rows[1][key] = m[key]; $('#n' + id).textContent = fmt(m[key]); say(); };
    return;
  }
  const v = current(), pal = v.palettes[palId(v)];
  $('#mat-title').textContent = v.title + ' is made of';
  $('#mats').innerHTML = v.rows.map((r, i) => `<li><span class="num">${i}</span><span class="shades">${r.ramp.map((c) => `<i style="background:rgb(${pal[c].slice(0, 3).join(',')})"></i>`).join('')}</span><span class="what"><b>${r.name}</b><span>${traits(r).join(' · ')}</span></span></li>`).join('');
}
// the stored sheets, drawn exactly as the numbers in them
function sheets(){
  const v = current(); if(S.ball || !v.rawColor) return;
  const [w, h] = v.atlas, pal = v.palettes[palId(v)], n = w * h;
  const paint = (id, px) => { const c = $('#' + id); c.width = w; c.height = h; c.style.aspectRatio = w + ' / ' + h; const img = new ImageData(w, h); px(img.data); c.getContext('2d').putImageData(img, 0, 0); };
  paint('sheet-color', (o) => { for(let i = 0; i < n; i++) if(v.rawColor[i * 4 + 3]){ const c = pal[v.rawColor[i * 4]]; o.set([c[0], c[1], c[2], v.rawColor[i * 4 + 3]], i * 4); } });
  paint('sheet-light', (o) => { for(let i = 0; i < n; i++) if(v.rawColor[i * 4 + 3]) o.set([v.rawLight[i * 4], v.rawLight[i * 4 + 1], Math.min(255, v.rawLight[i * 4 + 2] * 36), 255], i * 4); });
  paint('sheet-glow', (o) => { for(let i = 0; i < n; i++) if(v.rawColor[i * 4 + 3]){ const g = v.rawLight[i * 4 + 3]; o.set([g, g, g, 255], i * 4); } });
}
const PANELS = {
  light: () => `<h2>Lit by wherever it stands</h2>
    <p>The file says which way each pixel faces, what it is made of and how much it glows. The place supplies the light. Nothing is redrawn between places.</p>
    <div class="seg wrapping" id="place"></div><p class="hint" id="place-hint"></p>
    <div class="row"><button class="tog" id="behind" type="button">Light from behind</button><button class="tog" id="flat" type="button">Lighting off</button><button class="tog" id="pure" type="button">Light alone</button></div>
    <p class="small">Light never invents a color. Each material lists its own shades, dark to light, and light only moves a pixel along that list, so it still reads as pixel art. With the lighting off you see the exact picture every program must draw. Light alone throws away the shade the artist painted and works it out again from the facing data. It should change almost nothing: the facing data is built so that light from the upper left lands on the painted shades.</p>`,
  materials: () => `<h2>Every pixel names its material</h2>
    <p>A sprite carries a short list of materials. Each is two dials. <b>Finish</b>: fuzzy, matte or polished. <b>Body</b>: metal, solid, translucent or clear. A pixel stores only which material it is.</p>
    <div class="row"><button class="tog" id="bymat" type="button">Color by material</button></div>
    <h3 id="mat-title"></h3><ul id="mats"></ul>`,
  surface: () => `<h2>Which way every pixel faces</h2>
    <p>Two numbers per pixel. In this view red runs left to right and green runs bottom to top, so a pixel tinted green faces up.</p>
    <p>Nothing here was hand-painted. The sprite is posed and the whole picture is blown up like a balloon: the silhouette gives the roundness and every outline the artist drew becomes a crease. Then each pixel is turned toward or away from the upper left, just far enough that light from there picks the shade the artist painted. So the artist's shading is in the facing data, and light from anywhere else stays consistent with it.</p>
    <p class="small">Rain wets and snow settles on whatever faces up. Try both under Light.</p>`,
  glow: () => `<h2>Glow, and what it lights</h2>
    <p>Each pixel stores how much it glows, in its own color. A place can treat glowing pixels as lamps: a tail flame lights the wall, the floor and the rest of the body.</p>
    <div class="seg" id="glowshow"></div>
    <div class="row"><button class="tog" id="glowlights" type="button">Glow lights the room</button></div>
    <p class="hint">Clearest in the Cave with Charizard, Chandelure or Rotom. Switch the room lighting off and on to see the difference.</p>`,
  parts: () => `<h2>Parts that move</h2>
    <p>A sprite is a set of parts. Each part shows one drawing at a time and can move, turn, stretch and swap to another drawing. Lighting data is stored per drawing, so it travels with the part.</p>
    <label class="slide" for="apart">Pull apart<input id="apart" type="range" min="0" max="1" step="0.02" value="${S.explode}"></label>
    <ul class="legend" id="legend"></ul>`,
  motion: () => `<h2>Animation</h2>
    <p>Time is counted in ticks. At each tick every part has a position, a turn, a stretch and a drawing. Drag to step through; the light keeps moving.</p>
    <div class="seg" id="animseg"></div>
    <label class="slide" for="tick"><span id="tickread"></span><input id="tick" type="range" min="0" max="1" step="1" value="0"></label>
    <div class="row"><button class="tog" id="pause" type="button">Pause</button><div class="seg" id="speed"></div></div>
    <p class="small" id="motionstats"></p>`,
  views: () => `<h2>Front and back</h2>
    <p>One sprite holds several views of the same creature. Each view has its own pair of sheets, so a game loads only the angles it uses. Materials and lighting data exist for every view.</p>
    <div class="seg" id="viewseg"></div>
    <p class="small">Parts, drawings and animations are shared by every view. Each view keeps its own order of parts and its own length for each animation.</p>`,
  looks: () => `<h2>Alternate looks</h2>
    <p>A look reuses the same animation. Some swap the <b>palette</b>: glow and lighting follow the new colors, because they are worked out from the pixel's color. Some swap <b>drawings</b>: closed eyes, or a whole different body. Looks combine. On the other tabs this page uses Eyes closed to blink.</p>
    <div class="seg wrapping" id="palseg"></div>
    <div class="row" id="lookrow"><button class="tog" id="shiny" type="button">Shiny</button><button class="tog" id="blink" type="button">Eyes closed</button></div>
    <p class="small" id="lookstats"></p>`,
  file: () => `<h2>What is in the file</h2>
    <ul class="facts">
      <li><b>Per pixel:</b> color number, opacity, which way it faces, material number, glow. Two images per view.</li>
      <li><b>Per material:</b> a name, its shades, and two dials.</li>
      <li><b>Per sprite:</b> parts, drawings, animation, palettes and looks.</li>
    </ul>
    <p class="small" id="filestats"></p>`,
};
const WIDE = {
  materials: `<h2>A material is two dials</h2>
    <p>Every ball is the same clay color, lit like the sprites. Only the dials differ: finish down the side, body across the top. Pick one to put it on the stage and drag its dials.</p>
    <div id="map"></div>
    <h2 class="sub">Named materials are points on that map, plus a color</h2>
    <p>The first twelve are the settings these sprites use.</p>
    <div id="balls"></div>`,
  file: `<h2>The two sheets, as stored</h2>
    <p>Every drawing sits at the same place on both sheets, so one rectangle finds its color and its lighting.</p>
    <div class="sheets">
      <figure><canvas id="sheet-color"></canvas><figcaption><b>Color.</b> A palette color and an opacity per pixel.</figcaption></figure>
      <figure><canvas id="sheet-light"></canvas><figcaption><b>Light.</b> Red and green are which way the pixel faces; blue is its material number.</figcaption></figure>
      <figure><canvas id="sheet-glow"></canvas><figcaption><b>Glow.</b> The fourth channel of the Light sheet.</figcaption></figure>
    </div>`,
};
let ballShots = null;
function buildGallery(){
  const card = (m, src, host, label) => {
    const b = document.createElement('button'); b.type = 'button'; b.setAttribute('aria-pressed', 'false'); b.title = m.name;
    b.innerHTML = `<img alt="${m.name}" src="${src}">` + (label ? `<span>${m.name}</span>` : '');
    b.onclick = () => { pickBall(m, b); canvas.scrollIntoView({ block: 'nearest', behavior: 'smooth' }); }; host.appendChild(b);
  };
  if(!ballShots) ballShots = [ballThumbs(MAP), ballThumbs(NAMED)];
  const map = $('#map'); map.appendChild(document.createElement('span'));
  BODY.forEach(([, n]) => { const h = document.createElement('span'); h.className = 'col'; h.textContent = n; map.appendChild(h); });
  FINISH.forEach(([, n], r) => {
    const h = document.createElement('span'); h.className = 'row'; h.textContent = n; map.appendChild(h);
    BODY.forEach((_, c) => card(MAP[r * BODY.length + c], ballShots[0][r * BODY.length + c], map, false));
  });
  ballShots[1].forEach((src, i) => card(NAMED[i], src, $('#balls'), true));
}
function pickBall(m, el){ S.ball = { ...m, c: m.c.slice() }; S.ballEl = el || null; setBall(S.ball); mark(); materialList(); }
const mark = () => {
  [...$('#sprites').children].forEach((b, j) => b.setAttribute('aria-pressed', String(!S.ball && j === S.sprite)));
  document.querySelectorAll('#map button, #balls button').forEach((b) => b.setAttribute('aria-pressed', String(b === S.ballEl)));
};
// fill the side panel for the chosen tab and wire its controls
function show(tab){
  if(tab !== 'materials' && S.ball){ S.ball = null; S.ballEl = null; }
  S.tab = tab; setMode(); syncAnim();
  [...$('#tabs').children].forEach((b, i) => b.setAttribute('aria-pressed', String(TABS[i][0] === tab)));
  $('#panel').innerHTML = PANELS[tab]();
  const wide = $('#wide'); wide.hidden = !WIDE[tab]; wide.innerHTML = WIDE[tab] || '';
  const v = current(), sp = sprite();
  if(tab === 'light'){
    const hint = () => { $('#place-hint').textContent = PLACES[S.place].hint; };
    seg($('#place'), Object.entries(PLACES).map(([k, p]) => [k, p.name]), () => S.place, (x) => { S.place = x; S.placeChosen = true; hint(); }); hint();
    tog('behind', () => S.behind, (x) => { S.behind = x; }); tog('flat', () => !!S.flat, (x) => { S.flat = x; setMode(); }); tog('pure', () => !!S.pure, (x) => { S.pure = x; });
  }
  if(tab === 'materials'){ tog('bymat', () => !!S.byMat, (x) => { S.byMat = x; setMode(); }); materialList(); buildGallery(); }
  if(tab === 'glow'){
    seg($('#glowshow'), [[true, 'Glow only'], [false, 'Lit']], () => S.glowOnly !== false, (x) => { S.glowOnly = x; setMode(); });
    if(S.glowOnly === undefined){ S.glowOnly = true; setMode(); }
    tog('glowlights', () => S.glowLights, (x) => { S.glowLights = x; });
  }
  if(tab === 'parts'){
    $('#apart').oninput = (e) => { S.explode = Number(e.target.value); };
    $('#legend').innerHTML = v.parts.map((p, i) => `<li><i style="background:${partHue(i)}"></i>${p}</li>`).join('');
  }
  if(tab === 'motion'){
    const pick = $('#animseg'); pick.hidden = v.anims.length < 2;
    seg(pick, v.anims.map((an) => [an.id, an.name]), () => v.anims[v.animIndex].id, (x) => { S.animId = x; S.anim = 0; show('motion'); });
    const t = $('#tick'); t.max = v.length - 1;
    t.oninput = () => { S.paused = true; S.anim = (Number(t.value) + .5) / v.tps; pauseBtn(); };
    const pauseBtn = () => { const b = $('#pause'); b.setAttribute('aria-pressed', String(S.paused)); b.textContent = S.paused ? 'Play' : 'Pause'; };
    $('#pause').onclick = () => { S.paused = !S.paused; pauseBtn(); }; pauseBtn();
    seg($('#speed'), [[.25, '¼ speed'], [.5, '½ speed'], [1, 'Full speed']], () => S.speed, (x) => { S.speed = x; });
    $('#motionstats').textContent = `${v.length} ticks at ${v.tps} a second, ${v.parts.length} parts, ${v.keys.toLocaleString('en')} keys, ${v.starts.length} distinct poses.`;
  }
  if(tab === 'views') seg($('#viewseg'), Object.keys(sp.views).map((id) => [id, id[0].toUpperCase() + id.slice(1)]), () => S.view, (x) => { S.view = x; syncAnim(); });
  if(tab === 'looks'){
    const plain = (l) => !Object.keys(l.swaps).length, bases = v.looks.filter((l) => l.palette && plain(l) && l.id !== 'shiny' && !l.id.endsWith('-shiny'));
    const pals = $('#palseg'); pals.hidden = !bases.length;
    seg(pals, [['normal', 'Normal'], ...bases.map((l) => [l.palette, l.name])], () => S.palBase, (x) => { S.palBase = x; });
    tog('shiny', () => S.shiny, (x) => { S.shiny = x; }); $('#shiny').disabled = !sp.palettes.shiny;
    tog('blink', () => S.blink, (x) => { S.blink = x; }); $('#blink').disabled = !Object.keys(v.blink || {}).length;
    const swaps = v.looks.filter((l) => !plain(l));
    swaps.forEach((l) => { const b = document.createElement('button'); b.className = 'tog'; b.type = 'button'; b.id = 'look-' + l.id; b.textContent = l.name; $('#lookrow').appendChild(b); tog(b.id, () => !!S.looks[l.id], (x) => { S.looks[l.id] = x; }); });
    const n = Object.keys(sp.palettes).length, m = swaps.length + (Object.keys(v.blink || {}).length ? 1 : 0);
    $('#lookstats').textContent = `${v.title} carries ${n} palette${n === 1 ? '' : 's'} and ${m} look${m === 1 ? ' that swaps' : 's that swap'} drawings.`;
  }
  if(tab === 'file'){
    $('#filestats').textContent = `${v.title}.spu holds ${Object.keys(sp.views).length > 1 ? 'both views' : 'this view'} in ${Math.max(1, Math.round(v.bytes / 1024))} KB: ${sp.fileParts} parts, ${sp.fileDrawings} drawings, ${Object.keys(sp.palettes).length} palettes, ${v.rows.length} materials. This view uses ${v.colors} colors and has ${v.anims.length} animation${v.anims.length === 1 ? '' : 's'}. The page reads the file itself; drop any .spu file on the stage to see it.`;
    sheets();
  }
}
onFrame = () => {
  if(S.tab !== 'motion') return; const v = current(), t = $('#tick'); if(!t) return;
  const tick = tickNow(v); if(document.activeElement !== t || !S.paused) t.value = tick;
  $('#tickread').textContent = `Tick ${tick} of ${v.length}`;
};
async function pick(i){
  S.sprite = i; S.ball = null; S.ballEl = null; const sp = DATA.sprites[i]; await load(sp);
  if(!sp.views[S.view]) S.view = Object.keys(sp.views)[0];
  S.palBase = 'normal'; S.looks = {};
  if(!S.placeChosen) S.place = sp.scene;
  mark(); show(S.tab);
}
function addSprite(sp, i){
  const b = document.createElement('button'); b.type = 'button'; b.setAttribute('aria-pressed', 'false');
  b.innerHTML = (sp.thumb ? `<img alt="" src="data:image/png;base64,${sp.thumb}">` : '') + `<span></span>`; b.lastChild.textContent = sp.title || sp.id;
  b.onclick = () => pick(i); $('#sprites').appendChild(b);
}
function boot(){
  TABS.forEach(([id, label]) => { const b = document.createElement('button'); b.type = 'button'; b.textContent = label; b.onclick = () => show(id); $('#tabs').appendChild(b); });
  DATA.sprites.forEach(addSprite);
  const at = (e) => { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left) / r.width * SW, (e.clientY - r.top) / r.height * SH]; };
  canvas.addEventListener('pointermove', (e) => { S.pointer = at(e); S.pointerAt = performance.now(); $('#hint').hidden = true; });
  canvas.addEventListener('pointerdown', (e) => { S.pointer = at(e); S.pointerAt = performance.now(); canvas.setPointerCapture(e.pointerId); });
  const drop = () => { S.pointer = null; S.pointerAt = performance.now(); };
  canvas.addEventListener('pointerleave', drop); canvas.addEventListener('pointerup', (e) => { if(e.pointerType !== 'mouse') drop(); });
  // any .spu file dropped on the stage joins the list
  const stage = canvas.parentElement;
  stage.addEventListener('dragover', (e) => e.preventDefault());
  stage.addEventListener('drop', async (e) => {
    e.preventDefault(); const file = e.dataTransfer.files[0]; if(!file) return;
    const sp = { id: file.name, scene: S.place, bytes: new Uint8Array(await file.arrayBuffer()) };
    try { await load(sp); } catch(err){ fail(file.name + ': ' + err.message); setTimeout(() => { $('#fail').hidden = true; }, 4000); return; }
    DATA.sprites.push(sp); addSprite(sp, DATA.sprites.length - 1); pick(DATA.sprites.length - 1);
  });
  new ResizeObserver(resize).observe(canvas); resize();
  if(matchMedia('(prefers-reduced-motion: reduce)').matches){ S.paused = true; S.calm = true; }
  pick(0).then(() => requestAnimationFrame((n) => { last = n; frame(n); }));
}
// for checking the page from a script: freeze a moment and choose what to look at
window.lab = { async set(o){
  if(o.sprite !== undefined) await pick(typeof o.sprite === 'number' ? o.sprite : DATA.sprites.findIndex((s) => s.id.includes(o.sprite)));
  for(const k of ['view', 'shiny', 'palBase', 'looks', 'animId', 'blink', 'behind', 'flat', 'pure', 'byMat', 'glowOnly', 'glowLights', 'explode', 'explodeAlways', 'plain', 'noGlow']) if(o[k] !== undefined) S[k] = o[k];
  if(o.place){ S.place = o.place; S.placeChosen = true; }
  show(o.tab || S.tab);
  if(o.mode !== undefined) S.mode = o.mode;          // 0 lit, 1 colors only, 2 facing, 3 materials, 4 glow, 5 parts
  if(o.ball !== undefined) pickBall(typeof o.ball === 'string' ? NAMED.find((m) => m.name.toLowerCase() === o.ball) : { name: 'Test', c: MONO, ...o.ball });
  if(o.t !== undefined){ S.fixed = true; S.t = o.t; S.anim = o.anim ?? o.t; }
  if(o.lamp) S.lamp = o.lamp;
  draw();
  return canvas.toDataURL('image/png');
} };
if(gl) boot();
