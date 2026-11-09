# SPU version 1 — draft specification

SPU ("Sprite Universal", `.spu`) is an editable file for one animated 2D
character, object or effect: its drawings, how they are joined into parts,
how the parts move, its alternate looks, and optionally how light treats its
surfaces. This document is the draft of version 1. It is implemented by this
repository and checked by the [conformance suite](../conformance/README.md).
**Draft**: details may still change before version 1 is declared final.

The words *must*, *must not* and *may* are requirements on files and readers.

## Scope

**In scope**

- Pixel art and other raster drawings, full-color or palette-indexed.
- Frame-by-frame animation, articulated (cut-out) animation, and both mixed.
- Several views of one subject (front and back, eight walking directions).
- Alternate looks that reuse the same animation: recolors and replacement
  drawings (shiny, female, seasonal, eyes closed).
- Optional surface maps for lighting: normal, surface and emission.
- One exact, lighting-free picture for every tick, so any two tools can be
  compared pixel for pixel.
- Remaining editable: a file opened and saved by any tool keeps everything,
  including what that tool does not understand.

**Out of scope for version 1** (a file may carry these as unknown fields,
but no reader is required to act on them)

- Mesh deformation, bones separate from parts, constraints, physics.
- Blending between animations, state machines, events, sound.
- Vector art, text, video.
- Scenes, cameras, lights, weather and which way a view faces in a game.
- A compiled runtime form (packed tracks, texture atlases, compressed
  textures). SPU is the source; games may compile it however they like.
- Exactly how a lit picture looks. Materials are described; lighting models
  are the renderer's business.

## The file

A `.spu` file is a ZIP archive. It must contain `sprite.json` (UTF-8 JSON)
at its root. Every other member is addressed by its path; drawings are PNG.
Members must not be encrypted, and no path may leave the archive.

A reader must refuse a file that breaks any rule in this document rather
than guess. A reader must keep, and write back unchanged when saving, every
archive member and every JSON field it does not understand.

```jsonc
{
  "format": "spu",          // must be exactly this
  "version": 1,             // must be exactly this
  "name": "Example",
  "ticksPerSecond": 60,     // > 0
  "required": [],           // capabilities a reader must have; none exist yet
  "palettes": [ … ],        // optional
  "variants": [ … ],        // optional
  "materials": [ … ],       // optional
  "views": [ … ]            // at least one
}
```

A reader must refuse a `version` it does not implement and any `required`
entry it does not know. Readers may open older files by upgrading them in
memory (see "Older files"); they always save version 1.

Every palette, variant, material, view, drawing, part and animation has an
`id` (a non-empty string) and a `name` (shown to people, any text). Ids must
be unique within their list; drawings, parts and animations are unique
within their view. Everything refers to everything else by id.

## Views

```jsonc
{ "id": "front", "name": "Front",
  "canvas": {"x": -48, "y": -80, "width": 96, "height": 96},
  "images": [ … ], "parts": [ … ], "animations": [ … ] }
```

Sprite space has +X right and +Y down, in pixels. The canvas is the window
exported pictures show: `x`,`y` is the sprite-space point of its top-left
pixel. Width and height must be whole numbers from 1 to 4096; `x` and `y`
must be whole numbers. A view must have at least one animation.

### Drawings (`images`)

```jsonc
{ "id": "head", "name": "Head", "x": -12, "y": -30,
  "color": "images/0/3.png",
  "normal": "…", "surface": "…", "emission": "…",   // optional maps
  "material": "default",                             // optional
  "overlays": ["head-eyes-open"] }                   // optional
```

`x`,`y` place the image's top-left corner relative to the origin of the part
that shows it. The image's size is the PNG's size, 1 to 4096 pixels a side.

- **Color** is a PNG, 8 bits per channel or less. If the file has palettes and the PNG is
  palette-indexed, its pixels are indices into the active palette and the
  PNG's own colors are ignored (writers should store the first palette there
  so ordinary image viewers show something sensible). Otherwise its pixels
  are ordinary straight-alpha sRGB colors.
- **Maps**, when present, must be the same size as the color image. `normal`
  stores a direction as RGB (+X right, +Y up, +Z toward the viewer),
  `surface` stores roughness, metalness, transmission and thickness
  multipliers in R, G, B, A, and `emission` stores emitted color. Normal and
  surface are plain numbers; color and emission are sRGB.
- **Overlays** are other drawings of the same view painted straight over
  this one, in order, with the same placement. An overlay must not have
  overlays of its own.

### Parts

```jsonc
{ "id": "arm", "name": "Arm", "parent": "body" }
```

A part is one moving piece. It shows at most one drawing at a time. `parent`
is optional; parents must not form a loop. Order in the list matters only to
break draw-order ties.

### Animations

```jsonc
{ "id": "idle", "name": "Idle", "length": 192, "looping": true,
  "tracks": [ { "part": "arm",
                "image": [{"tick": 0, "value": "arm-down"}],
                "rotation": [{"tick": 0, "value": 0, "ease": "easeInOut"},
                             {"tick": 40, "value": 25}] } ] }
```

`length` is in ticks and must be greater than 0. An animation has at most
one track per part. A track's properties and their values when it has no
keys:

| Property | Value | Without keys |
| --- | --- | --- |
| `image` | drawing id, or `null` for none | none |
| `visible` | true / false | true |
| `x`, `y` | pixels, relative to the parent | 0 |
| `rotation` | degrees, clockwise on screen | 0 |
| `scaleX`, `scaleY` | factor; negative mirrors | 1 |
| `z` | draw order, higher is in front | 0 |
| `opacity` | 0 to 1 | 1 |
| `emission` | multiplier on emitted light | 1 |

