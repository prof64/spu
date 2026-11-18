"""Animate palette and material numbers from their key onto Charizard's pinned pose."""
import base64
from functools import lru_cache
from io import BytesIO

import numpy as np
from PIL import Image, ImageDraw


LIGHT, DARK, ACCENT = (240, 238, 231), (24, 26, 33), (255, 179, 87)   # text, the panels it sits on, and a heading that states the point

# Preferred spot in the 192 × 128 pose, and where to set the number relative to it.
# Tiny regions (line art, claws, flame) get a short pointer so the pixels remain visible.
HIDE = (95, 76, 0, 0)
BELLY = (90, 90, 0, 0)
WING = (113, 53, 0, 0)
LINE = (128, 64, 7, -3)
CLAWS = (116, 106, 5, 5)
MOUTH = (71, 78, -7, 5)
PLACES = {
    "palette": {
        7: HIDE, 5: BELLY, 12: WING, 1: LINE, 2: CLAWS,
        15: MOUTH, 13: (114, 37, 7, 0), 14: (108, 29, -5, -5),
    },
    "materials": {
        1: HIDE, 2: BELLY, 3: WING, 0: LINE, 4: CLAWS,
        5: (111, 33, 0, 0), 6: MOUTH,
    },
}


@lru_cache(maxsize=2)
def anchors(kind, encoded):
    """Choose the nearest pixel of the requested ID on the captured sprite."""
    mask = np.frombuffer(base64.b64decode(encoded), np.uint8).reshape(128, 192)
    visible = set(np.unique(mask)) - {255}
    if visible != set(PLACES[kind]):
        raise ValueError(f"Update {kind} label positions for the visible IDs {visible}")
    result = []
    for index, (x, y, dx, dy) in PLACES[kind].items():
        ys, xs = np.where(mask == index)
        at = np.argmin((xs - x) ** 2 + (ys - y) ** 2)
        result.append((index, float(xs[at]) + .5, float(ys[at]) + .5, dx, dy))
    return result


def material_color(index):
    """Material ID hue used in the page's material view."""
    if index == 0:
        return (20, 20, 20)
    hue = (index * .618 + .05) % 1
    channels = (max(0, min(1, abs(((hue + shift) % 1) * 6 - 3) - 1)) for shift in (0, .667, .333))
    return tuple(round(255 * (.0625 + .75 * value)) for value in channels)


def number_badge(d, px, numeral, x, y, w, h, color, index, moving=False):
    ink = (12, 14, 19) if sum(c * weight for c, weight in zip(color, (.2126, .7152, .0722))) > 122 else (255, 255, 255)
    box = (px(x - w / 2), px(y - h / 2), px(x + w / 2), px(y + h / 2))
    d.rounded_rectangle(box, radius=px(.6), fill=color, outline=(238, 238, 235) if moving else (117, 120, 128), width=px(.23))
    if moving:
        d.rounded_rectangle((box[0]-1, box[1]-1, box[2]+1, box[3]+1), radius=px(.7), outline=(20, 22, 27))
    d.text((px(x), px(y)), str(index), font=numeral, fill=ink, anchor="mm")


