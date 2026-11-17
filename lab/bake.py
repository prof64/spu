"""Author the lighting data for a sprite and bake everything the viewer needs.

For every drawing this makes the two things the format stores per pixel
beyond color: a surface direction (normal) and a material number, plus glow.
Normals come from three cues: the drawing's own outline (inflated like a balloon),
the line art inside it (creases), and the shading the artist already painted
(lighter than the base color leans toward the art's light, darker leans away).
"""
import sys, os, json, base64, io, fnmatch, colorsys
import numpy as np
from PIL import Image
from scipy import ndimage, sparse
from scipy.sparse.linalg import spsolve
from spu import Sprite
from materials import M, D, FIELDS
from recipes import R

# the sprite library: a checkout of https://github.com/prof64/pku-spu
LIB = os.path.join(os.path.expanduser(os.environ.get("SPU_LIBRARY") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "pk-mono", "pku-spu")), "black2-white2", "")
L0 = np.array([-0.5, 0.6, 0.62]); L0 /= np.linalg.norm(L0)      # the light the art was painted under (x right, y up, z out)
L0_2D = L0[:2] / np.linalg.norm(L0[:2])
SOFT_TILT = float(os.environ.get('SOFT_TILT', 0.0))
FIT = float(os.environ.get('FIT', 1.0))        # how far the facing data is refit to the painted shading: 0 not at all, 1 fully

HEAD_ON = 0.6      # how much a surface facing the viewer head-on faces the art's light; it shows the base shade

def reach(n, body, base):
    """Shades per unit of facing, toward the light and away from it (the same sum as the shader). A surface
    facing the viewer head-on shows its base shade; every lighter and darker shade has room either side."""
    t = min(max((n - 3) / 6, 0), 1); cl = max(0.0, body); ss = 1 - abs(2 * cl - 1) if cl > 0 else 0.0
    g = max(2.0, (n - 1) * (1 - 0.55 * t * t * (3 - 2 * t))) * (1 - 0.45 * ss)
    return max(g, (n - 0.5 - base) / 0.32), max(g, (base + 0.5) / 1.2)

def poisson(mask):
    """u with laplacian(u) = -4 inside the mask and 0 outside: sqrt(u) is a hemisphere over a disc.
    The mask must have an empty border."""
    out = np.zeros(mask.shape)
    ys, xs = np.nonzero(mask); n = len(ys)
    if n == 0: return out
    idx = -np.ones(mask.shape, np.int64); idx[ys, xs] = np.arange(n)
    rows, cols, vals = [np.arange(n)], [np.arange(n)], [np.full(n, 4.0)]
    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
        j = idx[ys + dy, xs + dx]; k = j >= 0
        rows.append(np.arange(n)[k]); cols.append(j[k]); vals.append(-np.ones(int(k.sum())))
    A = sparse.coo_matrix((np.concatenate(vals), (np.concatenate(rows), np.concatenate(cols))), shape=(n, n)).tocsc()
    out[ys, xs] = np.maximum(spsolve(A, np.full(n, 4.0)), 0)
    return out

def inflate(opaque, cells):
    """Slopes (x right, y up) of a surface blown up from a picture: the whole outline gives the
    overall roundness, and each area enclosed by drawn lines swells on its own, so lines become creases."""
    pad = lambda m: np.pad(m, 1)
    H = 0.35 * np.sqrt(poisson(pad(opaque))) + 0.65 * np.sqrt(poisson(pad(cells)))
    gy, gx = np.gradient(H)
    return -gx[1:-1, 1:-1], gy[1:-1, 1:-1]

