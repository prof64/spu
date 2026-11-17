"""Add captions to the frames video.cjs captured and encode ../docs/spu-tour.mp4 (and its poster).

Each frame is one image pixel per sprite pixel (192 x 128), shown five times larger, with a caption
underneath. Where one scene wipes in over another, a bright line sweeps across."""
import json, os, subprocess, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
here = os.path.dirname(os.path.abspath(__file__)); src = os.path.join(here, "out", "video")
data = json.load(open(os.path.join(src, "frames.json"))); fps = data["fps"]; frames = data["frames"]
S, W, H, BAR = 5, 192, 128, 110
BG, TEXT, DIM, LAMP = (15, 17, 23), (232, 228, 218), (155, 154, 165), (255, 179, 87)
def font(size, bold):
    for path, index in [("/System/Library/Fonts/HelveticaNeue.ttc", 1 if bold else 0), ("/System/Library/Fonts/Helvetica.ttc", 1 if bold else 0)]:
        try: return ImageFont.truetype(path, size, index=index)
        except OSError: pass
    return ImageFont.load_default()
BIG, SMALL = font(30, True), font(20, False)
def native(name):
    im = np.asarray(Image.open(os.path.join(src, name)).convert("RGB")); k = im.shape[1] // W
    return im[k // 2::k, k // 2::k][:H, :W]
bars = {}
def bar(cap, sub):
    if (cap, sub) not in bars:
        im = Image.new("RGB", (W * S, BAR), BG); d = ImageDraw.Draw(im)
        d.text((24, 22), cap, font=BIG, fill=TEXT); d.text((24, 66), sub, font=SMALL, fill=DIM)
        bars[cap, sub] = np.asarray(im)
    return bars[cap, sub]
def compose(f):
    a = native(f["a"])
    if f.get("b"):
        x = round(W * f["wipe"]); out = native(f["b"]).copy(); out[:, :x] = a[:, :x]        # the new scene sweeps in from the left
        if 0 < x < W: out[:, x] = LAMP
        a = out
    return np.concatenate([np.kron(a, np.ones((S, S, 1), np.uint8)), bar(f["cap"], f["sub"])], 0)
out = os.path.join(here, "..", "docs", "spu-tour.mp4")
size = f"{W * S}x{H * S + BAR}"
if "--poster-only" not in sys.argv:
    enc = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", size, "-r", str(fps), "-i", "-",
                            "-c:v", "libx264", "-preset", "slow", "-tune", "animation", "-crf", os.environ.get("CRF", "25"), "-pix_fmt", "yuv420p", "-movflags", "+faststart", out], stdin=subprocess.PIPE)
    for i, f in enumerate(frames):
        frame = compose(f); enc.stdin.write(frame.tobytes())
        if str(i) in sys.argv[1:]: Image.fromarray(frame).save(os.path.join(src, f"still-{i}.png"))
    enc.stdin.close(); enc.wait()
# the poster: the sprite pulled into its parts, with a play mark in the corner
shown = [f for f in frames if f["cap"] == "It is built from parts."][-1]
poster = Image.fromarray(compose(shown)).convert("RGBA")
mark = Image.new("RGBA", poster.size, (0, 0, 0, 0)); d = ImageDraw.Draw(mark); r = 46; cx, cy = W * S - r - 28, H * S - r - 28
d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(15, 17, 23, 215)); d.polygon([(cx - 13, cy - 22), (cx - 13, cy + 22), (cx + 24, cy)], fill=LAMP + (255,))
Image.alpha_composite(poster, mark).convert("RGB").save(os.path.join(here, "..", "docs", "images", "tour-poster.png"), optimize=True)
print(out, os.path.getsize(out) // 1024, "KB,", len(frames), "frames,", round(len(frames) / fps, 1), "s,", size)
