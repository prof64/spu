"""Lay out the frames video.cjs captured, with captions and overlays, and encode ../docs/spu-tour.mp4.

Each frame is one image pixel per sprite pixel (192 x 128), shown five times larger, with one caption
line underneath. On top go the things a viewer needs to see: the palette, a legend for facing and for
materials, a material's two settings, and labels on side-by-side comparisons."""
import json, os, subprocess, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
here = os.path.dirname(os.path.abspath(__file__)); src = os.path.join(here, "out", "video")
data = json.load(open(os.path.join(src, "frames.json"))); fps = data["fps"]; frames = data["frames"]; scenes = data["scenes"]
S, W, H, BAR = 5, 192, 128, 64
BG, TEXT, DIM, LAMP, PANEL = (15, 17, 23), (232, 228, 218), (170, 169, 178), (255, 179, 87), (15, 17, 23, 215)
def font(size, bold=False):
    for path in ["/System/Library/Fonts/HelveticaNeue.ttc", "/System/Library/Fonts/Helvetica.ttc"]:
        try: return ImageFont.truetype(path, size, index=1 if bold else 0)
        except OSError: pass
    return ImageFont.load_default()
CAP, HEAD, BODY, TINY = font(23), font(16, True), font(15), font(13)
def native(name):
    im = np.asarray(Image.open(os.path.join(src, name)).convert("RGB")); k = im.shape[1] // W
    return im[k // 2::k, k // 2::k][:H, :W]

def sphere(size):
    """A ball colored the way the facing view colors a surface: red is left to right, green is bottom to top."""
    y, x = np.mgrid[0:size, 0:size]; nx = (x + .5) / size * 2 - 1; ny = 1 - (y + .5) / size * 2; r2 = nx * nx + ny * ny
    nz = np.sqrt(np.clip(1 - r2, 0, 1)); rgb = np.stack([nx * .5 + .5, ny * .5 + .5, nz * .5 + .5], -1) * 255
    return Image.fromarray(np.dstack([rgb, np.where(r2 <= 1, 255, 0)]).astype(np.uint8), "RGBA")
BALL = sphere(96)
def chip(i):
    """The color the material view gives row i."""
    if i == 0: return (20, 20, 20)
    h = (i * 0.618 + 0.05) % 1; hue = [min(max(abs(((h + o) % 1) * 6 - 3) - 1, 0), 1) for o in (0, 0.667, 0.333)]
    return tuple(int((0.25 * 0.25 + c * 0.75) * 255) for c in hue)
def panel(im, box):
    layer = Image.new("RGBA", im.size, (0, 0, 0, 0)); ImageDraw.Draw(layer).rectangle(box, fill=PANEL); im.alpha_composite(layer); return ImageDraw.Draw(im)
def dial(d, x, y, w, name, stops, value):
    """One setting as a line with its stops named and a marker where this material sits. value runs from -1 to 1."""
    d.text((x, y), name, font=HEAD, fill=TEXT); y += 30
    d.line([x, y, x + w, y], fill=DIM, width=2)
    for label, at in stops:
        px = x + w * (at + 1) / 2; d.line([px, y - 4, px, y + 4], fill=DIM, width=2)
        d.text((px, y + 9), label, font=TINY, fill=DIM, anchor="ma" if 0 < (at + 1) / 2 < 1 else ("la" if at < 0 else "ra"))
    px = x + w * (value + 1) / 2; d.ellipse([px - 7, y - 7, px + 7, y + 7], fill=LAMP)

def overlay(im, sc):
    ov, info = sc.get("ov"), sc["info"]
    if sc.get("labels"):
        for k, label in enumerate(sc["labels"]):
            w = CAP.getlength(label) + 28; x = k * W * S // 2 + 16
            panel(im, [x, 16, x + w, 56]).text((x + 14, 24), label, font=CAP, fill=TEXT)
    if not ov: return
    if ov["kind"] == "palette":
        cols = info["palette"]; sw = 30; w = len(cols) * (sw + 4) + 24
        d = panel(im, [16, 16, 16 + w, 16 + 96]); d.text((28, 26), "Palette" + (": " + ov["name"] if ov.get("name") else ""), font=HEAD, fill=TEXT)
        for i, c in enumerate(cols):
            x = 28 + i * (sw + 4); d.rectangle([x, 54, x + sw - 1, 54 + sw - 1], fill=tuple(c)); d.text((x + sw / 2, 88), str(i + 1), font=TINY, fill=DIM, anchor="ma")
    elif ov["kind"] == "facing":
        d = panel(im, [16, 16, 16 + 250, 16 + 150]); im.alpha_composite(BALL, (40, 48)); d = ImageDraw.Draw(im)
        d.text((28, 24), "Facing", font=HEAD, fill=TEXT)
        d.text((152, 58), "green: faces up", font=BODY, fill=DIM); d.text((152, 84), "red: faces right", font=BODY, fill=DIM); d.text((152, 110), "blue: faces you", font=BODY, fill=DIM)
    elif ov["kind"] == "shades":
        row = next(r for r in info["rows"] if r["name"] == ov["row"]); sw = 44; x0 = W * S // 2 + 16; y0 = H * S - 16 - 104
        d = panel(im, [x0, y0, x0 + len(row["ramp"]) * (sw + 4) + 24, y0 + 104]); d.text((x0 + 12, y0 + 10), "The shades of “" + row["name"] + "”", font=HEAD, fill=TEXT)
        for i, c in enumerate(row["ramp"]): d.rectangle([x0 + 12 + i * (sw + 4), y0 + 40, x0 + 12 + i * (sw + 4) + sw - 1, y0 + 40 + 30], fill=tuple(c))
        d.text((x0 + 12, y0 + 78), "dark", font=TINY, fill=DIM); d.text((x0 + 12 + len(row["ramp"]) * (sw + 4) - 4, y0 + 78), "light", font=TINY, fill=DIM, anchor="ra")
    elif ov["kind"] == "materials":
        rows = info["rows"]; d = panel(im, [16, 16, 16 + 330, 16 + 44 + len(rows) * 30]); d.text((28, 26), "Materials", font=HEAD, fill=TEXT)
        for i, r in enumerate(rows):
            y = 56 + i * 30; d.rectangle([28, y, 28 + 20, y + 20], fill=chip(i), outline=DIM)
            d.text((58, y + 1), r["name"], font=BODY, fill=TEXT); d.text((334, y + 1), r["words"], font=BODY, fill=DIM, anchor="ra")
    elif ov["kind"] == "dials":
        row = next(r for r in info["rows"] if r["name"] == ov["row"])
        d = panel(im, [16, 16, 16 + 320, 16 + 182]); d.text((28, 24), "“" + row["name"] + "”", font=HEAD, fill=LAMP)
        dial(d, 28, 52, 296, "Finish", [("fuzzy", -1), ("matte", 0), ("polished", 1)], row["f"])
        dial(d, 28, 116, 296, "Body", [("metal", -1), ("solid", 0), ("translucent", 0.5), ("clear", 1)], row["b"])

def compose(f):
    sc = scenes[f["scene"]]; a = native(f["a"])
    if f.get("c"):                                           # a comparison: the middle of each, side by side
        a = np.concatenate([a[:, W // 4:W * 3 // 4], native(f["c"])[:, W // 4:W * 3 // 4]], 1).copy(); a[:, W // 2] = BG
    if f.get("b"):                                           # the new scene sweeps in from the left behind a line
        x = round(W * f["wipe"]); out = native(f["b"]).copy(); out[:, :x] = a[:, :x]
        if 0 < x < W: out[:, x] = LAMP
        a = out
    im = Image.new("RGBA", (W * S, H * S + BAR), BG + (255,)); im.paste(Image.fromarray(np.kron(a, np.ones((S, S, 1), np.uint8))), (0, 0))
    if not f.get("b"): overlay(im, sc)
    ImageDraw.Draw(im).text((W * S // 2, H * S + BAR // 2), sc["cap"], font=CAP, fill=TEXT, anchor="mm")
    return np.asarray(im.convert("RGB"))

out = os.path.join(here, "..", "docs", "spu-tour.mp4"); size = f"{W * S}x{H * S + BAR}"
if "--poster-only" not in sys.argv:
    enc = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", size, "-r", str(fps), "-i", "-",
                            "-c:v", "libx264", "-preset", "slow", "-tune", "animation", "-crf", os.environ.get("CRF", "25"), "-pix_fmt", "yuv420p", "-movflags", "+faststart", out], stdin=subprocess.PIPE)
    for i, f in enumerate(frames):
        frame = compose(f); enc.stdin.write(frame.tobytes())
        if str(i) in sys.argv[1:]: Image.fromarray(frame).save(os.path.join(src, f"still-{i}.png"))
    enc.stdin.close(); enc.wait()
# the poster: the sprite pulled into its parts, with a play mark in the corner
shown = [f for f in frames if scenes[f["scene"]]["cap"].startswith("The sprite is made of parts")][-1]
poster = Image.fromarray(compose(shown)).convert("RGBA")
mark = Image.new("RGBA", poster.size, (0, 0, 0, 0)); d = ImageDraw.Draw(mark); r = 46; cx, cy = W * S - r - 28, H * S - r - 28
d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(15, 17, 23, 215)); d.polygon([(cx - 13, cy - 22), (cx - 13, cy + 22), (cx + 24, cy)], fill=LAMP + (255,))
Image.alpha_composite(poster, mark).convert("RGB").save(os.path.join(here, "..", "docs", "images", "tour-poster.png"), optimize=True)
print(out, os.path.getsize(out) // 1024, "KB,", len(frames), "frames,", round(len(frames) / fps, 1), "s,", size)
print("captions wider than the frame:", [sc["cap"] for sc in scenes if CAP.getlength(sc["cap"]) > W * S - 40])
