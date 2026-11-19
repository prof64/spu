"""Compose the tour at integer sprite-pixel scales, then encode with selectable captions."""
import base64, json, os, subprocess, sys
from io import BytesIO
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
ease = lambda x: (lambda x: x * x * (3 - 2 * x))(min(1, max(0, x)))
GROW = .45                                                  # how long a growing grid takes to re-fit itself
ASSEMBLED = 3.75                                            # when both Jellicent forms have left their sheet (video_looks.cjs)
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
    if left + tile.width > 0:                                 # a panel sliding off the left edge keeps the part still in frame
        im.alpha_composite(tile, (max(left, 0), top), (max(-left, 0), 0))
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
    elif kind == "glow" and ov.get("title"):
        note_card(im, 685, 106, sc["note"], ov["title"], ov["material"])
    elif kind == "sheet":
        pill(im, 702, 590, sc["note"], centered=True)
    elif kind == "motion":
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
def grid_rects(n):
    """Where n cells sit, for up to eight. A cell never changes rows and never passes another, so the eye can stay
    on one: the first fills the frame, the second takes the right half, and the third and fourth go underneath as the
    two above them zoom out. After that the grid gains a column at a time, top then bottom, and the columns already
    there only narrow. Every one of these puts a whole number of video pixels on each sprite pixel."""
    w, h = W * S, H * S
    if n == 1: return [(0, 0, w, h)]
    if n == 2: return [(0, 0, w / 2, h), (w / 2, 0, w / 2, h)]
    if n == 3: return [(0, 0, w / 2, h), (w / 2, 0, w / 2, h / 2), (w / 2, h / 2, w / 2, h / 2)]
    slots = [(0, 0), (1, 0), (1, 1), (0, 1), (2, 0), (2, 1), (3, 0), (3, 1)]             # column and row, in order of arrival
    w /= (n + 1) // 2
    return [(column * w, row * h / 2, w, h / 2) for column, row in slots[:n]]

