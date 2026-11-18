"""Lay out the frames video.cjs captured, with captions and overlays, and encode ../docs/spu-tour.mp4.

Each frame is one image pixel per sprite pixel (192 x 128), shown eight times larger: every sprite
pixel is then exactly one 8 x 8 block of the video, which is what keeps the pixels crisp once it is
compressed. Under the picture go the chapter and one caption line. On top go the things a viewer
needs to see: the palette, a legend for facing and for materials, a material's two settings, and
labels on side-by-side comparisons."""
import json, os, subprocess, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
here = os.path.dirname(os.path.abspath(__file__)); src = os.path.join(here, "out", "video")
data = json.load(open(os.path.join(src, "frames.json"))); fps = data["fps"]; frames = data["frames"]; scenes = data["scenes"]
S, W, H, BAR = 8, 192, 128, 128
U = S / 5                                                   # overlays were laid out for five times; scale them with the picture
u = lambda v: int(round(v * U))
BG, BARBG, LINE, TEXT, DIM, LAMP, PANEL = (15, 17, 23), (20, 23, 31), (44, 49, 64), (236, 233, 226), (166, 166, 178), (255, 179, 87), (15, 17, 23, 222)
fonts = {}
def font(size, weight="Regular"):
    """The system font at a weight, or Helvetica Neue where it is missing."""
    if (size, weight) not in fonts:
        try:
            f = ImageFont.truetype("/System/Library/Fonts/SFNS.ttf", size); f.set_variation_by_name(weight)
        except (OSError, ValueError):
            try: f = ImageFont.truetype("/System/Library/Fonts/HelveticaNeue.ttc", size, index=0 if weight == "Regular" else 1)
            except OSError: f = ImageFont.load_default()
        fonts[size, weight] = f
    return fonts[size, weight]