def auto_rows(s, cfg):
    """Build the material rows from a few named colors instead of a full list.

    Each surface is named by one or more sample colors (or by which drawings it is). Every color the
    sprite uses joins the surface whose sample it most resembles, and each surface's shades are then
    put in order, dark to light. This is what an editor would do after the artist clicks a color and
    picks a material; with a few hundred colors nobody will list shades by hand."""
    pal = np.array(s.rgba["normal"], float)
    hexrgb = lambda h: np.array([int(h[i:i + 2], 16) for i in (0, 2, 4)], float)
    lines = [hexrgb(h) for h in cfg.get("outline", [])]
    def outline(i):
        c = pal[i]
        return (c[:3].max() <= cfg.get("dark", 46) and c[3] >= 250) or any(np.abs(c[:3] - o).max() <= 6 for o in lines)
    def dist(c, a):
        h1, s1, v1 = colorsys.rgb_to_hsv(*(c[:3] / 255)); h2, s2, v2 = colorsys.rgb_to_hsv(*(a / 255))
        dh = min(abs(h1 - h2), 1 - abs(h1 - h2)) * 2
        return dh * min(s1, s2) * 3.0 + abs(s1 - s2) + abs(v1 - v2) * 0.35      # hue only counts if both are colorful
    fams = cfg["families"]
    ladder = {"outline": set(), **{f["name"]: set() for f in fams}}
    rules = []
    for d, art in enumerate(s.art):
        did = s.images[d]["id"]
        by_drawing = next((f["name"] for f in fams if any(fnmatch.fnmatch(did, g) for g in f.get("drawings", []))), None)
        mp = {}
        for i in np.unique(art):
            i = int(i)
            if i == 0: continue
            if outline(i): t = "outline"
            elif by_drawing: t = by_drawing
            else: t = min((f for f in fams if f.get("anchors")), key=lambda f: min(dist(pal[i], hexrgb(a)) for a in f["anchors"]))["name"]
            ladder[t].add(i); mp[i] = t
        rules.append((did, mp))
    light = lambda i: (0.3 * pal[i][0] + 0.5 * pal[i][1] + 0.2 * pal[i][2]) * (0.5 + 0.5 * pal[i][3] / 255)
    rows = [("outline", "outline", sorted(ladder["outline"], key=light), None)]
    for f in fams:
        ramp = sorted(ladder[f["name"]], key=light)
        if not ramp: continue
        g = f.get("glow")
        rows.append((f["name"], f["lib"], ramp, [g[0] + (g[1] - g[0]) * k / max(len(ramp) - 1, 1) for k in range(len(ramp))] if g else None))
    return rows, rules

def blur_in(mask, a, sigma):
    w = ndimage.gaussian_filter(mask.astype(float), sigma)
    return np.where(mask, ndimage.gaussian_filter(np.where(mask, a, 0.0), sigma) / np.maximum(w, 1e-6), a)

