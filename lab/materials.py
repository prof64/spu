"""Materials as two dials, and how the baker shapes each surface.

finish: -1 fuzzy .. 0 matte .. +1 polished      (what the surface is like)
body:   -1 metal .. 0 solid .. 0.5 translucent .. +1 clear   (what happens to light that gets past the surface)

The lighting still works on six internal values; values() derives them, so nothing else is stored.
bulge, tilt and facet are authoring aids for the surface map, not part of a material.
"""
def values(finish, body):
    fuzz = max(0.0, -finish); gloss = max(0.0, finish)
    metal = max(0.0, -body); c = max(0.0, body)
    through = max(0.0, 2 * c - 1) * 0.85                 # seen straight through
    inside = (1 - abs(2 * c - 1)) if c > 0 else 0.0     # scattered inside and glowing back out; strongest half way
    #        rough        metal  see-through light-through coat fuzz  bend depth film
    return (1.0 - gloss, metal, through,     inside,       0.0, fuzz, 1.4, 0.5,  0.0)

#               finish body | bulge tilt facet
D = {
 "outline":   ( 0.10, 0.00,  1.0, 0.0, 0),
 "scales":    ( 0.50, 0.05,  1.0, 0.9, 0),
 "soft skin": ( 0.40, 0.25,  1.0, 0.8, 0),
 "hide":      ( 0.20, 0.02,  1.0, 0.9, 0),
 "membrane":  ( 0.45, 0.50,  0.35,0.9, 0),
 "horn":      ( 0.70, 0.05,  1.0, 0.7, 0),
 "mouth":     ( 0.80, 0.20,  0.6, 0.5, 0),
 "eye":       ( 0.90, 0.00,  0.8, 0.3, 0),
 "flame":     ( 0.00, 0.00,  0.3, 0.0, 0),
 "steel":     ( 0.70,-1.00,  1.0, 0.5, 0),
 "dark iron": ( 0.45,-1.00,  1.0, 0.6, 0),
 "paint":     ( 0.80, 0.00,  1.0, 0.6, 0),
 "ice":       ( 0.94, 0.82,  0.8, 1.0, 1),
 "frost":     ( 0.65, 0.65,  0.9, 1.0, 0.6),
 "fur":       (-0.70, 0.05,  1.0, 0.8, 0),
 "plasma":    ( 0.75, 0.30,  1.0, 0.7, 0),
 "aura":      ( 0.80, 0.95,  0.5, 0.5, 0),
 "glass":     ( 0.97, 1.00,  1.0, 0.6, 0),
 "spectral":  ( 0.55, 0.30,  1.0, 0.5, 0),     # a ghost ship's hull
 "mist":      ( 0.00, 0.00,  0.0, 0.0, 0),     # drawn soft and see-through already; has no surface of its own
 "nuclear":   ( 0.75, 0.30,  1.0, 0.5, 0),
 "needles":   (-0.45, 0.05,  1.0, 0.5, 0),
 "gold":      ( 0.80,-1.00,  1.0, 0.5, 0),
 "jelly":     ( 0.85, 0.50,  1.0, 0.6, 0),
 "fluff":     (-1.00, 0.20,  1.0, 0.6, 0),
 "leaf":      ( 0.40, 0.40,  0.6, 0.6, 0),
 "gem":       ( 0.90, 0.30,  0.8, 0.3, 0),
}
M = {k: values(v[0], v[1]) + v[2:] for k, v in D.items()}
FIELDS = ["finish", "body"]
