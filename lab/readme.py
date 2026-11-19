"""Lay out the README pictures from out/readme-shots.json (made by readme.cjs) into ../docs/images/ (not committed)."""
import json, base64, io, os, shutil
import numpy as np
from PIL import Image, ImageDraw, ImageFont
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
def font(size, bold=False):
    for path in ["/System/Library/Fonts/HelveticaNeue.ttc", "/System/Library/Fonts/Helvetica.ttc"]:
        try: return ImageFont.truetype(path, size, index=1 if bold else 0)
        except OSError: pass
    return ImageFont.load_default()
TEXT, DIM = (232, 228, 218), (155, 154, 165)
def tile(u, crop, scale=2):
    x0, x1, y0, y1 = crop
    return Image.fromarray(np.kron(native(u)[y0:y1, x0:x1], np.ones((scale, scale, 1), np.uint8)))
def labeled(name, rows, crop, gap=8, strip=30):
    """Rows of tiles with a label under each. A row is a list of (capture, label) or a string: a sign drawn between tiles."""
    w, h = tile(rows[0][0][0], crop).size; sign = 44
    width = max(sum(sign if isinstance(c, str) else w for c in row) + gap * (len(row) - 1) for row in rows)
    out = Image.new("RGB", (width, len(rows) * (h + strip) + (len(rows) - 1) * gap), BG); d = ImageDraw.Draw(out)
    for r, row in enumerate(rows):
        x, y = 0, r * (h + strip + gap)
        for c in row:
            if isinstance(c, str):
                d.text((x + sign / 2, y + h / 2), c, font=font(40, True), fill=TEXT, anchor="mm"); x += sign + gap
            else:
                out.paste(tile(c[0], crop), (x, y)); d.text((x + 2, y + h + 7), c[1], font=font(15), fill=DIM); x += w + gap
    out.save(os.path.join(dst, name + ".png"), optimize=True); print(name, out.size)
def loop(name, frames, crop, scale=3, colors=160):
    x0, x1, y0, y1 = crop; fr = [native(u)[y0:y1, x0:x1] for u in frames]
    pal = Image.fromarray(np.concatenate(fr[::4], 1)).quantize(colors=colors, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)   # one palette for every frame
    ims = [Image.fromarray(np.kron(a, np.ones((scale, scale, 1), np.uint8))).quantize(palette=pal, dither=Image.Dither.NONE) for a in fr]
    path = os.path.join(dst, name + ".gif"); ims[0].save(path, save_all=True, append_images=ims[1:], duration=83, loop=0, optimize=False)
    print(name, ims[0].size, len(ims), "frames", os.path.getsize(path) // 1024, "KB")
BODY = (40, 152, 16, 116)
loop("hero", shots["hero"], (0, 192, 0, 128), colors=128)
loop("gears", shots["gears"], (46, 146, 28, 120))
grid("places", shots["places"], 3, (24, 168, 12, 120))
grid("pixel", shots["pixel"], 4, BODY)
grid("light", shots["light"], 4, BODY)
e = shots["equation"]
labeled("equation", [[(e[0], "flat color"), "+", (e[1], "which way each pixel faces"), "=", (e[2], "add a light from the upper left"), (e[3], "the art as the artist drew it")],
                     [(e[4], "the same data, lit from the right"), (e[5], "from below"), (e[6], "from behind, in a cave"), (e[7], "in rain, in a lightning flash")]], BODY)
m = shots["made"]
labeled("made-of", [[(m[0], "Polished metal"), (m[1], "Ice"), (m[2], "Fur")], [(m[3], "Wing membrane, lit from behind"), (m[4], "Glass and flame"), (m[5], "Jelly")]], (24, 168, 12, 120))
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