CARD = (92, 26)                                             # a material's card, in sprite pixels of its cell
cards = {}
def material_card(card, label, s):
    """A sample ball, the material's name and where it sits on the two dials, at s video pixels per sprite pixel."""
    if (label, s) not in cards:
        px = lambda v: round(v * s)
        im = Image.new("RGBA", (px(CARD[0]), px(CARD[1])), (0, 0, 0, 0)); d = ImageDraw.Draw(im)
        d.rounded_rectangle((0, 0, im.width - 1, im.height - 1), radius=px(1), fill=PANEL[:3] + (235,))
        with Image.open(BytesIO(base64.b64decode(card["ball"]))) as tile:       # the sphere and one pixel around it
            im.paste(tile.convert("RGB").crop((4, 5, 52, 53)).resize((px(24), px(24)), Image.Resampling.NEAREST), (px(1), px(1)))
        d.text((px(28.5), px(6)), label, font=font(px(6.6), "Semibold"), fill=TEXT, anchor="lm")
        ends = font(px(5.2)); rail = (96, 100, 114)
        for row, (low, high, value) in enumerate([("fuzzy", "polished", card["f"]), ("metal", "clear", card["b"])]):
            y, x0, x1 = px(14.4 + row * 7), px(45), px(68.5)
            d.text((px(42.5), y), low, font=ends, fill=DIM, anchor="rm"); d.text((px(71), y), high, font=ends, fill=DIM, anchor="lm")
            d.line([x0, y, x1, y], fill=rail, width=max(2, px(.6)))
            d.line([(x0 + x1) // 2, y - px(1.3), (x0 + x1) // 2, y + px(1.3)], fill=rail, width=max(2, px(.5)))
            x, r = x0 + (x1 - x0) * (value + 1) / 2, px(1.7)
            d.ellipse([x - r, y - r, x + r, y + r], fill=LAMP, outline=BG, width=max(1, px(.4)))
        cards[label, s] = im
    return cards[label, s]

def growing_grid(im, sc, c, seconds):
    """A grid that gains a cell at each time in `at`. The cells already there shrink and move over to make room,
    which shows more of each scene or less; the new one fades in where it belongs."""
    at, labels = sc["grow"]["at"], sc["grow"]["labels"]
    n = sum(t <= seconds for t in at); move = ease((seconds - at[n - 1]) / GROW)
    now, before = grid_rects(n), grid_rects(n - 1) if n > 1 else None
    for i, name in enumerate(c["cells"][:n]):
        newest = n > 1 and i == n - 1
        x, y, w, h = now[i] if newest or n == 1 else [a + (b - a) * move for a, b in zip(before[i], now[i])]
        left, top, right, bottom = round(x), round(y), round(x + w), round(y + h)
        s = (bottom - top) / H; wide = (right - left) / s                      # video pixels per sprite pixel; sprite pixels across
        middle = min(W - wide / 2, W / 2 + sc["grow"]["shifts"][i])            # a cell narrower than the scene can look to one side of it
        if s == int(s) and wide == int(wide) and middle - wide / 2 == int(middle - wide / 2):      # at rest: exact blocks
            pic = big(native(name)[:, int(middle - wide / 2):int(middle + wide / 2)], int(s))
        else:                                                                  # on the way: averaged down from the largest size
            a = round((middle - wide / 2) * S); b = min(W * S, a + round(wide * S))
            pic = big(native(name), S).crop((a, 0, b, H * S)).resize((right - left, bottom - top), Image.Resampling.BOX)
        pic = pic.convert("RGBA")
        card = material_card(sc["info"]["cards"][i], labels[i], int(s)) if s in (S // 2, S) else \
            material_card(sc["info"]["cards"][i], labels[i], S).resize((round(CARD[0] * s), round(CARD[1] * s)), Image.Resampling.LANCZOS)
        pic.alpha_composite(card, (round(2 * s), round(2 * s)))
        ImageDraw.Draw(pic).rectangle([0, 0, pic.width - 1, pic.height - 1], outline=BG + (255,), width=2)
        shown = ease((seconds - at[i] - .12) / .3) if newest else 1
        if shown > 0: im.paste(pic, (left, top), Image.new("L", pic.size, round(255 * shown)))
        marked = shown * (1 - ease((seconds - at[i] - 1.0) / .4)) if newest else 0       # a frame around the newcomer, for a second
        if marked > 0:
            frame = Image.new("RGBA", im.size, (0, 0, 0, 0))
            ImageDraw.Draw(frame).rectangle([left + 2, top + 2, right - 3, bottom - 3], outline=LAMP + (round(255 * marked),), width=4)
            im.alpha_composite(frame)

def render(sc, c, seconds, labels=True):
    """One scene's picture, with whatever is drawn on top of it. `labels` off leaves out what fades."""
    im = Image.new("RGBA", (W * S, H * S), BG + (255,)); half = S // 2
    kind = (sc.get("ov") or {}).get("kind")
    k = 1 if c.get("open") is None else c["open"]                # how far a side panel is in
    if kind in ("intro", "end"):
        # Bookend the tour with the live sprite beside the type. The type's panel slides off to the left, and
        # the picture gives way by half as much, so the sprite stays in the middle of what is left.
        opening = kind == "intro"
        width = (84 if opening else 72) * S
        left = round(width * (k - 1))
        im.paste(big(native(c["a"]), S), ((left + width) // 2, 0))
        if k > 0:
            d = ImageDraw.Draw(im); d.rectangle([left, 0, left + width - 1, H * S], fill=BG)
            baseline = 190 if opening else 306
            d.text((left + u(45), u(baseline)), "SPU", font=font(u(100), "Semibold"), fill=TEXT, anchor="ls")
            heading = sc["ov"]["heading"] if opening else "Universal Sprite Format"
            d.text((left + u(48), u(baseline + 50)), heading, font=font(u(23), "Medium"), fill=LAMP, anchor="ls")
            if opening:
                # What the file holds, as a list to glance down instead of a sentence to read.
                d.text((left + u(48), u(baseline + 104)), sc["ov"]["lead"], font=font(u(19)), fill=DIM, anchor="ls")
                for i, item in enumerate(sc["ov"]["items"]):
                    y = u(baseline + 142 + i * 34)
                    d.rectangle([left + u(49), y - u(15), left + u(49) + u(7) - 1, y - u(15) + u(7) - 1], fill=LAMP)
                    d.text((left + u(68), y), item, font=font(u(24), "Medium"), fill=TEXT, anchor="ls")
    elif kind == "sharedSheet":
        with Image.open(os.path.join(src, c["a"])) as sheet:
            im.paste(sheet.resize(im.size, Image.Resampling.NEAREST))
        if labels:
            pill(im, W * 5 / 2, 70, sc["note"] if seconds < ASSEMBLED else sc["ov"]["assembled"], centered=True)
            if seconds >= ASSEMBLED:
                pill(im, 245, 132, "Male", centered=True)
                pill(im, 715, 132, "Female", centered=True)
    elif sc.get("sum"):
        # What is stored, small on the left, and what a light makes of it, large on the right.
        # The left column shows the middle half of the stage at half size, and slides away like the title's panel.
        lw = W // 2 * half; left = round(lw * (k - 1))
        im.paste(big(native(c["a"]), S), ((left + lw) // 2, 0))
        if k > 0:
            im.paste(big(native(c["flat"])[:, W // 4:W * 3 // 4], half), (left, 0)); im.paste(big(native(c["facing"])[:, W // 4:W * 3 // 4], half), (left, H * half))
            d = ImageDraw.Draw(im); d.line([left, H * half, left + lw, H * half], fill=BG, width=4); d.line([left + lw, 0, left + lw, H * S], fill=BG, width=4)
            sign(im, left + lw // 2, H * half, "+"); sign(im, left + lw, H * S // 2, "=")
            pill(im, left / U + 6, 6, "Flat color", HEAD); pill(im, left / U + 6, H * half / U + 6, "Facing", HEAD)
        if labels: pill(im, (left + lw) / U + 10, 10, sc["note"])
    elif sc.get("grow"):
        growing_grid(im, sc, c, seconds)
    elif sc.get("grid"):
        columns = len(sc["grid"]) // 2
        # Trim four native pixels of sky and floor per cell to make room for the explanation.
        trim = 4
        heading_height = 4 * trim * half
        width, height = W * S // columns, (H - 2 * trim) * half
        crop = (W - width // half) // 2
        for j, name in enumerate(c["cells"]):
            x, y = j % columns * width, heading_height + j // columns * height
            im.paste(big(native(name)[trim:-trim, crop:crop + width // half], half), (x, y))
        d = ImageDraw.Draw(im)
        d.text((W * S // 2, heading_height // 2), sc["note"], font=LABEL, fill=TEXT, anchor="mm")
        for col in range(1, columns):
            d.line([col * width, heading_height, col * width, H * S], fill=BG, width=4)
        middle = heading_height + height
        d.line([0, middle, W * S, middle], fill=BG, width=4)
        for j, label in enumerate(sc["grid"]):
            x, y = j % columns * width / U + 8, (heading_height + j // columns * height) / U + 8
            label_width = pill(im, x, y, label, HEAD)
            if sc.get("swatches"):
                colors = sc["swatches"][j]
                d = panel(im, x + label_width, y, 8 * len(colors) + 8, 32)
                for i, color in enumerate(colors):
                    left = u(x + label_width + i * 8)
                    d.rectangle([left, u(y + 9), left + u(8) - 1, u(y + 23)], fill=tuple(color))
    else:
        a = big(native(c["a"]), S)
        if c.get("c"):
            # A comparison: the middle of each, side by side. The left one pushes in from the edge of the
            # frame, and the right one, which had the frame to itself, moves over to make room.
            edge = round(W * S / 2 * k)
            im.paste(big(native(c["c"]), S), (edge // 2, 0))
            if edge:
                im.paste(a.crop((W * S // 4, 0, W * S * 3 // 4, H * S)), (edge - W * S // 2, 0))
                ImageDraw.Draw(im).rectangle([edge, 0, edge + S - 1, H * S], fill=BG)
        else:
            im.paste(a, (0, 0))
        if labels: overlay(im, sc, seconds)
    return np.asarray(im.convert("RGB"))
def opacity(sc, seconds):
    """How much of a scene's labels show: they can fade in after it starts and be gone before it ends."""
    fade = sc.get("fade", .3)
    if (sc.get("ov") or {}).get("kind") == "sharedSheet":
        return ease(abs(seconds - ASSEMBLED) / fade)             # one note gives way to the next
    start, end = sc.get("show") or (None, None); k = 1.0
    if start is not None: k *= ease((seconds - start) / fade)
    if end is not None: k *= ease((end - seconds) / fade)
    return k
def picture(sc, c, seconds):
    k = opacity(sc, seconds)
    if k >= 1: return render(sc, c, seconds)
    plain = render(sc, c, seconds, labels=False)
    if k <= 0: return plain
    return np.rint(plain + (render(sc, c, seconds).astype(np.float32) - plain) * k).astype(np.uint8)
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

out = os.path.join(here, "out", "spu-tour.mp4"); size = f"{W * S}x{H * S}"
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
