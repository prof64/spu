"""Pack the demo's sprites into out/sprites.json, the list the page embeds.

Each sprite is a lit .spu file, out/spu/<id>.spu; set SPU_SPRITES to read them from another folder.
The picture on each sprite's button is drawn from its file by the spu command (SPU_BIN, default
../target/release/spu)."""
import sys, os, json, base64, io, subprocess, tempfile
from PIL import Image

here = os.path.dirname(os.path.abspath(__file__))
SPU = os.environ.get("SPU_BIN") or os.path.join(here, "..", "target", "release", "spu")
FOLDER = os.environ.get("SPU_SPRITES") or os.path.join(here, "out", "spu")

# id: its title, the place it opens in, and the views the page shows (None: every view in the file)
SPRITES = {
    "0006-charizard": ("Charizard", "cave", None),
    "0082-magneton": ("Magneton", "rain", None),
    "0615-cryogonal": ("Cryogonal", "cave", None),
    "0646-kyurem": ("Kyurem", "rain", None),
    "0479-rotom": ("Rotom", "cave", None),
    "0059-arcanine": ("Arcanine", "rain", None),
    "0609-chandelure": ("Chandelure", "cave", None),
    "0493-arceus": ("Arceus", "field", None),
    "0593-jellicent": ("Jellicent", "deep", None),
    "0601-klinklang": ("Klinklang", "cave", None),
    "0547-whimsicott": ("Whimsicott", "field", None),
    "0579-reuniclus": ("Reuniclus", "studio", None),
    "117-titanice-form-03-halloween": ("Halloween Titanice", "cave", ["front"]),
    "028-gyarados-nuclear-combined": ("Nuclear Gyarados", "cave", ["front"]),
    "002-metalynx": ("Metalynx", "field", ["front"]),
    "082-corsoreef-form-03-christmas": ("Christmas Corsoreef", "snow", ["front"]),
}

def pack(name):
    title, scene, views = SPRITES[name]
    path = os.path.join(FOLDER, name + ".spu")
    info = subprocess.run([SPU, "info", path], check=True, capture_output=True, text=True).stdout
    views = views or [line.split()[1] for line in info.splitlines() if line.startswith("view ")]
    with tempfile.TemporaryDirectory() as tmp:
        png = os.path.join(tmp, "thumb.png"); subprocess.run([SPU, "render", path, png], check=True)
        im = Image.open(png).convert("RGBA"); im = im.crop(im.getbbox())
    bio = io.BytesIO(); im.save(bio, "PNG", optimize=True)
    return dict(id=name, title=title, scene=scene, views=views, thumb=base64.b64encode(bio.getvalue()).decode(), file=base64.b64encode(open(path, "rb").read()).decode())

if __name__ == "__main__":
    names = sys.argv[1:] or list(SPRITES)
    out = os.path.join(here, "out", "sprites.json"); os.makedirs(os.path.dirname(out), exist_ok=True)
    json.dump(dict(sprites=[pack(n) for n in names]), open(out, "w"), separators=(",", ":"))
    print(len(names), "sprites,", os.path.getsize(out) // 1024, "KB for the page")
