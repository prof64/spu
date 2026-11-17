"""Bake the chosen sprites into one data blob for the viewer page."""
import sys, json, base64, zlib, io
import numpy as np
from PIL import Image
from bake import Baked
from materials import M, D, FIELDS
from recipes import R

def z64(a): return base64.b64encode(zlib.compress(np.ascontiguousarray(a).tobytes(), 9)).decode()

def pack(sizes, width=256):
    """Shelf-pack rectangles with a 1 pixel gap; returns positions and the sheet height."""
    order = sorted(range(len(sizes)), key=lambda i: -sizes[i][1])
    pos = [None] * len(sizes); x = y = 1; shelf = 0
    for i in order:
        w, h = sizes[i]
        if x + w + 1 > width: x = 1; y += shelf + 1; shelf = 0
        pos[i] = (x, y); x += w + 1; shelf = max(shelf, h)
    return pos, y + shelf + 1

def export_view(name, view):
    b = Baked(name, view); s = b.s; rec = b.rec
    sizes = [(a.shape[1], a.shape[0]) for a in s.art]
    pos, AH = pack(sizes); AW = 256
    color = np.zeros((AH, AW, 4), np.uint8); light = np.zeros((AH, AW, 4), np.uint8)
    drawings = []; glowinfo = []
    for d, art in enumerate(s.art):
        (x, y), (w, h) = pos[d], sizes[d]
        op = art != 0
        color[y:y + h, x:x + w, 0] = art; color[y:y + h, x:x + w, 1] = b.level[d]; color[y:y + h, x:x + w, 3] = s.opacity[d]                # the fourth channel is the pixel's own opacity
        n = b.nrm[d]
        light[y:y + h, x:x + w, 0] = np.clip(np.round(n[..., 0] * 127 + 128), 0, 255)
        light[y:y + h, x:x + w, 1] = np.clip(np.round(n[..., 1] * 127 + 128), 0, 255)
        light[y:y + h, x:x + w, 2] = b.mat[d]
        light[y:y + h, x:x + w, 3] = np.round(b.glow[d] * 255)
        im = s.images[d]
        drawings.append([x, y, w, h, im["x"], im["y"]])
        g = b.glow[d]; e = float(g.sum())
        if e > 0:
            ys, xs = np.nonzero(g > 0)
            cx = float((xs * g[ys, xs]).sum() / e) + 0.5 + im["x"]; cy = float((ys * g[ys, xs]).sum() / e) + 0.5 + im["y"]
            cols = {}
            for pid, pal in s.palettes.items():
                pa = np.array(pal, float) / 255
                cols[pid] = [round(float(v), 3) for v in (pa[art[ys, xs]] * g[ys, xs, None]).sum(0) / e]
            glowinfo.append((cx, cy, e, cols))
        else: glowinfo.append(None)
    def frames(clip):
        """Every distinct pose of one animation: the glow lights, then which drawing each part shows and where."""
        L = int(clip["length"])
        starts, offsets, buf, last = [], [], [], None
        for t in range(L):
            pl = s.placements(t, clip["id"])
            key = [(d, tuple(round(v, 6) for v in m), p) for d, m, op, p in pl if op > 0]
            if key == last: continue
            last = key; starts.append(t); offsets.append(len(buf))
            lights = []
            for d, m, _ in key:
                gi = glowinfo[d]
                if not gi: continue
                x = m[0] * gi[0] + m[2] * gi[1] + m[4]; y = m[1] * gi[0] + m[3] * gi[1] + m[5]
                lights.append([x, y, gi[2], d])
            lights.sort(key=lambda l: -l[2]); merged = []
            for l in lights:
                for q in merged:
                    if (q[0] - l[0]) ** 2 + (q[1] - l[1]) ** 2 < 18 ** 2:
                        w = q[2] + l[2]; q[0] = (q[0] * q[2] + l[0] * l[2]) / w; q[1] = (q[1] * q[2] + l[1] * l[2]) / w; q[2] = w; break
                else: merged.append(list(l))
            merged.sort(key=lambda l: -l[2]); merged = (merged + [[0, 0, 0, 0]] * 3)[:3]
            for l in merged: buf += l
            buf.append(len(key))
            for d, m, p in key: buf += [d] + list(m) + [p]          # drawing, placement, which part shows it
        keys = sum(len(t.get(k) or []) for t in clip["tracks"] for k in ("image", "x", "y", "rotation", "scaleX", "scaleY", "z", "visible"))
        return dict(id=clip["id"], name=NAMES[clip["id"]], length=L, keys=keys, starts=starts, offsets=offsets, frames=z64(np.array(buf, np.float32)))
    NAMES = {"idle": "Idle", "alert": "Alert", "alternate-1": "Alternate", "alternate-2": "Second alternate"}
    anims = sorted((frames(c) for c in s.view["animations"] if c["id"] in NAMES and (c["id"] == "idle" or c["length"] > 8)), key=lambda a: list(NAMES).index(a["id"]))
    g0 = b.gbuffer(0); ys, xs = np.nonzero(g0["alpha"])
    pal0 = np.array(s.palettes["normal"], np.uint8)
    thumb = np.zeros(g0["alpha"].shape + (4,), np.uint8); thumb[..., :3] = pal0[g0["idx"]]; thumb[..., 3] = g0["alpha"] * 255
    thumb = Image.fromarray(thumb[ys.min():ys.max() + 1, xs.min():xs.max() + 1]); bio = io.BytesIO(); thumb.save(bio, "PNG", optimize=True)
    rows = []
    for i, (rn, lib, ramp, glows) in enumerate(b.rows):
        rows.append(dict(name=rn, lib=lib, ramp=ramp, base=b.base[i], glow=bool(glows), f=D[lib][0], b=D[lib][1], p=[float(v) for v in M[lib][:9]]))
    blink = {}                                               # the eyes-closed look: which drawing replaces which
    for v in s.doc.get("variants", []):
        if v["id"] != "eyes-closed": continue
        for r in v.get("replacements", []):
            if r["view"] == view: blink[s.index[r["image"]]] = s.index[r["with"]]
    looks = []                                               # every other look: a palette, drawing swaps, or both
    for v in s.doc.get("variants", []):
        if v["id"] == "eyes-closed": continue
        swaps = {s.index[r["image"]]: s.index[r["with"]] for r in v.get("replacements", []) if r["view"] == view}
        # what is painted over a drawing (its eyes) goes with it: use the replacement's own, not the original's.
        # Version 1's reference picture keeps the original's, which puts male eyes on a female Jellicent.
        for a_, b_ in list(swaps.items()):
            for oa, ob in zip(s.images[a_].get("overlays") or [], s.images[b_].get("overlays") or []): swaps.setdefault(s.index[oa], s.index[ob])
        if v.get("palette") or swaps: looks.append(dict(id=v["id"], name=v.get("name") or v["id"].title(), palette=v.get("palette"), swaps=swaps))
    used = sorted({int(i) for a in s.art for i in np.unique(a) if i})
    soft = any(((o > 0) & (o < 250)).any() for o in s.opacity)               # any pixel that is partly see-through
    out = dict(canvas=list(s.canvas), colors=len(used), soft=soft,
                bbox=[int(xs.min()) + s.canvas[0], int(ys.min()) + s.canvas[1], int(xs.max()) + 1 + s.canvas[0], int(ys.max()) + 1 + s.canvas[1]],
                atlas=[AW, AH], color=z64(color), light=z64(light), drawings=drawings, names=[im["id"] for im in s.images],
                parts=[p["id"] for p in s.view["parts"]], blink=blink, looks=looks, anims=anims,
                glowCols=[g[3] if g else None for g in glowinfo], rows=rows, tps=s.doc["ticksPerSecond"])
    return out, base64.b64encode(bio.getvalue()).decode(), s

