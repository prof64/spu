"""Read an SPU version 1 file and place its parts at a tick (port of src/reference.rs)."""
import io, json, math, zipfile
import numpy as np
from PIL import Image

EASE = {
    "step": lambda t: 0.0, "linear": lambda t: t, "easeIn": lambda t: t * t,
    "easeOut": lambda t: t * (2 - t), "easeInOut": lambda t: t * t * (3 - 2 * t),
}

def sample(keys, tick, default):
    i = 0
    for k in keys:
        if k["tick"] <= tick: i += 1
        else: break
    if i == 0:
        return keys[0]["value"] if keys else default
    a = keys[i - 1]
    if i < len(keys) and a.get("ease", "step") != "step":
        b = keys[i]
        return a["value"] + (b["value"] - a["value"]) * EASE[a["ease"]]((tick - a["tick"]) / (b["tick"] - a["tick"]))
    return a["value"]

def hold(keys, tick, default):
    i = 0
    for k in keys:
        if k["tick"] <= tick: i += 1
        else: break
    return keys[max(i - 1, 0)]["value"] if keys else default

def trs(track, tick):
    g = lambda n, d: sample(track.get(n) or [], tick, d)
    return [g("x", 0.0), g("y", 0.0), g("rotation", 0.0), g("scaleX", 1.0), g("scaleY", 1.0)]

def matrix(t):
    x, y, r, sx, sy = t
    s, c = math.sin(math.radians(r)), math.cos(math.radians(r))
    return [c * sx, s * sx, -s * sy, c * sy, x, y]

def then(p, l):
    m = matrix(p)
    x = m[0] * l[0] + m[2] * l[1] + m[4]
    y = m[1] * l[0] + m[3] * l[1] + m[5]
    turn = -l[2] if (p[3] < 0) != (p[4] < 0) else l[2]
    return [x, y, p[2] + turn, p[3] * l[3], p[4] * l[4]]