@lru_cache(maxsize=24)
def surface_sample(encoded, max_size):
    """Keep the entire sphere in view at an integer enlargement."""
    with Image.open(BytesIO(base64.b64decode(encoded))) as image:
        size = image.width * (max_size // image.width) if max_size >= image.width else max_size
        return image.convert("RGB").resize((size, size), Image.Resampling.NEAREST)


@lru_cache(maxsize=24)
def map_sample(encoded, max_size, color):
    """The outline's black sphere, recolored: a flat sample in a material's map color, exactly where its lit sphere goes."""
    sample = np.array(surface_sample(encoded, max_size))
    sample[sample.max(-1) < 12] = color
    return Image.fromarray(sample)


def material_list(im, sc, font, lit):
    """Every material down the left: a sample, its number and its name. Returns where each number sits.

    The sample is the map color on the material map and a lit sphere after it. Nothing else differs,
    so a wipe between the two changes the samples and the sprite and leaves the list where it is."""
    info = sc["info"]
    px = lambda n: round(n * (im.width // 192))
    d = ImageDraw.Draw(im)
    title, numeral, body = font(px(3.45), "Semibold"), font(px(3), "Semibold"), font(px(3.15))
    d.rounded_rectangle(tuple(px(n) for n in (5, 2.5, 56, 122)), radius=px(1), fill=DARK)
    for j, line in enumerate(sc["note"].splitlines()):
        d.text((px(7.5), px(5 + j * 4.4)), line, font=title, fill=LIGHT, anchor="lt")
    spots = {}
    for index, row in enumerate(info["rows"]):
        cy = 22.7 + index * 15
        sample = surface_sample(info["samples"][index], px(14)) if lit else map_sample(info["samples"][0], px(14), material_color(index))
        im.paste(sample, (px(7.5), px(cy) - sample.height // 2))
        number_badge(d, px, numeral, 25.8, cy, 4.6, 4.6, material_color(index), index)
        d.text((px(29.6), px(cy)), row["name"], font=body, fill=LIGHT, anchor="lm")
        spots[index] = (25.8, cy)
    return spots


def draw_numbers(im, sc, seconds, font):
    kind, info = sc["ov"]["kind"], sc["info"]
    targets = anchors(kind, info["indices"])
    scale = im.width / 192
    px = lambda n: round(n * scale)
    d = ImageDraw.Draw(im)
    numeral = font(px(3), "Semibold")

    if kind == "palette":
        # Two cards in the space beside the pose. The first says what the file stores, and is there from the start.
        # The palette key fades in under it once that has been read.
        ov, left, width = sc["ov"], 6, 49
        card = lambda top, height: d.rounded_rectangle((px(left), px(top), px(left + width), px(top + height)),
                                                       radius=px(1.1), fill=DARK, outline=(65, 67, 76), width=px(.2))
        heading, body = font(px(3.45), "Semibold"), font(px(3.15))
        top = 28
        if ov.get("lead"):
            lines = ov["lead"]["note"].splitlines()
            card(top, 8.6 + 4.4 * len(lines))
            d.text((px(left + 2.4), px(top + 4.3)), ov["lead"]["heading"], font=heading, fill=ACCENT, anchor="lm")
            for line, words in enumerate(lines):
                d.text((px(left + 2.4), px(top + 9.2 + line * 4.4)), words, font=body, fill=LIGHT, anchor="lm")
            top += 8.6 + 4.4 * len(lines) + 3
        origins, colors = {}, {}
        for i, rgb in enumerate(info["palette"], 1):
            origins[i] = (left + 6.5 + ((i - 1) % 5) * 9, top + 14.9 + ((i - 1) // 5) * 5.9)
            colors[i] = tuple(rgb)
        shown = 1 if ov.get("key") is None else min(1, max(0, (seconds - ov["key"]) / .3))
        if shown > 0:
            before = im.copy() if shown < 1 else None
            card(top, 31)
            for line, words in enumerate(sc["note"].splitlines()):
                d.text((px(left + 2.4), px(top + 1.7 + line * 3.9)), words, font=font(px(3.25), "Semibold"), fill=LIGHT, anchor="lt")
            for i, (x, y) in origins.items():
                number_badge(d, px, numeral, x, y, 7.4, 4.8, colors[i], i)
            if before is not None:
                im.paste(Image.blend(before, im, shown * shown * (3 - 2 * shown)))
    else:
        origins = material_list(im, sc, font, lit=False)
        colors = {i: material_color(i) for i in origins}

    # The numbers wait for the key to be read, then leave one after another.
    start = sc["ov"].get("from", .8)
    for order, (index, ax, ay, dx, dy) in enumerate(targets):
        k = min(1, max(0, (seconds - start - order * .16) / .95))
        if k == 0:
            continue
        k = k * k * (3 - 2 * k)
        sx, sy = origins[index]
        x = sx + (ax + dx - sx) * k
        y = sy + (ay + dy - sy) * k - 18 * k * (1 - k)
        if k > .90 and (dx or dy):
            # Draw the stem from the exact pixel center toward the landing badge.
            tip = (px(ax), px(ay))
            d.line([tip, (px(x), px(y))], fill=(20, 22, 27), width=px(.6))
            d.line([tip, (px(x), px(y))], fill=LIGHT, width=px(.28))
            d.ellipse((tip[0]-px(.38), tip[1]-px(.38), tip[0]+px(.38), tip[1]+px(.38)), fill=LIGHT, outline=(20, 22, 27))
        number_badge(d, px, numeral, x, y, 5.0 if index >= 10 else 4.2, 4.1, colors[index], index, moving=True)


def draw_surfaces(im, sc, font):
    """The numbered materials as lit spheres, with each number on the sprite."""
    info = sc["info"]
    px = lambda n: round(n * (im.width // 192))
    d = ImageDraw.Draw(im)
    numeral = font(px(3), "Semibold")
    material_list(im, sc, font, lit=True)
    for index, ax, ay, dx, dy in anchors("materials", info["indices"]):
        x, y = ax + dx, ay + dy
        if dx or dy:
            d.line([(px(ax), px(ay)), (px(x), px(y))], fill=DARK, width=px(.6))
            d.line([(px(ax), px(ay)), (px(x), px(y))], fill=LIGHT, width=px(.28))
        number_badge(d, px, numeral, x, y, 4.2, 4.1, material_color(index), index, moving=True)