class Baked:
    def __init__(self, name, view="front", base=None):
        rec = R[name]
        self.s = s = Sprite(os.path.join(os.path.dirname(LIB.rstrip("/")), rec.get("file", "black2-white2"), name + ".spu"), view)
        if "auto" in rec:
            rows, rules = auto_rows(s, rec["auto"]); rec = dict(rec, rows=rows, rules=rules)
        self.name = name; self.rec = rec
        rows = rec["rows"]; self.rows = rows
        self.row_index = {r[0]: i for i, r in enumerate(rows)}
        default = {}
        for i, (_, _, ramp, _) in enumerate(rows):
            for c in ramp: default.setdefault(c, i)
        self.mat, self.level, self.nrm, self.glow = [], [], [], []
        counts = np.zeros((len(rows), 256), np.int64)
        for d, art in enumerate(s.art):
            did = s.images[d]["id"]
            mat = np.zeros(art.shape, np.uint8)
            for c in np.unique(art):
                if c == 0: continue
                row = default.get(int(c))
                for pat, mp in rec.get("rules_" + view, []) + rec["rules"]:
                    if fnmatch.fnmatch(did, pat) and int(c) in mp:
                        row = self.row_index[mp[int(c)]]; break
                assert row is not None, (name, did, int(c), "color not in any row")
                mat[art == c] = row
            lvl = np.zeros(art.shape, np.uint8)
            for i, (_, _, ramp, _) in enumerate(rows):
                for k, c in enumerate(ramp):
                    m = (mat == i) & (art == c); lvl[m] = k; counts[i, k] += int(m.sum())
            self.mat.append(mat); self.level.append(lvl)
        # the base shade of each row: the one most of its pixels use
        self.base = [int(np.argmax(counts[i, :len(r[2])])) for i, r in enumerate(rows)]
        for i, r in enumerate(rows):
            if len(r) > 4: self.base[i] = r[4]
        if base is not None: self.base = list(base)          # a sprite has one material table: later views use the first view's
        self.nrm = [None] * len(s.art); self.glow = [None] * len(s.art)
        for d, (px, py) in enumerate(self.surface()):
            self.nrm[d], self.glow[d] = self.finish(d, px, py)

    def surface(self):
        """Slopes for every drawing, taken from the whole sprite as it is actually assembled.

        Shaping each drawing alone rounds off edges that are not edges at all: where a neck runs into a
        body, or an eye patch sits on a head. So the sprite is posed, the assembled picture is inflated
        (only transparency and drawn lines count as edges), and each drawing takes the piece that fell on
        it. Texels that were covered by another part copy their nearest uncovered neighbor."""
        s = self.s; L = int(s.clip()["length"])
        best = {}
        for t in range(0, L, 6):                         # the tick that shows the most of each drawing
            ids, _, _ = s.render_ids(t)
            cnt = np.bincount(ids[ids >= 0], minlength=len(s.art))
            for d in np.nonzero(cnt)[0]:
                if cnt[d] > best.get(d, (0, 0))[0]: best[d] = (int(cnt[d]), t)
        out = [None] * len(s.art)
        for t in sorted({t for _, t in best.values()}):
            ids, uu, vv = s.render_ids(t); inst = s.inst
            mat = np.zeros(ids.shape, np.int32)
            for k, (d, m, op, p) in enumerate(s.placed):
                sel = inst == k; mat[sel] = self.mat[d][vv[sel], uu[sel]]
            body = np.array([M[r[1]][9] > 0 for r in self.rows])                 # mist and the like have no shape to blow up
            opaque = (ids >= 0) & body[mat]
            # a drawn line is an edge only if it is part of the outline; a pupil, a nostril or a
            # wrinkle standing on its own is a mark on the surface and must not dent it
            line = opaque & (mat == 0)
            lab, nl = ndimage.label(line, structure=np.ones((3, 3)))
            rim = ndimage.binary_dilation(~opaque, structure=np.ones((3, 3)))
            edge = np.zeros(ids.shape, bool)
            for q in range(1, nl + 1):
                c = lab == q
                if c.sum() >= 14 or (c & rim).any(): edge |= c
            sx, sy = inflate(opaque, opaque & ~edge)
            sx[~opaque] = 0; sy[~opaque] = 0
            acc = {}
            for k, (d, m, op, p) in enumerate(s.placed):
                sel = inst == k
                if best.get(d, (0, -1))[1] != t or not sel.any(): continue
                lin = np.array([[m[0], m[2]], [m[1], m[3]]]); sc = np.sqrt(abs(np.linalg.det(lin))) or 1.0
                inv = np.linalg.inv(lin) * sc                                   # back into the drawing's own frame
                cx, cy = sx[sel], -sy[sel]
                a = acc.setdefault(d, [np.zeros(s.art[d].shape), np.zeros(s.art[d].shape), np.zeros(s.art[d].shape)])
                np.add.at(a[0], (vv[sel], uu[sel]), inv[0, 0] * cx + inv[0, 1] * cy)
                np.add.at(a[1], (vv[sel], uu[sel]), -(inv[1, 0] * cx + inv[1, 1] * cy))
                np.add.at(a[2], (vv[sel], uu[sel]), 1)
            for d, (ax, ay, n) in acc.items():
                known = n > 0
                iy, ix = ndimage.distance_transform_edt(~known, return_distances=False, return_indices=True)
                px = (ax / np.maximum(n, 1))[iy, ix]; py = (ay / np.maximum(n, 1))[iy, ix]
                out[d] = (px, py)
        # a drawing only a variant shows (eyes closed) sits where the one it replaces sits
        for v in s.doc.get("variants", []):
            for r in v.get("replacements", []):
                if r["view"] != s.view["id"]: continue
                src, dst = s.index[r["image"]], s.index[r["with"]]
                if out[dst] is not None or out[src] is None: continue
                dx = int(s.images[dst]["x"] - s.images[src]["x"]); dy = int(s.images[dst]["y"] - s.images[src]["y"])
                h, w = s.art[dst].shape; sh, sw = s.art[src].shape
                yy = np.clip(np.arange(h)[:, None] + dy, 0, sh - 1); xx = np.clip(np.arange(w)[None, :] + dx, 0, sw - 1)
                out[dst] = (out[src][0][yy, xx], out[src][1][yy, xx])
        for d, art in enumerate(s.art):                  # never shown at all: shape it by itself
            if out[d] is None: out[d] = inflate(art != 0, (art != 0) & (self.mat[d] != 0))
        return out

    def finish(self, d, px, py):
        art, mat, lvl = self.s.art[d], self.mat[d], self.level[d]
        opaque = art != 0
        cells = opaque & (mat != 0)
        bulge = np.zeros(art.shape); tilt = np.zeros(art.shape); facet = np.zeros(art.shape); glow = np.zeros(art.shape)
        delta = np.zeros(art.shape)
        for i, (_, libname, ramp, glows) in enumerate(self.rows):
            m = opaque & (mat == i)
            if not m.any(): continue
            lib = M[libname]
            bulge[m] = lib[9] * self.rec.get("bulge", {}).get(self.rows[i][0], 1.0)
            tilt[m] = lib[10]; facet[m] = lib[11]
            n = max(len(ramp) - 1, 1)
            delta[m] = (lvl[m].astype(float) - self.base[i]) / n
            if glows: glow[m] = np.array(glows)[lvl[m]]
            # a stroke one pixel wide (a wrinkle, a brow) is a drawn line, not a slope: it stays a line
            for sign in (1, -1):
                side = m & (delta * sign > 0)
                broad = ndimage.binary_opening(side, structure=np.ones((2, 2)))
                delta[side & ~broad] = 0
        # painted shading only tilts faceted surfaces (each flat patch turns as one); on rounded ones it
        # stays as the artist drew it and the light slides over the smooth shape underneath
        lean = np.where(facet > 0, 0.6 * (1 - FIT), SOFT_TILT) * tilt * delta
        px = px * bulge + lean * L0_2D[0]
        py = py * bulge + lean * L0_2D[1]
        smooth = cells & (facet < 0.5)
        if smooth.any():
            px = np.where(smooth, blur_in(cells, px, 0.9), px); py = np.where(smooth, blur_in(cells, py, 0.9), py)
        fm = cells & (facet > 0)
        if fm.any():
            for c in np.unique(art[fm]):                                   # one facet = touching pixels of one color
                lab2, n2 = ndimage.label(fm & (art == c))
                for q in range(1, n2 + 1):
                    rr = lab2 == q; f = facet[rr].mean()
                    px[rr] = px[rr] * (1 - f) + px[rr].mean() * f; py[rr] = py[rr] * (1 - f) + py[rr].mean() * f
        ln = np.sqrt(px * px + py * py + 1)
        N = np.stack([px / ln, py / ln, 1 / ln], -1)
        if FIT > 0:
            # Refit to the painted shading. The artist's shades say how much each pixel faces the art's own
            # light (upper left). Keep the blown-up shape for everything else, but turn each pixel toward or
            # away from that light until light alone would pick the shade the artist painted.
            up = np.array([0, 0, 1.0]) - L0[2] * L0; up /= np.linalg.norm(up)
            for i, (_, libname, ramp, glows) in enumerate(self.rows):
                m = opaque & (mat == i)
                if i == 0 or len(ramp) < 2 or M[libname][9] <= 0 or not m.any(): continue
                up_, dn_ = reach(len(ramp), D[libname][1], self.base[i])
                step = lvl[m].astype(float) - self.base[i]
                edge = lambda k: HEAD_ON + np.where(k > 0, k / up_, k / dn_)       # facing at which shade k begins
                lo, hi = edge(step - 0.5), edge(step + 0.5); pad = 0.15 * (hi - lo)
                have = N[m] @ L0
                # leave a pixel alone if the blown-up shape already gives its painted shade; otherwise turn it
                # just far enough that it does
                d = have + (np.clip(have, lo + pad, hi - pad) - have) * FIT
                side = N[m] - have[:, None] * L0; sl = np.linalg.norm(side, axis=1, keepdims=True)
                side = np.where(sl > 1e-4, side / np.maximum(sl, 1e-4), up)
                fit = np.sqrt(1 - d * d)[:, None] * side + d[:, None] * L0
                low = fit[:, 2] < 0.05                                  # never let a pixel face away from the viewer
                fit[low] = np.sqrt(1 - d[low] ** 2)[:, None] * up + d[low][:, None] * L0
                N[m] = fit
        n = N[..., :2].copy()
        n[~opaque] = 0
        return n, glow

    def gbuffer(self, tick, palette="normal"):
        s = self.s
        ids, uu, vv = s.render_ids(tick)
        H, W = ids.shape
        g = dict(alpha=ids >= 0, idx=np.zeros((H, W), np.int32), mat=np.zeros((H, W), np.int32),
                 lvl=np.zeros((H, W), np.int32), n=np.zeros((H, W, 3)), glow=np.zeros((H, W)))
        g["n"][..., 2] = 1
        for k, (d, m, op, _) in enumerate(s.placed):
            sel = s.inst == k
            if not sel.any(): continue
            u, v = uu[sel], vv[sel]
            g["idx"][sel] = s.art[d][v, u]; g["mat"][sel] = self.mat[d][v, u]; g["lvl"][sel] = self.level[d][v, u]
            g["glow"][sel] = self.glow[d][v, u]
            nx, ny = self.nrm[d][v, u, 0], -self.nrm[d][v, u, 1]         # to y-down for the part matrix
            rx, ry = m[0] * nx + m[2] * ny, m[1] * nx + m[3] * ny
            sc = np.sqrt(abs(m[0] * m[3] - m[1] * m[2])) or 1.0
            rx, ry = rx / sc, -ry / sc
            l2 = np.minimum(rx * rx + ry * ry, 0.999)
            g["n"][sel] = np.stack([rx, ry, np.sqrt(1 - l2)], -1)
        return g

    def ramp_colors(self, palette="normal"):
        """Per row: [one shade darker, its shades..., one shade lighter]."""
        pal = np.array(self.s.palettes[palette], float) / 255
        out = []
        for _, _, ramp, _ in self.rows:
            c = [pal[i] for i in ramp]
            under = c[0] * np.array([0.62, 0.60, 0.72]) if len(c) == 1 else np.clip(c[0] - (c[1] - c[0]) * 0.8, 0, 1) * 0.9
            over = c[-1] + (1 - c[-1]) * 0.35
            out.append([under] + c + [over])
        return out

    def relight(self, g, L, gain=1.1, palette="normal"):
        """Repaint under a distant light, staying on each material's own shades."""
        L = np.array(L, float); L /= np.linalg.norm(L)
        ramps = self.ramp_colors(palette)
        D = g["n"] @ L; D0 = g["n"] @ L0
        H, W = D.shape
        out = np.zeros((H, W, 3))
        for i, (_, _, ramp, _) in enumerate(self.rows):
            m = g["alpha"] & (g["mat"] == i)
            if not m.any(): continue
            n = len(ramp)
            k = g["lvl"][m] + (0 if i == 0 else gain * max(n - 1, 2) * (D[m] - D0[m]))
            k = np.clip(np.round(k), -1, n).astype(int) + 1
            out[m] = np.array(ramps[i])[k]
        glow = g["glow"][..., None]
        pal = np.array(self.s.palettes[palette], float) / 255
        out = out * (1 - glow) + pal[g["idx"]] * glow
        return out