class Sprite:
    def __init__(self, path, view="front"):
        z = zipfile.ZipFile(path)
        self.doc = json.loads(z.read("sprite.json"))
        self.view = next(v for v in self.doc["views"] if v["id"] == view)
        # palette colors with their own opacity; index 0 always draws nothing. Short palettes are padded.
        self.rgba = {p["id"]: ([self._rgba(c) for c in p["colors"]] + [(0, 0, 0, 0)] * 256)[:256] for p in self.doc.get("palettes", [])}
        for q in self.rgba.values(): q[0] = (q[0][0], q[0][1], q[0][2], 0)
        self.palettes = {k: [c[:3] for c in v] for k, v in self.rgba.items()}
        self.images = self.view["images"]
        self.index = {im["id"]: i for i, im in enumerate(self.images)}
        self.art = []  # palette indices per drawing (0 transparent)
        for im in self.images:
            png = Image.open(io.BytesIO(z.read(im["color"])))
            assert png.mode == "P", (im["id"], png.mode)
            a = np.array(png); self.art.append(a)
        # a palette color that is fully see-through draws nothing, so treat it as empty
        clear = np.array([c[3] == 0 for c in next(iter(self.rgba.values()))])
        self.art = [np.where(clear[a], 0, a).astype(np.uint8) for a in self.art]
        # The current format keeps opacity on the pixel. Color numbers stay as they are, so that the
        # shades this lab works out are the numbers the converted file uses.
        src = next(iter(self.rgba.values())); strength = np.array([c[3] for c in src], np.uint8)
        self.opacity = [np.where(a != 0, strength[a], 0).astype(np.uint8) for a in self.art]
        c = self.view["canvas"]
        self.canvas = (int(c["x"]), int(c["y"]), int(c["width"]), int(c["height"]))

    @staticmethod
    def _rgba(c):
        c = c.lstrip("#")
        return tuple(int(c[i:i + 2], 16) for i in (0, 2, 4)) + ((int(c[6:8], 16),) if len(c) >= 8 else (255,))

    @staticmethod
    def _col(c):
        c = c.lstrip("#")
        return tuple(int(c[i:i + 2], 16) for i in (0, 2, 4))

    def clip(self, animation="idle"):
        return next(a for a in self.view["animations"] if a["id"] == animation)

    def placements(self, tick, animation="idle", variants=()):
        """Back-to-front list of (drawing index, matrix, opacity, part index)."""
        clip = self.clip(animation)
        tick = tick % clip["length"] if clip.get("looping", True) else max(0, min(tick, clip["length"] - 1))
        parts = self.view["parts"]
        pidx = {p["id"]: i for i, p in enumerate(parts)}
        tracks = [None] * len(parts)
        for t in clip["tracks"]: tracks[pidx[t["part"]]] = t
        locals_ = [trs(t, tick) if t else [0, 0, 0, 1, 1] for t in tracks]
        repl = {}
        for vid in variants:
            v = next(x for x in self.doc["variants"] if x["id"] == vid)
            for r in v.get("replacements", []):
                if r["view"] == self.view["id"]: repl[r["image"]] = r["with"]
        out = []
        for p, t in enumerate(tracks):
            if t is None or not hold(t.get("visible") or [], tick, True): continue
            image = hold(t.get("image") or [], tick, None)
            if image is None: continue
            chain = [p]
            while parts[chain[-1]].get("parent"): chain.append(pidx[parts[chain[-1]]["parent"]])
            world = locals_[chain[-1]]
            for c in reversed(chain[:-1]): world = then(world, locals_[c])
            zed = sample(t.get("z") or [], tick, 0.0)
            op = max(0.0, min(1.0, sample(t.get("opacity") or [], tick, 1.0)))
            out.append((zed, p, image, op, world))
        out.sort(key=lambda e: (e[0], e[1]))
        res = []
        for _, p, image, op, world in out:
            base = self.images[self.index[image]]
            for d in [image] + list(base.get("overlays") or []):
                d = repl.get(d, d)
                res.append((self.index[d], matrix(world), op, p))
        return res

    def render_ids(self, tick, animation="idle", variants=()):
        """Per canvas pixel: drawing index (-1 none), texel u, v."""
        cx, cy, W, H = self.canvas
        ids = -np.ones((H, W), np.int32); uu = np.zeros((H, W), np.int32); vv = np.zeros((H, W), np.int32)
        self.inst = -np.ones((H, W), np.int32); self.placed = self.placements(tick, animation, variants)
        ys, xs = np.mgrid[0:H, 0:W]
        px, py = xs + cx + 0.5, ys + cy + 0.5
        for k, (d, m, op, _) in enumerate(self.placed):
            if op <= 0: continue
            det = m[0] * m[3] - m[1] * m[2]
            if abs(det) < 1e-12: continue
            a, b, c, dd = m[3] / det, -m[1] / det, -m[2] / det, m[0] / det
            tx, ty = -(a * m[4] + c * m[5]), -(b * m[4] + dd * m[5])
            im = self.images[d]; art = self.art[d]; h, w = art.shape
            lx = a * px + c * py + tx - im["x"]; ly = b * px + dd * py + ty - im["y"]
            def texel(l):
                e = np.round(l)
                return np.where(np.abs(l - e) < 1e-9, e, np.floor(l)).astype(np.int64)
            u, v = texel(lx), texel(ly)
            ok = (u >= 0) & (v >= 0) & (u < w) & (v < h)
            uc, vc = np.clip(u, 0, w - 1), np.clip(v, 0, h - 1)
            ok &= art[vc, uc] != 0
            ids[ok] = d; uu[ok] = uc[ok]; vv[ok] = vc[ok]; self.inst[ok] = k
        return ids, uu, vv

    def render(self, tick, animation="idle", variants=(), palette=None):
        ids, uu, vv = self.render_ids(tick, animation, variants)
        pal = self.palettes[palette or next(iter(self.palettes))]
        H, W = ids.shape
        out = np.zeros((H, W, 4), np.uint8)
        for d in range(len(self.art)):
            m = ids == d
            if not m.any(): continue
            idx = self.art[d][vv[m], uu[m]]
            out[m, :3] = np.array(pal)[idx]; out[m, 3] = 255
        return out