HEAD, BODY, TINY, LABEL = font(u(14), "Semibold"), font(u(13.5)), font(u(11)), font(u(15), "Medium")
def native(name):
    im = np.asarray(Image.open(os.path.join(src, name)).convert("RGB")); k = im.shape[1] // W
    return im[k // 2::k, k // 2::k][:H, :W]

def sphere(size):
    """A ball colored the way the facing view colors a surface: red is left to right, green is bottom to top."""
    y, x = np.mgrid[0:size, 0:size]; nx = (x + .5) / size * 2 - 1; ny = 1 - (y + .5) / size * 2; r2 = nx * nx + ny * ny
    nz = np.sqrt(np.clip(1 - r2, 0, 1)); rgb = np.stack([nx * .5 + .5, ny * .5 + .5, nz * .5 + .5], -1) * 255
    return Image.fromarray(np.dstack([rgb, np.where(r2 <= 1, 255, 0)]).astype(np.uint8), "RGBA")
BALL = sphere(u(52))
def chip(i):
    """The color the material view gives row i."""
    if i == 0: return (20, 20, 20)
    h = (i * 0.618 + 0.05) % 1; hue = [min(max(abs(((h + o) % 1) * 6 - 3) - 1, 0), 1) for o in (0, 0.667, 0.333)]
    return tuple(int((0.25 * 0.25 + c * 0.75) * 255) for c in hue)
def panel(im, x, y, w, h):
    layer = Image.new("RGBA", im.size, (0, 0, 0, 0)); ImageDraw.Draw(layer).rectangle([u(x), u(y), u(x + w), u(y + h)], fill=PANEL); im.alpha_composite(layer); return ImageDraw.Draw(im)
def bar(d, x, y, w, stops, value):
    """One setting as a short line with a tick at each stop and a marker where this material sits. value runs from -1 to 1."""
    d.line([u(x), u(y), u(x + w), u(y)], fill=(96, 98, 112), width=max(2, u(1.2)))
    for at in stops: d.line([u(x + w * (at + 1) / 2), u(y - 3), u(x + w * (at + 1) / 2), u(y + 3)], fill=(96, 98, 112), width=max(2, u(1.2)))
    px = x + w * (value + 1) / 2; d.ellipse([u(px - 4.5), u(y - 4.5), u(px + 4.5), u(y + 4.5)], fill=LAMP)
FINISH, BODYSTOPS = [("fuzzy", -1), ("matte", 0), ("polished", 1)], [("metal", -1), ("solid", 0), ("translucent", 0.5), ("clear", 1)]
def stops(d, x, y, w, named):
    for label, at in named: d.text((u(x + w * (at + 1) / 2), u(y)), label, font=TINY, fill=DIM, anchor="ma" if -1 < at < 1 else ("la" if at < 0 else "ra"))
def pill(im, x, y, text, f=None, fill=TEXT):
    """A short label on a dark strip, sized to its text."""
    f = f or LABEL; w = f.getlength(text) / U + 16
    panel(im, x, y, w, 26).text((u(x + 8), u(y + 13)), text, font=f, fill=fill, anchor="lm"); return w

def overlay(im, sc):
    ov, info = sc.get("ov"), sc["info"]
    if sc.get("labels"):
        for k, label in enumerate(sc["labels"]): pill(im, k * W * 5 // 2 + 10, 10, label)
    if not ov: return
    if ov["kind"] == "palette":                                          # one strip: the name, then every color with its number
        cols = info["palette"]; sw = 22; title = "Palette" + (": " + ov["name"] if ov.get("name") else ""); tw = HEAD.getlength(title) / U + 12
        d = panel(im, 10, 10, 8 + tw + len(cols) * (sw + 2) + 6, sw + 12); d.text((u(18), u(16 + sw / 2)), title, font=HEAD, fill=TEXT, anchor="lm")
        for i, c in enumerate(cols):
            x = 18 + tw + i * (sw + 2); d.rectangle([u(x), u(16), u(x + sw) - 1, u(16 + sw) - 1], fill=tuple(c))
            d.text((u(x + sw / 2), u(16 + sw / 2)), str(i + 1), font=TINY, fill=(15, 17, 23) if sum(c) > 380 else (236, 233, 226), anchor="mm")
    elif ov["kind"] == "facing":
        x, y = (W * 5 - 10 - 190, H * 5 - 10 - 68) if ov.get("at") == "corner" else (10, 10)      # out of the way of the sheet
        d = panel(im, x, y, 190, 68); im.alpha_composite(BALL, (u(x + 8), u(y + 8))); d = ImageDraw.Draw(im)
        for k, line in enumerate(["green: faces up", "red: faces right", "blue: faces you"]): d.text((u(x + 70), u(y + 7 + k * 19)), line, font=BODY, fill=TEXT)
    elif ov["kind"] == "materials":                                      # a table: each material and where its two settings sit
        rows = info["rows"]; nw = max(BODY.getlength(r["name"]) for r in rows) / U; x1 = 18 + 14 + 6; x2 = x1 + nw + 14; bw = 84; x3 = x2 + bw + 14
        d = panel(im, 10, 10, x3 + bw + 8 - 10, 30 + len(rows) * 20 + 22)
        d.text((u(x1), u(16)), "Material", font=HEAD, fill=TEXT); d.text((u(x2), u(16)), "Finish", font=HEAD, fill=TEXT); d.text((u(x3), u(16)), "Body", font=HEAD, fill=TEXT)
        for i, r in enumerate(rows):
            y = 37 + i * 20; d.rectangle([u(18), u(y), u(32), u(y + 14)], fill=chip(i), outline=(96, 98, 112))
            d.text((u(x1), u(y + 7)), r["name"], font=BODY, fill=TEXT, anchor="lm")
            if r["words"] == "line art": d.text((u(x2), u(y + 7)), "line art: takes light flat", font=TINY, fill=DIM, anchor="lm")
            else: bar(d, x2, y + 7, bw, [-1, 0, 1], r["f"]); bar(d, x3, y + 7, bw, [-1, 0, 0.5, 1], r["b"])
        y = 37 + len(rows) * 20 + 2; stops(d, x2, y, bw, [("fuzzy", -1), ("polished", 1)]); stops(d, x3, y, bw, [("metal", -1), ("clear", 1)])
    elif ov["kind"] == "dials":                                          # one material: its name and its two settings
        row = next(r for r in info["rows"] if r["name"] == ov["row"]); bw = 250
        d = panel(im, 10, 10, 8 + 56 + bw + 12, 100); d.text((u(18), u(16)), "“" + row["name"] + "”", font=HEAD, fill=LAMP)
        for k, (name, named, value) in enumerate([("Finish", FINISH, row["f"]), ("Body", BODYSTOPS, row["b"])]):
            y = 46 + k * 33; d.text((u(18), u(y)), name, font=BODY, fill=TEXT, anchor="lm"); bar(d, 74, y, bw, [at for _, at in named], value); stops(d, 74, y + 6, bw, named)

def spaced(d, xy, text, f, fill, gap):
    """Text with a little space between letters, for the chapter name."""
    x, y = xy
    for ch in text: d.text((x, y), ch, font=f, fill=fill); x += f.getlength(ch) + gap
captions = {}
def caption(sc):
    """The strip under the picture: the chapter in small capitals, then one sentence, sized to fit."""
    key = (sc["chapter"], sc["cap"])
    if key not in captions:
        im = Image.new("RGB", (W * S, BAR), BARBG); d = ImageDraw.Draw(im); d.line([0, 0, W * S, 0], fill=LINE, width=2)
        size = 36
        while font(size).getlength(sc["cap"]) > W * S - 96: size -= 1
        if sc["chapter"]:
            spaced(d, (48, 26), sc["chapter"].upper(), font(19, "Semibold"), LAMP, 2.5); d.text((48, 86), sc["cap"], font=font(size), fill=TEXT, anchor="ls")
        else: d.text((W * S // 2, BAR // 2), sc["cap"], font=font(52, "Semibold"), fill=TEXT, anchor="mm")
        captions[key] = np.asarray(im).astype(np.float32)
    return captions[key]
EMPTY = np.asarray(Image.new("RGB", (W * S, BAR), BARBG)).astype(np.float32); EMPTY[:2] = LINE
last, start = None, 0                                       # how long each frame's caption has been up, to fade a new one in
for i, f in enumerate(frames):
    if scenes[f["scene"]]["cap"] != last: start = i
    f["since"] = i - start; last = scenes[f["scene"]]["cap"]

def big(a, scale): return Image.fromarray(np.kron(a, np.ones((scale, scale, 1), np.uint8)))
def sign(im, x, y, text):
    """A plus or equals sign on a dark disc, where two pictures meet."""
    d = ImageDraw.Draw(im); r = u(17); d.ellipse([x - r, y - r, x + r, y + r], fill=BG, outline=LINE, width=2)
    d.text((x, y - u(1.5)), text, font=font(u(24), "Medium"), fill=LAMP, anchor="mm")
def picture(sc, c):
    """One scene's picture, with whatever is drawn on top of it."""
    im = Image.new("RGBA", (W * S, H * S), BG + (255,)); half = S // 2
    if sc.get("sum"):
        # what is stored, small on the left, and what a light makes of it, large on the right
        lw = W // 2 * half                                                # the left column shows the middle half of the stage at half size
        im.paste(big(native(c["flat"])[:, W // 4:W * 3 // 4], half), (0, 0)); im.paste(big(native(c["facing"])[:, W // 4:W * 3 // 4], half), (0, H * half))
        im.paste(big(native(c["a"])[:, (W - (W * S - lw) // S) // 2:][:, :(W * S - lw) // S], S), (lw, 0))
        d = ImageDraw.Draw(im); d.line([0, H * half, lw, H * half], fill=BG, width=4); d.line([lw, 0, lw, H * S], fill=BG, width=4)
        sign(im, lw // 2, H * half, "+"); sign(im, lw, H * S // 2, "=")
        pill(im, 6, 6, "Stored: flat color", HEAD); pill(im, 6, H * half / U + 6, "Stored: facing", HEAD); pill(im, lw / U + 10, 10, "Worked out by a light: shading")
    elif sc.get("grid"):
        # the same sprite in six places: three across, two down, each the middle of the stage at half size
        side = H * half; d = ImageDraw.Draw(im)
        for k, name in enumerate(c["cells"]): im.paste(big(native(name)[:, (W - H) // 2:(W + H) // 2], half), (k % 3 * side, k // 3 * side))
        for k in (1, 2): d.line([k * side, 0, k * side, H * S], fill=BG, width=4)
        d.line([0, side, W * S, side], fill=BG, width=4)
        for k, label in enumerate(sc["grid"]): pill(im, k % 3 * side / U + 8, k // 3 * side / U + 8, label, HEAD)
    else:
        a = native(c["a"])
        if c.get("c"):                                           # a comparison: the middle of each, side by side
            a = np.concatenate([a[:, W // 4:W * 3 // 4], native(c["c"])[:, W // 4:W * 3 // 4]], 1).copy(); a[:, W // 2] = BG
        im.paste(big(a, S), (0, 0)); overlay(im, sc)
    return np.asarray(im.convert("RGB"))
def compose(f):
    sc = scenes[f["scene"]]; pic = picture(sc, f["caps"])
    if f.get("prev"):                                            # the new scene sweeps in from the left behind a line
        x = round(W * f["wipe"]) * S; old = picture(scenes[f["scene"] - 1], f["prev"]).copy(); old[:, :x] = pic[:, :x]
        if 0 < x < W * S: old[:, x:x + S // 2] = LAMP
        pic = old
    k = min(1.0, (f["since"] + 1) / 8)                       # a new caption fades in over a quarter of a second
    return np.concatenate([pic, (EMPTY * (1 - k) + caption(sc) * k).astype(np.uint8)], 0)

out = os.path.join(here, "..", "docs", "spu-tour.mp4"); size = f"{W * S}x{H * S + BAR}"
if "--poster-only" not in sys.argv:
    enc = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", size, "-r", str(fps), "-i", "-",
                            "-c:v", "libx264", "-preset", "slow", "-tune", "animation", "-crf", os.environ.get("CRF", "24"), "-pix_fmt", "yuv420p", "-movflags", "+faststart", out], stdin=subprocess.PIPE)
    for i, f in enumerate(frames):
        frame = compose(f); enc.stdin.write(frame.tobytes())
        if str(i) in sys.argv[1:]: Image.fromarray(frame).save(os.path.join(src, f"still-{i}.png"))
    enc.stdin.close(); enc.wait()
# the poster: the sprite's pieces on their sheet, with a play mark in the corner
shown = [f for f in frames if scenes[f["scene"]]["chapter"] == "Pieces"][-1]
poster = Image.fromarray(compose(shown)).convert("RGBA")
mark = Image.new("RGBA", poster.size, (0, 0, 0, 0)); d = ImageDraw.Draw(mark); r = u(46); cx, cy = W * S - r - u(28), H * S - r - u(28)
d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(15, 17, 23, 215)); d.polygon([(cx - u(13), cy - u(22)), (cx - u(13), cy + u(22)), (cx + u(24), cy)], fill=LAMP + (255,))
Image.alpha_composite(poster, mark).convert("RGB").save(os.path.join(here, "..", "docs", "images", "tour-poster.png"), optimize=True)
print(out, os.path.getsize(out) // 1024, "KB,", len(frames), "frames,", round(len(frames) / fps, 1), "s,", size)
