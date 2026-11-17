"""Give each test sprite its lighting and write it as a real .spu file.

For every sprite in recipes.py: work out facing, material and glow for each drawing (bake.py), write
that as a lighting folder, and have the `spu` command convert the library's older file with it.
Results: out/spu/<id>.spu, and out/sprites.json, the list the page embeds."""
import sys, os, json, base64, io, shutil, subprocess
import numpy as np
from PIL import Image
from bake import Baked, LIB
from materials import D
from recipes import R

here = os.path.dirname(os.path.abspath(__file__))
SPU = os.environ.get("SPU_BIN") or os.path.join(here, "..", "target", "release", "spu")

def export(name):
    rec = R[name]
    front = Baked(name, "front"); views = [front]
    if "back" in rec.get("views", ["front", "back"]) and any(v["id"] == "back" for v in front.s.doc["views"]):
        views.append(Baked(name, "back", base=front.base))
    folder = os.path.join(here, "out", "light", name); shutil.rmtree(folder, ignore_errors=True)
    depth = {}
    for b in views:
        s = b.s; os.makedirs(os.path.join(folder, s.view["id"]))
        for d, art in enumerate(s.art):
            px = np.zeros(art.shape + (4,), np.uint8)
            px[..., 0] = np.clip(np.round(b.nrm[d][..., 0] * 127 + 128), 0, 255); px[..., 1] = np.clip(np.round(b.nrm[d][..., 1] * 127 + 128), 0, 255)
            px[..., 2] = b.mat[d]; px[..., 3] = np.round(b.glow[d] * 255)
            px[art == 0] = 0
            Image.fromarray(px, "RGBA").save(os.path.join(folder, s.view["id"], s.images[d]["id"] + ".png"))
        # a part that shows something glowing sits where the recipe says its glow is
        for clip in s.view["animations"]:
            for t in clip["tracks"]:
                if any(k["value"] and b.glow[s.index[k["value"]]].any() for k in t.get("image") or []): depth[t["part"]] = rec["glow_z"]
    rows = [dict(name=r[0], finish=D[r[1]][0], body=D[r[1]][1], shades=[int(c) for c in r[2]], base=int(front.base[i])) for i, r in enumerate(front.rows)]
    json.dump(dict(materials=rows, depth=depth), open(os.path.join(folder, "materials.json"), "w"), indent=1)
    src = os.path.join(os.path.dirname(LIB.rstrip("/")), rec.get("file", "black2-white2"), name + ".spu")
    out = os.path.join(here, "out", "spu", name + ".spu"); os.makedirs(os.path.dirname(out), exist_ok=True)
    subprocess.run([SPU, "convert", src, out, "--light", folder], check=True)
    # the picture on its button comes from the file itself
    png = os.path.join(here, "out", "light", name, "thumb.png"); subprocess.run([SPU, "render", out, png], check=True)
    im = Image.open(png).convert("RGBA"); im = im.crop(im.getbbox()); bio = io.BytesIO(); im.save(bio, "PNG", optimize=True)
    return dict(id=name, title=rec["title"], scene=rec["scene"], views=[b.s.view["id"] for b in views],
                thumb=base64.b64encode(bio.getvalue()).decode(), file=base64.b64encode(open(out, "rb").read()).decode()), os.path.getsize(src), os.path.getsize(out)

if __name__ == "__main__":
    names = sys.argv[1:] or list(R)
    done = [export(n) for n in names]
    json.dump(dict(sprites=[d[0] for d in done]), open(os.path.join(here, "out", "sprites.json"), "w"), separators=(",", ":"))
    for sp, before, after in done: print(f"  {sp['title']:<22} {before // 1024:5d} KB as it was  {after // 1024:5d} KB with lighting  views {sp['views']}")
    print(len(done), "sprites,", os.path.getsize(os.path.join(here, "out", "sprites.json")) // 1024, "KB for the page")
