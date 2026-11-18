"""Compose the tour at integer sprite-pixel scales, then encode with selectable captions."""
import json, os, subprocess, sys
from itertools import groupby
from mmap import mmap

import numpy as np
from PIL import Image, ImageDraw, ImageFont

from video_numbers import draw_numbers, draw_surfaces, material_color, number_badge

here = os.path.dirname(os.path.abspath(__file__)); src = os.path.join(here, "out", "video")
data = json.load(open(os.path.join(src, "frames.json"))); fps = data["fps"]; frames = data["frames"]; scenes = data["scenes"]
S, W, H = 8, 192, 128
U = S / 5                                                   # overlays were laid out for five times; scale them with the picture
u = lambda v: int(round(v * U))
BG, LINE, TEXT, DIM, LAMP, PANEL = (15, 17, 23), (44, 49, 64), (236, 233, 226), (166, 166, 178), (255, 179, 87), (15, 17, 23, 222)
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
HEAD, BODY, LABEL = font(u(17), "Semibold"), font(u(15)), font(u(18), "Medium")
def native(name):
    im = np.asarray(Image.open(os.path.join(src, name)).convert("RGB")); k = im.shape[1] // W
    return im[k // 2::k, k // 2::k][:H, :W]

def sphere(size):
    """A ball colored the way the facing view colors a surface: red is left to right, green is bottom to top."""
    y, x = np.mgrid[0:size, 0:size]; nx = (x + .5) / size * 2 - 1; ny = 1 - (y + .5) / size * 2; r2 = nx * nx + ny * ny
    nz = np.sqrt(np.clip(1 - r2, 0, 1)); rgb = np.stack([nx * .5 + .5, ny * .5 + .5, nz * .5 + .5], -1) * 255
    return Image.fromarray(np.dstack([rgb, np.where(r2 <= 1, 255, 0)]).astype(np.uint8), "RGBA")
BALL = sphere(u(52))
def panel(im, x, y, w, h):
    left, top = u(x), u(y)
    tile = Image.new("RGBA", (u(x + w) - left, u(y + h) - top), PANEL)
    im.alpha_composite(tile, (left, top))
    return ImageDraw.Draw(im)

def pill(im, x, y, text, f=None, fill=TEXT, centered=False):
    """A short label on a dark strip, sized to its text."""
    f = f or LABEL; w = f.getlength(text) / U + 20
    if centered: x -= w / 2
    panel(im, x, y, w, 32).text((u(x + 10), u(y + 16)), text, font=f, fill=fill, anchor="lm"); return w

def note_card(im, x, y, note, heading=None, material=None):
    lines = note.splitlines()
    w = max(LABEL.getlength(line) for line in lines)
    indent = 23 if material is not None else 0
    if heading:
        w = max(w, HEAD.getlength(heading) + u(indent))
    line_height = 23
    h = 12 + line_height * (len(lines) + bool(heading))
    d = panel(im, x, y, w / U + 22, h)
    ty = y + 6 + line_height / 2
    if heading:
        if material is not None:
            number_badge(d, u, HEAD, x + 20, ty, 19, 19, material_color(material), material)
        d.text((u(x + 11 + indent), u(ty)), heading, font=HEAD, fill=LAMP, anchor="lm")
        ty += line_height
    for line in lines:
        d.text((u(x + 11), u(ty)), line, font=LABEL, fill=TEXT, anchor="lm")
        ty += line_height

def overlay(im, sc, seconds):
    ov = sc.get("ov") or {}
    if sc.get("labels"):
        for k, label in enumerate(sc["labels"]):
            x = k * W * 5 // 2 + 12
            if k == 1 and sc.get("note"):
                note_card(im, x, 12, sc["note"], label, ov.get("material"))
            else:
                pill(im, x, 12, label)
    kind = ov.get("kind")
    if kind in ("palette", "materials"):
        draw_numbers(im, sc, seconds, font)
    elif kind == "surface":
        draw_surfaces(im, sc, font)
    elif kind == "sheet":
        pill(im, 702, 590, sc["note"], centered=True)
    elif kind == "motion" and seconds >= 2.5:
        note_card(im, 80, 278, sc["note"])
    elif kind == "facing":
        w = HEAD.getlength(sc["note"]) / U + 87
        x, y = W * 5 - w - 12, 560
        panel(im, x, y, w, 69)
        im.alpha_composite(BALL, (u(x + 9), u(y + 8)))
        d = ImageDraw.Draw(im)
        d.text((u(x + 74), u(y + 19)), sc["note"], font=HEAD, fill=TEXT, anchor="lm")
        px = u(x + 74)
        for word, color in [("Up", "#95e6a6"), ("Right", "#ffa0a5"), ("Front", "#a5bfff")]:
            d.text((px, u(y + 47)), word, font=BODY, fill=color, anchor="lm")
            px += BODY.getlength(word) + u(13)

def big(a, scale):
    image = Image.fromarray(a)
    return image.resize((image.width * scale, image.height * scale), Image.Resampling.NEAREST)

def sign(im, x, y, text):
    """A plus or equals sign on a dark disc, where two pictures meet."""
    d = ImageDraw.Draw(im); r = u(17); d.ellipse([x - r, y - r, x + r, y + r], fill=BG, outline=LINE, width=2)
    d.text((x, y - u(1.5)), text, font=font(u(24), "Medium"), fill=LAMP, anchor="mm")
def picture(sc, c, seconds):
    """One scene's picture, with whatever is drawn on top of it."""
    im = Image.new("RGBA", (W * S, H * S), BG + (255,)); half = S // 2
    kind = (sc.get("ov") or {}).get("kind")
    if kind in ("intro", "end"):
        # Bookend the tour with the live sprite beside the type.
        opening = kind == "intro"
        split = 84 if opening else 72
        im.paste(big(native(c["a"])[:, split // 2:W - split // 2], S), (split * S, 0))
        d = ImageDraw.Draw(im)
        baseline = 251 if opening else 306
        d.text((u(45), u(baseline)), "SPU", font=font(u(100), "Semibold"), fill=TEXT, anchor="ls")
        heading = sc["ov"]["heading"] if opening else "Universal Sprite Format"
        d.text((u(48), u(baseline + 50)), heading, font=font(u(23), "Medium"), fill=LAMP, anchor="ls")
        if opening:
            d.multiline_text((u(48), u(337)), sc["note"], font=font(u(21)), spacing=u(7), fill=TEXT)
    elif kind == "sharedSheet":
        with Image.open(os.path.join(src, c["a"])) as sheet:
            im.paste(sheet.resize(im.size, Image.Resampling.NEAREST))
        note = sc["note"] if seconds < 3.75 else sc["ov"]["assembled"]
        pill(im, W * 5 / 2, 70, note, centered=True)
        if seconds >= 3.75:
            pill(im, 245, 132, "Male", centered=True)
            pill(im, 715, 132, "Female", centered=True)
    elif sc.get("sum"):
        # what is stored, small on the left, and what a light makes of it, large on the right
        lw = W // 2 * half                                                # the left column shows the middle half of the stage at half size
        im.paste(big(native(c["flat"])[:, W // 4:W * 3 // 4], half), (0, 0)); im.paste(big(native(c["facing"])[:, W // 4:W * 3 // 4], half), (0, H * half))
        im.paste(big(native(c["a"])[:, (W - (W * S - lw) // S) // 2:][:, :(W * S - lw) // S], S), (lw, 0))
        d = ImageDraw.Draw(im); d.line([0, H * half, lw, H * half], fill=BG, width=4); d.line([lw, 0, lw, H * S], fill=BG, width=4)
        sign(im, lw // 2, H * half, "+"); sign(im, lw, H * S // 2, "=")
        pill(im, 6, 6, "Flat color", HEAD); pill(im, 6, H * half / U + 6, "Facing", HEAD); pill(im, lw / U + 10, 10, sc["note"])
    elif sc.get("grid"):
        columns = len(sc["grid"]) // 2
        # Trim four native pixels of sky and floor per cell to make room for the explanation.
        trim = 4
        heading_height = 4 * trim * half
        width, height = W * S // columns, (H - 2 * trim) * half
        crop = (W - width // half) // 2
        for k, name in enumerate(c["cells"]):
            x, y = k % columns * width, heading_height + k // columns * height
            im.paste(big(native(name)[trim:-trim, crop:crop + width // half], half), (x, y))
        d = ImageDraw.Draw(im)
        d.text((W * S // 2, heading_height // 2), sc["note"], font=LABEL, fill=TEXT, anchor="mm")
        for col in range(1, columns):
            d.line([col * width, heading_height, col * width, H * S], fill=BG, width=4)
        middle = heading_height + height
        d.line([0, middle, W * S, middle], fill=BG, width=4)
        for k, label in enumerate(sc["grid"]):
            x, y = k % columns * width / U + 8, (heading_height + k // columns * height) / U + 8
            label_width = pill(im, x, y, label, HEAD)
            if sc.get("swatches"):
                colors = sc["swatches"][k]
                d = panel(im, x + label_width, y, 8 * len(colors) + 8, 32)
                for j, color in enumerate(colors):
                    left = u(x + label_width + j * 8)
                    d.rectangle([left, u(y + 9), left + u(8) - 1, u(y + 23)], fill=tuple(color))
    else:
        a = native(c["a"])
        if c.get("c"):                                           # a comparison: the middle of each, side by side
            a = np.concatenate([a[:, W // 4:W * 3 // 4], native(c["c"])[:, W // 4:W * 3 // 4]], 1).copy(); a[:, W // 2] = BG
        im.paste(big(a, S), (0, 0)); overlay(im, sc, seconds)
    return np.asarray(im.convert("RGB"))
def compose(f):
    sc = scenes[f["scene"]]; pic = picture(sc, f["caps"], f["seconds"])
    if f.get("prev"):                                            # the new scene sweeps in from the left behind a line
        x = round(W * f["wipe"]) * S; old = picture(scenes[f["scene"] - 1], f["prev"], f["prevSeconds"]).copy(); old[:, :x] = pic[:, :x]
        if 0 < x < W * S: old[:, x:x + S // 2] = LAMP
        pic = old
    return pic

def srt_time(frame):
    seconds, ms = divmod(round(frame * 1000 / fps), 1000)
    minutes, seconds = divmod(seconds, 60)
    hours, minutes = divmod(minutes, 60)
    return f"{hours:02}:{minutes:02}:{seconds:02},{ms:03}"

def write_subtitles(path):
    start = 0
    with open(path, "w", encoding="utf-8") as out:
        for i, (caption, group) in enumerate(groupby(frames, key=lambda f: scenes[f["scene"]]["cap"]), 1):
            end = start + sum(1 for _ in group)
            out.write(f"{i}\n{srt_time(start)} --> {srt_time(end)}\n{caption}\n\n")
            start = end

def leave_captions_off(path):
    """ffmpeg enables the first MP4 subtitle track even with disposition 0."""
    with open(path, "r+b") as file, mmap(file.fileno(), 0) as movie:
        def children(start, end, kind):
            while start < end:
                size = int.from_bytes(movie[start:start + 4], "big")
                if size < 8 or start + size > end:
                    raise ValueError("Invalid box in generated MP4")
                if movie[start + 4:start + 8] == kind:
                    yield start + 8, start + size
                start += size

        moov = next(children(0, len(movie), b"moov"))
        for track in children(*moov, b"trak"):
            media = next(children(*track, b"mdia"))
            handler, _ = next(children(*media, b"hdlr"))
            if movie[handler + 8:handler + 12] in (b"sbtl", b"subt", b"text"):
                header, _ = next(children(*track, b"tkhd"))
                movie[header + 3] &= 0xfe  # Clear only the track_enabled flag.
                return
        raise ValueError("Missing subtitle track in generated MP4")

out = os.path.join(here, "..", "docs", "spu-tour.mp4"); size = f"{W * S}x{H * S}"
if "--poster-only" not in sys.argv:
    subtitles = os.path.join(src, "captions.srt")
    write_subtitles(subtitles)
    enc = subprocess.Popen(["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", size, "-r", str(fps), "-i", "-", "-i", subtitles,
                            "-map", "0:v:0", "-map", "1:s:0", "-c:s", "mov_text", "-metadata:s:s:0", "language=eng", "-disposition:s:0", "0",
                            "-c:v", "libx264", "-threads:v", "6", "-preset", "slow", "-tune", "animation", "-crf", os.environ.get("CRF", "24"), "-pix_fmt", "yuv420p", "-movflags", "+faststart", out], stdin=subprocess.PIPE)
    for i, f in enumerate(frames):
        frame = compose(f); enc.stdin.write(frame.tobytes())
        if str(i) in sys.argv[1:]: Image.fromarray(frame).save(os.path.join(src, f"still-{i}.png"))
    enc.stdin.close()
    if enc.wait(): raise SystemExit("ffmpeg could not encode the tour")
    leave_captions_off(out)
# the poster: the sprite's pieces on their sheet, with a play mark in the corner
shown = [f for f in frames if scenes[f["scene"]]["chapter"] == "Pieces"][-1]
poster = big(native(shown["caps"]["a"]), S).convert("RGBA")
mark = Image.new("RGBA", poster.size, (0, 0, 0, 0)); d = ImageDraw.Draw(mark); r = u(46); cx, cy = W * S - r - u(28), H * S - r - u(28)
d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(15, 17, 23, 215)); d.polygon([(cx - u(13), cy - u(22)), (cx - u(13), cy + u(22)), (cx + u(24), cy)], fill=LAMP + (255,))
Image.alpha_composite(poster, mark).convert("RGB").save(os.path.join(here, "..", "docs", "images", "tour-poster.png"), optimize=True)
print(out, os.path.getsize(out) // 1024, "KB,", len(frames), "frames,", round(len(frames) / fps, 1), "s,", size)
