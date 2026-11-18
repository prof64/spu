"""Animate palette and material numbers from their key onto Charizard's pinned pose."""
import base64
from functools import lru_cache
from io import BytesIO

import numpy as np
from PIL import Image, ImageDraw


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


def draw_numbers(im, sc, seconds, font):
    kind, info = sc["ov"]["kind"], sc["info"]
    targets = anchors(kind, info["indices"])
    scale = im.width / 192
    px = lambda n: round(n * scale)
    d = ImageDraw.Draw(im)
    text = (240, 238, 231)
    heading, numeral, name = font(px(3.25), "Semibold"), font(px(3), "Semibold"), font(px(2.9))

    # Both keys sit at the same eye level as the pose, in the space beside it.
    palette = kind == "palette"
    left, width = 17, 35
    height = 31 if palette else 54
    top = 69 - height / 2
    d.rounded_rectangle((px(left), px(top), px(left + width), px(top + height)),
                        radius=px(1.1), fill=(24, 26, 33), outline=(65, 67, 76), width=px(.2))
    for line, words in enumerate(sc["note"].splitlines()):
        d.text((px(left + 2), px(top + 1.7 + line * 3.9)), words, font=heading, fill=text, anchor="lt")

    origins, colors = {}, {}
    if palette:
        for i, rgb in enumerate(info["palette"], 1):
            x = left + 4.5 + ((i - 1) % 5) * 6.5
            y = top + 14.9 + ((i - 1) // 5) * 5.9
            origins[i], colors[i] = (x, y), tuple(rgb)
            number_badge(d, px, numeral, x, y, 5.5, 4.8, colors[i], i)
    else:
        for i, row in enumerate(info["rows"]):
            x, y = left + 4.4, top + 14.7 + i * 5.9
            origins[i], colors[i] = (x, y), material_color(i)
            number_badge(d, px, numeral, x, y, 4.6, 4.6, colors[i], i)
            d.text((px(x + 3.7), px(y)), row["name"], font=name, fill=text, anchor="lm")

    for order, (index, ax, ay, dx, dy) in enumerate(targets):
        k = min(1, max(0, (seconds - .8 - order * .21) / .95))
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
            d.line([tip, (px(x), px(y))], fill=text, width=px(.28))
            d.ellipse((tip[0]-px(.38), tip[1]-px(.38), tip[0]+px(.38), tip[1]+px(.38)), fill=text, outline=(20, 22, 27))
        number_badge(d, px, numeral, x, y, 5.0 if index >= 10 else 4.2, 4.1, colors[index], index, moving=True)


@lru_cache(maxsize=8)
def surface_sample(encoded):
    with Image.open(BytesIO(base64.b64decode(encoded))) as image:
        return image.convert("RGB")


def draw_surfaces(im, sc, font):
    """Show the numbered regions in real light, followed by paired material samples."""
    info, ov = sc["info"], sc["ov"]
    targets = anchors("materials", info["indices"])
    scale = im.width // 192
    px = lambda n: round(n * scale)
    d = ImageDraw.Draw(im)
    light, dark = (240, 238, 231), (24, 26, 33)
    title, numeral, body = font(px(3.45), "Semibold"), font(px(3), "Semibold"), font(px(3.15))
    rows = info["rows"]
    selected = ov.get("samples") or ([3] if ov.get("wing") else None)

    if selected is None:
        # Each thumbnail is a 9 × 6 cutout of the lit pose at the ID's exact pixel.
        # Capture before drawing the on-sprite numbers so no label can appear in a thumbnail.
        samples = {}
        for index, ax, ay, _, _ in targets:
            x, y = int(ax) - 4, int(ay) - 3
            crop = im.crop((px(x), px(y), px(x + 9), px(y + 6)))
            samples[index] = crop.resize((px(13.5), px(9)), Image.Resampling.NEAREST)
        panel = (6, 19, 56, 110)
        d.rounded_rectangle(tuple(px(n) for n in panel), radius=px(1), fill=dark)
        d.text((px(8.5), px(22)), "Charizard in the light", font=title, fill=light, anchor="lt")
        for index, row in enumerate(rows):
            y = 29 + index * 11.4
            im.paste(samples[index], (px(8.5), px(y)))
            number_badge(d, px, numeral, 26.5, y + 4.5, 4.6, 4.6, material_color(index), index)
            d.text((px(30.3), px(y + 4.5)), row["name"], font=body, fill=light, anchor="lm")
    else:
        # One specimen on each side leaves the whole lit sprite visible between them.
        for k, raw in enumerate(info["samples"]):
            x = (5, 138)[k]
            wing = bool(ov.get("wing"))
            index = 3 if wing else selected[k]
            name = ("If opaque", "Wing membrane")[k] if wing else rows[index]["name"].capitalize()
            note = (["An opaque wing", "blocks the backlight."], ["Light passes through", "Charizard’s membrane."])[k] if wing else ov["notes"][k]
            panel = (x, 30, x + 49, 96)
            d.rounded_rectangle(tuple(px(n) for n in panel), radius=px(1), fill=dark)
            number_badge(d, px, numeral, x + 5.3, 36, 4.6, 4.6, material_color(index), index)
            d.text((px(x + 9.3), px(36)), name, font=title, fill=light, anchor="lm")
            sample = surface_sample(raw)
            # Integer enlargement preserves the page's native pixels in the specimen.
            zoom = px(42) // sample.width
            enlarged = sample.resize((sample.width * zoom, sample.height * zoom), Image.Resampling.NEAREST)
            im.paste(enlarged, (px(x + 24.5) - enlarged.width // 2, px(61) - enlarged.height // 2))
            for j, line in enumerate(note):
                d.text((px(x + 3), px(83 + j * 5)), line, font=body, fill=light, anchor="lm")

    for index, ax, ay, dx, dy in targets:
        if selected is not None and index not in selected:
            continue
        x, y = ax + dx, ay + dy
        if dx or dy:
            d.line([(px(ax), px(ay)), (px(x), px(y))], fill=dark, width=px(.6))
            d.line([(px(ax), px(ay)), (px(x), px(y))], fill=light, width=px(.28))
        number_badge(d, px, numeral, x, y, 4.2, 4.1, material_color(index), index, moving=True)
