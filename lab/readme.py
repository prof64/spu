"""Lay out the README pictures from out/readme-shots.json (made by readme.cjs) into ../docs/images/."""
import json, base64, io, os, shutil
import numpy as np
from PIL import Image
here = os.path.dirname(os.path.abspath(__file__)); dst = os.path.join(here, "..", "docs", "images"); os.makedirs(dst, exist_ok=True)
shots = json.load(open(os.path.join(here, "out/readme-shots.json")))
BG = (15, 17, 23)
def native(u):
    """A capture back at one image pixel per sprite pixel (192 x 128)."""
    im = np.array(Image.open(io.BytesIO(base64.b64decode(u.split(",")[1]))).convert("RGB")); k = im.shape[1] // 192
    return im[k // 2::k, k // 2::k][:128, :192]
def grid(name, tiles, cols, crop, scale=2, gap=6):
    x0, x1, y0, y1 = crop; t = [np.kron(native(u)[y0:y1, x0:x1], np.ones((scale, scale, 1), np.uint8)) for u in tiles]
    h, w = t[0].shape[:2]; rows = -(-len(t) // cols)
    out = Image.new("RGB", (cols * w + (cols - 1) * gap, rows * h + (rows - 1) * gap), BG)
    for i, a in enumerate(t): out.paste(Image.fromarray(a), ((i % cols) * (w + gap), (i // cols) * (h + gap)))
    out.save(os.path.join(dst, name + ".png"), optimize=True); print(name, out.size)
def loop(name, frames, crop, scale=3, colors=160):
    x0, x1, y0, y1 = crop; fr = [native(u)[y0:y1, x0:x1] for u in frames]
    pal = Image.fromarray(np.concatenate(fr[::4], 1)).quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)   # one palette for every frame
    ims = [Image.fromarray(np.kron(a, np.ones((scale, scale, 1), np.uint8))).quantize(palette=pal, dither=Image.Dither.NONE) for a in fr]
    path = os.path.join(dst, name + ".gif"); ims[0].save(path, save_all=True, append_images=ims[1:], duration=83, loop=0, optimize=False)
    print(name, ims[0].size, len(ims), "frames", os.path.getsize(path) // 1024, "KB")
BODY = (40, 152, 16, 116)
loop("hero", shots["hero"], (0, 192, 0, 128), colors=112)
loop("gears", shots["gears"], (46, 146, 28, 120))
grid("places", shots["places"], 3, (24, 168, 12, 120))
grid("pixel", shots["pixel"], 4, BODY)
grid("light", shots["light"], 4, BODY)
grid("light-alone", shots["alone"], 4, BODY)
grid("palettes", shots["palettes"], 4, (52, 140, 26, 116))
grid("looks", shots["looks"], 4, (46, 146, 18, 114))
grid("parts", shots["parts"], 3, (21, 171, 2, 126))
grid("views", shots["views"], 4, BODY)
grid("soft", shots["soft"], 4, (44, 148, 20, 116))
# the sheets exactly as stored, side by side, three screen pixels per sheet pixel
sh = [Image.open(io.BytesIO(base64.b64decode(u.split(",")[1]))).convert("RGBA") for u in shots["sheets"]]
w, h = sh[0].size; out = Image.new("RGB", (w * 2 * 3 + 12, h * 2), BG)
for i, im in enumerate(sh):
    bg = Image.new("RGBA", im.size, (0, 0, 0, 255)); bg.alpha_composite(im); out.paste(bg.convert("RGB").resize((w * 2, h * 2), Image.NEAREST), (i * (w * 2 + 6), 0))
out.save(os.path.join(dst, "sheets.png"), optimize=True); print("sheets", out.size)
shutil.copy(os.path.join(here, "out/readme-map.png"), os.path.join(dst, "materials.png")); print("materials", Image.open(os.path.join(dst, "materials.png")).size)