def sheet(b, ticks=(0,), scale=3):
    rows = []
    for t in ticks:
        g = b.gbuffer(t)
        pal = np.array(b.s.palettes["normal"], float) / 255
        orig = pal[g["idx"]]
        nv = np.stack([g["n"][..., 0] * .5 + .5, g["n"][..., 1] * .5 + .5, g["n"][..., 2] * .5 + .5], -1)
        tiles = [orig, nv]
        for L in [(-0.5, 0.6, 0.62), (0.8, 0.3, 0.5), (-0.9, -0.1, 0.4), (0, 0.9, 0.4), (0.3, -0.8, 0.5), (0, 0, 1)]:
            tiles.append(b.relight(g, L))
        bg = np.array([0.22, 0.22, 0.26])
        tiles = [np.where(g["alpha"][..., None], t_, bg) for t_ in tiles]
        rows.append(np.concatenate(tiles, 1))
    im = np.concatenate(rows, 0)
    im = Image.fromarray((np.clip(im, 0, 1) * 255).astype(np.uint8))
    return im.resize((im.width * scale, im.height * scale), Image.NEAREST)

if __name__ == "__main__":
    for name in sys.argv[1:]:
        b = Baked(name)
        sheet(b, ticks=(0,), scale=2).save(f"out/relight-{name}.png")
        print(name, "base shades", b.base)
