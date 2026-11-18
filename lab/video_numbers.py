"""Animate palette and material numbers from their key onto Charizard's pinned pose."""
import base64
from functools import lru_cache

import numpy as np
from PIL import ImageDraw


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


def draw_numbers(im, sc, seconds, font):
    kind, info = sc["ov"]["kind"], sc["info"]
    targets = anchors(kind, info["indices"])
    scale = im.width / 192
    px = lambda n: round(n * scale)
    d = ImageDraw.Draw(im)
    text, edge = (240, 238, 231), (117, 120, 128)
    heading, numeral, name = font(px(3.5), "Semibold"), font(px(3), "Semibold"), font(px(2.9))

    # Both keys sit at the same eye level as the pose, in the space beside it.
    palette = kind == "palette"
    left, width, height = (17, 35, 26) if palette else (17, 35, 49)
    top = 69 - height / 2
    d.rounded_rectangle((px(left), px(top), px(left + width), px(top + height)),
                        radius=px(1.1), fill=(24, 26, 33), outline=(65, 67, 76), width=px(.2))
    title = "Color palette" if palette else "Materials"
    d.text((px(left + 2), px(top + 1.5)), title, font=heading, fill=text, anchor="lt")

    def badge(x, y, w, h, color, index, moving=False):
        ink = (12, 14, 19) if sum(c * weight for c, weight in zip(color, (.2126, .7152, .0722))) > 122 else (255, 255, 255)
        box = (px(x - w / 2), px(y - h / 2), px(x + w / 2), px(y + h / 2))
        d.rounded_rectangle(box, radius=px(.6), fill=color, outline=(238, 238, 235) if moving else edge, width=px(.23))
        if moving:
            d.rounded_rectangle((box[0]-1, box[1]-1, box[2]+1, box[3]+1), radius=px(.7), outline=(20, 22, 27))
        d.text((px(x), px(y)), str(index), font=numeral, fill=ink, anchor="mm")

    origins, colors = {}, {}
    if palette:
        for i, rgb in enumerate(info["palette"], 1):
            x = left + 4.5 + ((i - 1) % 5) * 6.5
            y = top + 10.3 + ((i - 1) // 5) * 5.9
            origins[i], colors[i] = (x, y), tuple(rgb)
            badge(x, y, 5.5, 4.8, colors[i], i)
    else:
        for i, row in enumerate(info["rows"]):
            x, y = left + 4.4, top + 10.0 + i * 5.9
            origins[i], colors[i] = (x, y), material_color(i)
            badge(x, y, 4.6, 4.6, colors[i], i)
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
        badge(x, y, 5.0 if index >= 10 else 4.2, 4.1, colors[index], index, moving=True)