def export(name):
    rec = R[name]
    front, thumb, s = export_view(name, "front")
    views = {"front": front}
    if "back" in rec.get("views", ["front", "back"]) and any(v["id"] == "back" for v in s.doc["views"]): views["back"] = export_view(name, "back")[0]
    n = 1 + max(i for v in s.rgba.values() for i, c in enumerate(v) if c[3] or i == 0)      # drop the unused tail of each palette
    return dict(id=name, title=rec["title"], scene=rec["scene"], ground=rec["ground"], glowZ=rec["glow_z"],
                palettes={k: [list(c) for c in v[:n]] for k, v in s.rgba.items()}, views=views, thumb=thumb)

if __name__ == "__main__":
    names = sys.argv[1:] or list(R)
    data = dict(fields=FIELDS, sprites=[export(n) for n in names])
    json.dump(data, open("out/data.json", "w"), separators=(",", ":"))
    import os; print(len(names), "sprites,", os.path.getsize("out/data.json") // 1024, "KB")
    for sp in data["sprites"]: print(" ", sp["id"], len(sp["palettes"]), "palettes", {k: (v["atlas"], len(v["parts"]), "parts", [(a["id"], a["length"]) for a in v["anims"]], [l["id"] for l in v["looks"]][:4]) for k, v in sp["views"].items()})
