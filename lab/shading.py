"""Build out/painted-or-flat.html: can light alone redraw the art?

Needs two captures of the lab page (see README): out/shading-shots.json with the facing data refit to the
painted shading, and out/shading-shots-old.json made with FIT=0, the blown-up shape only."""
import json, base64, io, os
import numpy as np
from PIL import Image
here = os.path.dirname(os.path.abspath(__file__))
shots = json.load(open(os.path.join(here, "out/shading-shots.json")))
old = json.load(open(os.path.join(here, "out/shading-shots-old.json")))
def crop(u):
    im = np.array(Image.open(io.BytesIO(base64.b64decode(u.split(",")[1]))).convert("RGB")); k = im.shape[1] // 192
    b = io.BytesIO(); Image.fromarray(im[k // 2::k, k // 2::k][8:124, 30:162]).save(b, "PNG", optimize=True)
    return "data:image/png;base64," + base64.b64encode(b.getvalue()).decode()
IMG = {k: crop(v) for k, v in shots.items() if not k.endswith("flat-off")}
IMG.update({"old-" + k: crop(v) for k, v in old.items() if "-flat-" in k and not k.endswith("-off")})
SPRITES = [
 ["charizard", "Charizard", "Light alone gives back the red shadow under the neck and along the tail. With the old facing data it washed out to plain orange."],
 ["metalynx", "Metalynx", "The stripes on its face and tail come back under the upper-left lamp. Lit from the right they dim: they are stored as lighter shades of the hide, so the fit reads them as bumps. Giving the markings their own material would fix that."],
 ["arcanine", "Arcanine", "The tufts of the mane and tail come back. With the old facing data the fur was one smooth lump."],
 ["kyurem", "Kyurem", "The facets the artist drew in the ice and the muscle lines in the gray body come back."],
]
LIGHTS = [["left", "Upper left", "The lamp is about where the artist imagined the light. Light alone should now give back the art."],
          ["right", "From the right", "The opposite side from the artist’s light. The two lit versions should still agree."],
          ["low", "From low right", "Light from below, which no sprite was painted for."]]
page = r'''<title>Painted or Flat</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=Pixelify+Sans:wght@500;600&display=swap">
<style>
/* One question, four pictures. Same dark viewing room as the format lab, on purpose. */
:root {
  color-scheme: dark;
  --room: #0f1117; --panel: #171a23; --raised: #1f2330; --line: #2c3140;
  --text: #e8e4da; --dim: #9b9aa5; --lamp: #ffb357; --cold: #86d3e6;
  --display: "Pixelify Sans", "Trebuchet MS", system-ui, sans-serif;
  --body: Inter, system-ui, -apple-system, "Segoe UI", sans-serif;
}
body { background: var(--room); color: var(--text); font: 15px/1.5 var(--body); }
.wrap { max-width: 900px; margin: 0 auto; padding-block: 28px 56px; padding-left: max(16px, env(safe-area-inset-left, 0px)); padding-right: max(16px, env(safe-area-inset-right, 0px)); display: grid; gap: 18px; }
h1 { font: 600 30px/1.1 var(--display); margin: 0; text-wrap: balance; } h1 em { font-style: normal; color: var(--lamp); }
h2 { font: 600 17px/1.25 var(--body); margin: 0; }
p { margin: 0; color: var(--dim); max-width: 68ch; } b { color: var(--text); font-weight: 600; }
header { display: grid; gap: 8px; }
button { font: 500 13px/1 var(--body); cursor: pointer; }
button:focus-visible { outline: 2px solid var(--cold); outline-offset: 2px; }
.controls { display: grid; gap: 10px; }
#sprites { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; max-width: 520px; }
#sprites button { display: grid; justify-items: center; gap: 4px; padding: 6px 4px 8px; background: var(--panel); border: 1px solid var(--line); color: var(--dim); font-size: 12px; min-width: 0; }
#sprites img { width: 100%; max-width: 88px; aspect-ratio: 1; object-fit: cover; image-rendering: pixelated; }
#sprites button[aria-pressed="true"] { border-color: var(--lamp); color: var(--text); background: var(--raised); }
#lights { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); border: 1px solid var(--line); max-width: 520px; }
#lights button { color: var(--dim); background: var(--panel); border: 0; padding: 10px 4px; min-width: 0; }
#lights button + button { border-left: 1px solid var(--line); }
#lights button[aria-pressed="true"] { background: var(--lamp); color: var(--room); }
.four { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px 12px; }
figure { margin: 0; display: grid; gap: 8px; min-width: 0; align-content: start; }
figure button { padding: 0; border: 1px solid var(--line); background: #000; line-height: 0; }
figure.now button { border-color: var(--lamp); }
figure img { width: 100%; aspect-ratio: 132 / 116; image-rendering: pixelated; display: block; }
figcaption { display: grid; gap: 2px; } figcaption b { font-size: 15px; } figcaption span { color: var(--dim); font-size: 13px; }
.notes { background: var(--panel); border: 1px solid var(--line); padding: 14px 16px; display: grid; gap: 6px; }
.notes p { font-size: 14px; } #lightnote { color: var(--lamp); }
.score { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
.score div { border: 1px solid var(--line); padding: 14px 16px; display: grid; gap: 2px; }
.score strong { font: 600 30px/1.1 var(--body); color: var(--text); font-variant-numeric: tabular-nums; }
.score div.now strong { color: var(--lamp); }
.score span { color: var(--dim); font-size: 13px; }
ul { margin: 0; padding-left: 18px; display: grid; gap: 8px; color: var(--dim); max-width: 68ch; }
@media (max-width: 520px) { h1 { font-size: 25px; } }
</style>
<div class="wrap">
  <header>
    <h1>Can <em>light alone</em> redraw the art?</h1>
    <p>If the facing data is right, a sprite should not need its painted shading. Start every surface from one flat color, put a lamp where the artist imagined the light, and the artist’s shading should come back. With the old facing data it did not. It does now.</p>
  </header>
  <div class="controls">
    <div id="sprites"></div>
    <div id="lights"></div>
  </div>
  <div class="four">
    <figure><button type="button" class="next" aria-label="Next light"><img id="img-art" alt="The sprite as drawn, with no lighting"></button>
      <figcaption><b>The art as drawn</b><span>No lighting. What the file’s colors say.</span></figcaption></figure>
    <figure><button type="button" class="next" aria-label="Next light"><img id="img-painted" alt="The sprite lit, starting from its painted colors"></button>
      <figcaption><b>Painted colors, lit</b><span>Start from the painted shade and let the light shift it.</span></figcaption></figure>
    <figure class="now"><button type="button" class="next" aria-label="Next light"><img id="img-flat" alt="The sprite lit from flat color with the refit facing data"></button>
      <figcaption><b>Light alone</b><span>Start from one flat color per surface. Facing data refit to the art.</span></figcaption></figure>
    <figure><button type="button" class="next" aria-label="Next light"><img id="img-old" alt="The sprite lit from flat color with the old facing data"></button>
      <figcaption><b>Light alone, before the fix</b><span>The same, with the old balloon-shaped facing data.</span></figcaption></figure>
  </div>
  <div class="notes"><p id="lightnote"></p><p id="spritenote"></p></div>
  <h2>Measured on all eleven test sprites</h2>
  <p>Lit from exactly the upper left, how many shaded pixels land on the shade the artist painted:</p>
  <div class="score">
    <div><strong>24%</strong><span>before, from the balloon shape alone</span></div>
    <div class="now"><strong>100%</strong><span>after the refit</span></div>
    <div><strong>98%</strong><span>after, once stored in the file’s 8 bits per number</span></div>
  </div>
  <h2>What this means</h2>
  <ul>
    <li><b>There is no painted-or-flat choice to make.</b> The file keeps the art exactly as painted, and the facing data now carries the same shading, so both lit versions give nearly the same picture.</li>
    <li><b>What changed.</b> The balloon shape is kept, but every pixel is turned toward or away from the upper left, just far enough that light from there picks the shade the artist painted.</li>
    <li><b>What is still wrong.</b> A marking painted as a lighter shade of the same surface is read as a bump. It needs its own material, which is a job for a person or a model.</li>
  </ul>
</div>
<script>
const IMG = /*IMG*/, SPRITES = /*SPRITES*/, LIGHTS = /*LIGHTS*/;
const $ = (s) => document.querySelector(s), state = { sprite: 0, light: 0 };
function show(){
  const [id, , note] = SPRITES[state.sprite], [l, , lnote] = LIGHTS[state.light];
  $('#img-art').src = IMG[`${id}-painted-off`]; $('#img-painted').src = IMG[`${id}-painted-${l}`]; $('#img-flat').src = IMG[`${id}-flat-${l}`]; $('#img-old').src = IMG[`old-${id}-flat-${l}`];
  $('#lightnote').textContent = lnote; $('#spritenote').textContent = note;
  [...$('#sprites').children].forEach((b, i) => b.setAttribute('aria-pressed', String(i === state.sprite)));
  [...$('#lights').children].forEach((b, i) => b.setAttribute('aria-pressed', String(i === state.light)));
}
SPRITES.forEach(([id, name], i) => { const b = document.createElement('button'); b.type = 'button'; b.id = 'sprite-' + id;
  b.innerHTML = `<img alt="" src="${IMG[id + '-painted-off']}"><span>${name}</span>`; b.onclick = () => { state.sprite = i; show(); }; $('#sprites').appendChild(b); });
LIGHTS.forEach(([id, name], i) => { const b = document.createElement('button'); b.type = 'button'; b.id = 'light-' + id; b.textContent = name; b.onclick = () => { state.light = i; show(); }; $('#lights').appendChild(b); });
document.querySelectorAll('.next').forEach((b) => { b.onclick = () => { state.light = (state.light + 1) % LIGHTS.length; show(); }; });
show();
</script>
'''
page = page.replace("/*IMG*/", json.dumps(IMG)).replace("/*SPRITES*/", json.dumps(SPRITES)).replace("/*LIGHTS*/", json.dumps(LIGHTS))
open(os.path.join(here, "out/painted-or-flat.html"), "w").write(page)
print("out/painted-or-flat.html", len(page) // 1024, "KB")
