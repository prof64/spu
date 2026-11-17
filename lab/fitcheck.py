"""Does light alone reproduce the art? Light each sprite's flat colors from the art's own direction and count
how many pixels land on the shade the artist painted. Run with FIT=0 for the old facing data."""
import sys
import numpy as np
import bake
from recipes import R
from materials import M, D
def flat_levels(b, g, L):
    """The shade light alone picks for every pixel: its material's base shade, moved by how much it faces L."""
    L = np.array(L, float); L /= np.linalg.norm(L); Dn = g["n"] @ L
    out = g["lvl"].copy(); ok = np.zeros(Dn.shape, bool)
    for i, (_, lib, ramp, glows) in enumerate(b.rows):
        m = g["alpha"] & (g["mat"] == i)
        if i == 0 or len(ramp) < 2 or M[lib][9] <= 0 or not m.any(): continue
        up, dn = bake.reach(len(ramp), D[lib][1], b.base[i]); e = Dn[m] - bake.HEAD_ON
        out[m] = np.clip(np.round(b.base[i] + e * np.where(e > 0, up, dn)), -1, len(ramp)); ok |= m
    return out, ok
if __name__ == "__main__":
    tot = hit = 0
    for name in (sys.argv[1:] or list(R)):
        b = bake.Baked(name); g = b.gbuffer(0); lv, ok = flat_levels(b, g, bake.L0)
        h = int((lv[ok] == g["lvl"][ok]).sum()); n = int(ok.sum()); tot += n; hit += h
        print(f"{R[name]['title']:<22} {h:6d} of {n:6d} shaded pixels match the art  {100 * h / n:5.1f}%")
    print(f"{'all':<22} {hit:6d} of {tot:6d}  {100 * hit / tot:5.1f}%")