Keys must be in strictly increasing `tick` order, at ticks ≥ 0. A key may
sit at or past the animation's length. Numbers must be finite and no larger
than 1,000,000 in size.

**Value at a tick.** Before a property's first key, the first key's value
holds. Otherwise take the last key at or before the tick. `image` and
`visible` hold that key's value. Numbers move toward the next key (if any)
by the earlier key's `ease`, with `t` the fraction of the way between the
two ticks:

| `ease` | amount moved |
| --- | --- |
| `step` (default) | 0 |
| `linear` | t |
| `easeIn` | t² |
| `easeOut` | t(2 − t) |
| `easeInOut` | t²(3 − 2t) |

**Playback.** At playback tick `T`, a looping animation shows tick
`T mod length`; any other shows `min(T, length − 1)`. Tools that show pixel
art should play at whole ticks.

## Variants and palettes

```jsonc
"palettes": [{"id": "normal", "name": "Normal", "colors": ["#00000000", "#d02020", …]}],
"variants": [{"id": "shiny", "name": "Shiny", "palette": "shiny"},
             {"id": "female", "name": "Female",
              "replacements": [{"view": "front", "image": "tail", "with": "tail-f"}]}]
```

A palette has 1 to 256 colors written `#rrggbb` or `#rrggbbaa`. Index 0
always draws fully transparent, whatever color is written there.

Any number of variants can be active at once, in a chosen order. The palette
in use is the one named by the last active variant that names one, else the
first palette. A drawing (or overlay) is replaced by `with` if an active
variant says so; when several do, the last wins. Replacements do not chain.

## Materials

```jsonc
{ "id": "glass", "name": "Glass", "roughness": 0.1, "metalness": 0,
  "transmission": 0.8, "ior": 1.45, "thickness": 1, "emission": 0,
  "normalStrength": 1, "opacity": 1, "reflectivity": 1 }
```

A drawing that names no material uses `default`, which need not be declared
and has the values a file gets by omitting every field: roughness 0.65,
metalness 0, transmission 0, ior 1.45, thickness 1, emission 0,
normalStrength 1, opacity 1, reflectivity 1. Roughness, metalness,
transmission and opacity are 0 to 1; ior is 1 to 3; the rest are 0 to 100.
Materials never affect the reference picture.

## The reference picture

For a view, an animation, a tick and a list of active variants, every
conforming renderer must be able to produce exactly this picture: straight
(not premultiplied) 8-bit RGBA, the size of the canvas, transparent where
nothing draws.

1. **Place each part.** A part's own placement is its `x`, `y`, `rotation`,
   `scaleX`, `scaleY` at the tick. A part with a parent combines with the
   parent's final placement: its position is carried through the parent's
   whole placement (scale, then rotation, then position); its rotation is
   added to the parent's, or subtracted if exactly one of the parent's
   scales is negative; its scales are multiplied by the parent's. Parts
   therefore never skew. After each combination, any of the five numbers
   within 10⁻⁹ of a multiple of 2⁻²⁰ becomes that multiple. Arithmetic is
   64-bit floating point.
2. **Choose what draws.** A part draws if it is visible and has a drawing.
   A parent passes on placement only: a hidden or faded parent does not hide
   or fade its children.
3. **Order.** Parts draw from lowest `z` to highest; equal `z` in the order
   the view lists its parts.
4. **Paint** the part's drawing (after replacements), then each overlay
   (after replacements). For every canvas pixel, take its center, map it
   back into the part's space, and subtract the drawing's `x`,`y`. A
   coordinate within 10⁻⁹ of a whole number counts as that number; otherwise
   round down. If that lands on a pixel of the image, that pixel's color
   (through the active palette if indexed) is blended "over" what is already
   there, with its alpha multiplied by the part's opacity clamped to 0–1.
   Indexed pixels of index 0, or beyond the palette's end, draw nothing. A
   part whose scale is 0 draws nothing.

There is no smoothing: one image pixel is chosen per canvas pixel.

Lit rendering (materials, maps, scene lights, bloom) is deliberately not
pinned down. With lighting switched off, a renderer must match the reference
picture.

## Limits

A reader may refuse files beyond these, and writers must stay within them:
64 MiB unpacked, 2048 archive members, 4 MiB manifest, 32 views, 256 parts
and 1024 drawings per view, 256 animations per view, 100,000 keys per
property, 256 palettes, variants and materials, drawings up to 4096 pixels a
side.

## Older files

- **SPU version 0** (Smeargle, `format: "spu"`, `version: 0`): entities were
  referred to by position and every drawing was indexed. Upgrading gives
  each entity an id made from its name, turns eye overlays into overlay
  drawings with an "Eyes closed" variant, and keeps every other member and
  field. The reference picture of the upgraded file is identical at every
  tick.
- **Sprite Lab prototype 0** (`format: "sprite-lab"`, `.sprite`): already
  the same shape; only the identity changes. One behavior changed with it:
  children no longer skew under an unevenly scaled parent, and no longer
  inherit a parent's visibility or opacity.

## Profiles (planned)

A profile is a named set of extra limits a file can be checked against so
that it fits a particular target. The first is **Gen 5**: what Pokémon
Black/White can hold (16-color palettes, tile and cell budgets, hardware
scales and angles), today enforced by Smeargle's checklist. Profiles limit
files; they never add meaning, so a file that passes a profile is still
ordinary SPU.
