// ---------------------------------------------------------------- the material spheres page
const fmt = (v) => (v > 0 ? '+' : '') + String(Math.round(v * 100) / 100);
const COLORS = [['Clay', MONO], ['Red', ['#6b1010', '#b52121', '#ef4231', '#ff8c73']], ['Gold', ['#7b5208', '#bd8c10', '#efc531', '#fff39c']],
                ['Green', ['#1a4a19', '#317b29', '#5aad42', '#9cd66b']], ['Blue', ['#3142a5', '#5263c5', '#8cadff', '#cedeff']],
                ['Violet', ['#3a104a', '#6b218c', '#9c42c5', '#c57bef']], ['Gray', ['#4a525a', '#84949c', '#b5c5ce', '#e6eff7']], ['White', ['#8494b5', '#b5c5de', '#dee6f7', '#ffffff']]];
const M = { name: 'Ice', c: COLORS[4][1].slice(), f: .94, b: .82, g: 0 };          // the ball on the stage
let chosen = null;
function seg(el, items, get, set){
  el.innerHTML = '';
  items.forEach(([value, label]) => { const b = document.createElement('button'); b.type = 'button'; b.textContent = label; b.setAttribute('aria-pressed', String(get() === value));
    b.onclick = () => { set(value); [...el.children].forEach((c, i) => c.setAttribute('aria-pressed', String(items[i][0] === get()))); }; el.appendChild(b); });
}
function nearest(){ let best = null, d = 9; for(const m of NAMED){ const q = Math.hypot(m.f - M.f, (m.b - M.b) * 1.2); if(q < d){ d = q; best = m; } } return d < .16 ? best.name : null; }
function apply(){
  S.ball = { ...M, glow: M.g > 0 ? [M.g * .35, M.g * .55, M.g * .8, M.g] : null }; setBall(S.ball);
  const like = nearest();
  $('#words').textContent = finishWord(M.f) + ', ' + bodyWord(M.b) + (M.g > 0 ? ', glowing' : '');
  $('#like').textContent = like ? 'About the same as ' + like.toLowerCase() + '.' : 'Not one of the named materials below.';
  for(const [id, v] of [['f', M.f], ['b', M.b], ['g', M.g]]){ $('#d' + id).value = v; $('#n' + id).textContent = id === 'g' ? String(Math.round(v * 100) / 100) : fmt(v); }
  [...$('#colors').children].forEach((b, i) => b.setAttribute('aria-pressed', String(COLORS[i][1].join() === M.c.join())));
  document.querySelectorAll('#map button, #balls button').forEach((b) => b.setAttribute('aria-pressed', String(b === chosen)));
}
function start(){
  seg($('#place'), Object.entries(PLACES).map(([k, p]) => [k, p.name]), () => S.place, (x) => { S.place = x; $('#place-hint').textContent = PLACES[x].hint; });
  $('#place-hint').textContent = PLACES[S.place].hint;
  const behind = $('#behind'); behind.onclick = () => { S.behind = !S.behind; behind.setAttribute('aria-pressed', String(S.behind)); };
  for(const [id, key] of [['f', 'f'], ['b', 'b'], ['g', 'g']]) $('#d' + id).oninput = (e) => { M[key] = Number(e.target.value); chosen = null; apply(); };
  COLORS.forEach(([name, c]) => { const b = document.createElement('button'); b.type = 'button'; b.title = name; b.setAttribute('aria-label', name); b.style.background = `linear-gradient(135deg, ${c[3]} 0 25%, ${c[2]} 25% 55%, ${c[1]} 55% 80%, ${c[0]} 80%)`;
    b.onclick = () => { M.c = c.slice(); apply(); }; $('#colors').appendChild(b); });
  const at = (e) => { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left) / r.width * SW, (e.clientY - r.top) / r.height * SH]; };
  canvas.addEventListener('pointermove', (e) => { S.pointer = at(e); S.pointerAt = performance.now(); $('#hint').hidden = true; });
  canvas.addEventListener('pointerdown', (e) => { S.pointer = at(e); S.pointerAt = performance.now(); canvas.setPointerCapture(e.pointerId); });
  const drop = () => { S.pointer = null; S.pointerAt = performance.now(); };
  canvas.addEventListener('pointerleave', drop); canvas.addEventListener('pointerup', (e) => { if(e.pointerType !== 'mouse') drop(); });
  new ResizeObserver(resize).observe(canvas); resize();
  if(matchMedia('(prefers-reduced-motion: reduce)').matches) S.calm = true;
  // every preset is a small still of the same ball, drawn by the same lighting
  const card = (m, src, host, label, keepColor) => { const b = document.createElement('button'); b.type = 'button'; b.title = m.name; b.setAttribute('aria-pressed', 'false');
    b.innerHTML = `<img alt="${m.name}" src="${src}">` + (label ? `<span>${m.name}</span>` : '');
    b.onclick = () => { Object.assign(M, { name: m.name, f: m.f, b: m.b, g: m.glow ? m.glow[3] : 0 }); if(keepColor) M.c = m.c.slice(); chosen = b; apply(); canvas.scrollIntoView({ block: 'nearest', behavior: 'smooth' }); }; host.appendChild(b); };
  const shots = ballThumbs(MAP), map = $('#map'); map.appendChild(document.createElement('span'));
  BODY.forEach(([, n]) => { const h = document.createElement('span'); h.className = 'col'; h.textContent = n; map.appendChild(h); });
  FINISH.forEach(([, n], r) => { const h = document.createElement('span'); h.className = 'row'; h.textContent = n; map.appendChild(h);
    BODY.forEach((_, c) => card(MAP[r * BODY.length + c], shots[r * BODY.length + c], map, false, false)); });
  ballThumbs(NAMED).forEach((src, i) => card(NAMED[i], src, $('#balls'), true, true));
  S.place = 'studio'; S.lamp = [50, 30]; apply();
  requestAnimationFrame((n) => { last = n; frame(n); });
}
// for checking the page from a script
window.lab = { set(o){ Object.assign(M, o.m || {}); if(o.place) S.place = o.place; if(o.behind !== undefined) S.behind = o.behind; chosen = null; apply();
  if(o.t !== undefined){ S.fixed = true; S.t = o.t; } if(o.lamp) S.lamp = o.lamp; draw(); return canvas.toDataURL('image/png'); } };
if(gl) start();
