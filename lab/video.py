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
HEAD, BODY, TINY, LABEL = font(u(16), "Semibold"), font(u(15)), font(u(12.5)), font(u(19), "Medium")
def native(name):
    im = np.asarray(Image.open(os.path.join(src, name)).convert("RGB")); k = im.shape[1] // W
    return im[k // 2::k, k // 2::k][:H, :W]

def sphere(size):
    """A ball colored the way the facing view colors a surface: red is left to right, green is bottom to top."""
    y, x = np.mgrid[0:size, 0:size]; nx = (x + .5) / size * 2 - 1; ny = 1 - (y + .5) / size * 2; r2 = nx * nx + ny * ny
    nz = np.sqrt(np.clip(1 - r2, 0, 1)); rgb = np.stack([nx * .5 + .5, ny * .5 + .5, nz * .5 + .5], -1) * 255
    return Image.fromarray(np.dstack([rgb, np.where(r2 <= 1, 255, 0)]).astype(np.uint8), "RGBA")
BALL = sphere(u(96))
def chip(i):
    """The color the material view gives row i."""
    if i == 0: return (20, 20, 20)
    h = (i * 0.618 + 0.05) % 1; hue = [min(max(abs(((h + o) % 1) * 6 - 3) - 1, 0), 1) for o in (0, 0.667, 0.333)]
    return tuple(int((0.25 * 0.25 + c * 0.75) * 255) for c in hue)
def panel(im, x, y, w, h):
    layer = Image.new("RGBA", im.size, (0, 0, 0, 0)); ImageDraw.Draw(layer).rectangle([u(x), u(y), u(x + w), u(y + h)], fill=PANEL); im.alpha_composite(layer); return ImageDraw.Draw(im)
def dial(d, x, y, w, name, stops, value):
    """One setting as a line with its stops named and a marker where this material sits. value runs from -1 to 1."""
    d.text((u(x), u(y)), name, font=HEAD, fill=TEXT); y += 30
    d.line([u(x), u(y), u(x + w), u(y)], fill=DIM, width=u(2))
    for label, at in stops:
        px = x + w * (at + 1) / 2; d.line([u(px), u(y - 4), u(px), u(y + 4)], fill=DIM, width=u(2))
        d.text((u(px), u(y + 9)), label, font=TINY, fill=DIM, anchor="ma" if 0 < (at + 1) / 2 < 1 else ("la" if at < 0 else "ra"))
    px = x + w * (value + 1) / 2; d.ellipse([u(px - 7), u(y - 7), u(px + 7), u(y + 7)], fill=LAMP)

def overlay(im, sc):
    ov, info = sc.get("ov"), sc["info"]
    if sc.get("labels"):
        for k, label in enumerate(sc["labels"]):
            w = LABEL.getlength(label) / U + 26; x = k * W * 5 // 2 + 14
            panel(im, x, 14, w, 38).text((u(x + 13), u(14 + 19)), label, font=LABEL, fill=TEXT, anchor="lm")
    if not ov: return
    if ov["kind"] == "palette":
        cols = info["palette"]; sw = 30
        d = panel(im, 16, 16, len(cols) * (sw + 4) + 20, 100); d.text((u(28), u(26)), "Palette" + (": " + ov["name"] if ov.get("name") else ""), font=HEAD, fill=TEXT)
        for i, c in enumerate(cols):
            x = 28 + i * (sw + 4); d.rectangle([u(x), u(54), u(x + sw) - 1, u(54 + sw) - 1], fill=tuple(c)); d.text((u(x + sw / 2), u(89)), str(i + 1), font=TINY, fill=DIM, anchor="ma")
    elif ov["kind"] == "facing":
        d = panel(im, 16, 16, 262, 150); im.alpha_composite(BALL, (u(34), u(52))); d = ImageDraw.Draw(im)
        d.text((u(28), u(24)), "Facing", font=HEAD, fill=TEXT)
        for k, line in enumerate(["green: faces up", "red: faces right", "blue: faces you"]): d.text((u(148), u(66 + k * 26)), line, font=BODY, fill=DIM)
    elif ov["kind"] == "shades":
        row = next(r for r in info["rows"] if r["name"] == ov["row"]); sw = 44; x0 = W * 5 // 2 + 14; y0 = H * 5 - 14 - 108; n = len(row["ramp"])
        d = panel(im, x0, y0, n * (sw + 4) + 20, 108); d.text((u(x0 + 12), u(y0 + 10)), "The shades of “" + row["name"] + "”", font=HEAD, fill=TEXT)
        for i, c in enumerate(row["ramp"]): d.rectangle([u(x0 + 12 + i * (sw + 4)), u(y0 + 40), u(x0 + 12 + i * (sw + 4) + sw) - 1, u(y0 + 70) - 1], fill=tuple(c))
        d.text((u(x0 + 12), u(y0 + 79)), "dark", font=TINY, fill=DIM); d.text((u(x0 + 12 + n * (sw + 4) - 4), u(y0 + 79)), "light", font=TINY, fill=DIM, anchor="ra")
    elif ov["kind"] == "materials":
        rows = info["rows"]; d = panel(im, 16, 16, 336, 46 + len(rows) * 30); d.text((u(28), u(26)), "Materials", font=HEAD, fill=TEXT)
        for i, r in enumerate(rows):
            y = 58 + i * 30; d.rectangle([u(28), u(y), u(48), u(y + 20)], fill=chip(i), outline=DIM)
            d.text((u(58), u(y + 10)), r["name"], font=BODY, fill=TEXT, anchor="lm"); d.text((u(340), u(y + 10)), r["words"], font=BODY, fill=DIM, anchor="rm")
    elif ov["kind"] == "dials":
        row = next(r for r in info["rows"] if r["name"] == ov["row"])
        d = panel(im, 16, 16, 320, 184); d.text((u(28), u(24)), "“" + row["name"] + "”", font=HEAD, fill=LAMP)
        dial(d, 28, 54, 296, "Finish", [("fuzzy", -1), ("matte", 0), ("polished", 1)], row["f"])
        dial(d, 28, 118, 296, "Body", [("metal", -1), ("solid", 0), ("translucent", 0.5), ("clear", 1)], row["b"])

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
        else: d.text((W * S // 2, BAR // 2), sc["cap"], font=font(44, "Semibold"), fill=TEXT, anchor="mm")
        captions[key] = np.asarray(im).astype(np.float32)
    return captions[key]
EMPTY = np.asarray(Image.new("RGB", (W * S, BAR), BARBG)).astype(np.float32); EMPTY[:2] = LINE
last, start = None, 0                                       # how long each frame's caption has been up, to fade a new one in
for i, f in enumerate(frames):
    if scenes[f["scene"]]["cap"] != last: start = i
    f["since"] = i - start; last = scenes[f["scene"]]["cap"]

def compose(f):
    sc = scenes[f["scene"]]; a = native(f["a"])
    if f.get("c"):                                           # a comparison: the middle of each, side by side
        a = np.concatenate([a[:, W // 4:W * 3 // 4], native(f["c"])[:, W // 4:W * 3 // 4]], 1).copy(); a[:, W // 2] = BG
    if f.get("b"):                                           # the new scene sweeps in from the left behind a line
        x = round(W * f["wipe"]); out = native(f["b"]).copy(); out[:, :x] = a[:, :x]
        if 0 < x < W: out[:, x] = LAMP
        a = out
    im = Image.new("RGBA", (W * S, H * S), BG + (255,)); im.paste(Image.fromarray(np.kron(a, np.ones((S, S, 1), np.uint8))), (0, 0))
    if not f.get("b"): overlay(im, sc)
    k = min(1.0, (f["since"] + 1) / 8)                       # a new caption fades in over a quarter of a second
    bar = (EMPTY * (1 - k) + caption(sc) * k).astype(np.uint8)
    return np.concatenate([np.asarray(im.convert("RGB")), bar], 0)

out = os.path.join(here, "..", "docs", "spu-tour.mp4"); size = f"{W * S}x{H * S + BAR}"
if "--poster-only" not in sys.argv:
    enc = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", size, "-r", str(fps), "-i", "-",
                            "-c:v", "libx264", "-preset", "slow", "-tune", "animation", "-crf", os.environ.get("CRF", "24"), "-pix_fmt", "yuv420p", "-movflags", "+faststart", out], stdin=subprocess.PIPE)
    for i, f in enumerate(frames):
        frame = compose(f); enc.stdin.write(frame.tobytes())
        if str(i) in sys.argv[1:]: Image.fromarray(frame).save(os.path.join(src, f"still-{i}.png"))
    enc.stdin.close(); enc.wait()
# the poster: the sprite pulled into its parts, with a play mark in the corner
shown = [f for f in frames if scenes[f["scene"]]["cap"].startswith("The sprite is made of parts")][-1]
poster = Image.fromarray(compose(shown)).convert("RGBA")
mark = Image.new("RGBA", poster.size, (0, 0, 0, 0)); d = ImageDraw.Draw(mark); r = u(46); cx, cy = W * S - r - u(28), H * S - r - u(28)
d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(15, 17, 23, 215)); d.polygon([(cx - u(13), cy - u(22)), (cx - u(13), cy + u(22)), (cx + u(24), cy)], fill=LAMP + (255,))
Image.alpha_composite(poster, mark).convert("RGB").save(os.path.join(here, "..", "docs", "images", "tour-poster.png"), optimize=True)
print(out, os.path.getsize(out) // 1024, "KB,", len(frames), "frames,", round(len(frames) / fps, 1), "s,", size)
