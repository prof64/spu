# SPU version 2 — proposal draft

**Status: proposal for review.** The file layout is not implemented yet. The
color, lighting and material design below was tried on eleven sprites in
[`lab/`](../lab/README.md), which reads version 1 files; "Tried and not tried"
at the end says exactly what that covers. This replaces [version 1](spec.md)
once agreed; the shape of the sprite (parts, drawings, animations, palettes,
variants, materials, the reference picture) carries over except where this
draft says otherwise.

## Why change

Version 1 is a ZIP of pretty-printed JSON plus one PNG per drawing. It is easy
to poke at but slow to load: Charizard's manifest is 700 KB, almost all of it
animation keys written as text and indentation, and the library's manifests
total about 360 MB. Games want one file they can read straight through.

## Two forms of one sprite

| Form | Extension | For | Contents |
| --- | --- | --- | --- |
| **Binary** | `.spu` | games, the browser, anything that ships | one file of chunks (below) |
| **Folder** | a folder holding `sprite.json` | artists, agents, debugging | `sprite.json`, animations as JSON, each drawing as its own PNG |

Both hold exactly the same sprite. The library converts either way without
loss: `spu unpack` turns a `.spu` into a folder to edit with any tools, and
`spu pack` turns it back. In the folder each drawing is its own PNG (with its
maps beside it), because that is what people edit; packing builds the sheets.
Editors may open both; everything that ships uses `.spu`. There is no zipped
form: a folder is what anyone would unzip to anyway.

## The sprite

Everything below is defined once for the whole sprite. A view (Front, Back,
eight walking directions) is an angle on the same sprite, not a separate one.

- **Basics**: `name`, `author`, `version` and ticks per second. `version` is
  the sprite's own version, a whole number from 1 that goes up each time the
  sprite is published. It is not the format version in the file header.
- **Views**: id, name, canvas (x, y, width, height).
- **Parts**: id, name, parent, and optionally `depth`. One list for all views.
  A part with nothing to show in a view simply has no drawing there; nothing
  is stored for it.
- **Drawings**: id and name, shared by all views ("arm-down" means the same
  pose everywhere). Each view gives the drawing its own picture (a rectangle
  on that sprite's sheets, plus an offset) or none.
- **Animations**: id, name, length in ticks, looping. One list for all views.
  Inside, each view has its own tracks, so the motion can differ per angle,
  but every view has every animation at the same length.
- **Palettes, variants, materials**: as below.

### Tracks and keys

A track animates one property of one part in one view. Properties: `image`
(drawing or none), `x`, `y`, `rotation`, `scaleX`, `scaleY`, `z`, `opacity`,
`emission`. Changes from version 1:

- Ticks are whole numbers (0 to 2³²−1).
- `visible` is gone: a part with no drawing at a tick does not draw.
- A key that repeats the previous value with the same ease is not stored.

Easing (`step`, `linear`, `easeIn`, `easeOut`, `easeInOut`) and the value at a
tick are unchanged from version 1.

### Overlays are gone

Version 1 let a drawing carry overlays (eyes painted over a head). Each overlay
becomes its own part, parented to the part it sat on, one `z` step above it.

### Pictures: two sheets per view

Each view's drawings are packed onto one **Color** sheet for that view, so a
game can load only the views it uses. A sprite with lighting has a second
sheet, **Light**, with the identical layout, so one rectangle finds a drawing's
color and its lighting. Every pixel holds six numbers:

| Sheet | Holds per pixel |
| --- | --- |
| Color | color number · opacity |
| Light | facing, left to right · facing, bottom to top · material number · glow |

- **Color number** picks a color from the active palette. This is the only
  way a pixel gets its color; there is no full-color form.
- **Opacity**, 0 to 1, is how much of the pixel is there. 0 is empty. It is
  part of the drawing: a soft edge or thinning mist shows with lighting off.
  It is not a material property; see-through *material* is the body dial.
- **Facing** is the surface direction, each number from −1 to 1. The third
  part (toward the viewer) is not stored: it is √(1 − x² − y²). A part that
  turns or mirrors turns its facing with it.
- **Material number** picks a row of the sprite's material table.
- **Glow**, 0 to 1: the pixel gives off light in its own color.

Color numbers and material numbers are plain numbers, never blended. Sheets
are stored as PNG streams: Color as a two-channel 16-bit image (number,
opacity), Light as an ordinary four-channel 8-bit image. Writers pack drawings
with at least one empty pixel between them and should keep positions stable
when only one drawing changes.

### Palettes

A palette is a list of 1 to 65,536 colors, sRGB, with no opacity of its own.
Every palette of a sprite has the same length, so any color number is valid in
all of them. Because every lighting effect takes its color from the pixel's
color (glow, the tint of clear parts, light glowing inside, sheen), a palette
swap such as shiny recolors all of them.

### Variants

A variant may, in any combination: choose a palette, replace drawings
(per view), **hide parts**, and **show parts**. A part may be marked `extra`:
hidden unless an active variant shows it (costumes, Mega accessories). With
several variants active, the last one that says something about a thing wins.

### Materials

Each sprite has its own material table of up to 256 rows. A Light pixel's
material number picks one row, the way a color number picks a palette color.
There is no built-in list of materials; an editor may offer presets.

| A row holds | |
| --- | --- |
| name | for people ("wing membrane"); readers ignore it |
| shades | color numbers that are this surface, darkest first; may be empty |
| base | which of the shades the surface shows when it faces the viewer head-on |
| finish | −1 to 1: fuzzy (−1), matte (0), polished (1) |
| body | −1 to 1: metal (−1), solid (0), translucent (0.5), clear (1) |

**Finish** is what the surface is like. **Body** is what happens to light that
gets past the surface: it bounces off (metal), stops (solid), scatters inside
and glows back out (translucent), or passes through (clear). Each dial puts
things that cannot be true together on one line, so every combination means
something: matte clear is frosted glass, fuzzy translucent is backlit fluff.

**Shades** are what keep lit pixel art looking drawn. A light should move a
pixel to a darker or lighter color *from its material's own list*, never blend
a new one. A color may appear in more than one material's list (a red used
for both a flame and a mouth). A pixel whose color is not in its material's
list, or whose material has an empty list, is tinted by the light but never
re-shaded; outlines are made this way.

**From facing to shade.** Let *f* be how much a pixel faces a light, from −1
(away) to 1 (straight at it). A surface facing the viewer head-on has *f* = 0.6
under the art's own light and shows the base shade. Turning toward the light
climbs through the lighter shades, reaching the lightest by about *f* = 0.92;
turning away falls through the darker ones, reaching the darkest by about
*f* = −0.6. Each shade gets an equal share of its side. The exact sum is in
[`lab/bake.py`](../lab/bake.py) (`reach`) and is not final.

**What a renderer should make of the dials.** How a lit picture looks stays the
renderer's business, but two renderers should agree on what a material is. For
engines with a standard material model the dials map onto its usual names:

| Standard name | From the dials |
| --- | --- |
| roughness | 1 − max(0, finish) |
| fuzz (sheen) | max(0, −finish) |
| metalness | max(0, −body) |
| transmission (seen through) | max(0, 2·body − 1) |
| subsurface (glows from inside) | 1 − \|2·body − 1\| when body > 0, else 0 |

A polished body between translucent and clear (ice) may glitter: single pixels
flashing as the light or the part moves. Clear glass and wax should not.

Row 0 is used where a sprite has no Light sheet. A value that changes across a
surface uses several rows in steps ("steel", "scuffed steel"). Looks the two
dials cannot say (a rainbow film, a glossy coat over a matte base, shine on
fuzz) can be added later in a new chunk that older readers ignore.

### What the lighting needs to know about the whole sprite

- **The floor is y = 0** in every view. Shadows and reflections are placed
  from it; a creature that hovers is drawn above it.
- **Facing data must agree with the painted shading.** The colors hold the
  artist's own highlights and shadows, painted as if lit from the upper left.
  The facing data must be made so that light from there, on each material's
  base shade alone, lands on the shade the artist painted (see below). Then
  the painted shading and the facing data say the same thing, a renderer can
  start from either, and light from any other side stays consistent with the
  art. A validator can check this pixel by pixel.
- **`depth`** on a part, optional, in pixels: how far in front of (positive)
  or behind the rest of the sprite the part sits. A glowing tail behind the
  body then lights the body's edge and shines through a wing.

### Extras

History, provenance and original game data (Black 2/White 2's own cells, repair
notes) go in **extra** chunks (in the folder form: an `extra/` folder). Games skip
them; editors keep them.

## The binary file

Little-endian throughout. A file is a 16-byte header followed by chunks.

**Header**: the 8 bytes `89 53 50 55 0D 0A 1A 0A` (`\x89SPU\r\n\x1a\n`, which
catches text-mode damage the way PNG's does), then `u16` version (2), `u16`
flags (0), `u32` total file length.

**Chunk**: `u32` length of the data, 4-byte ASCII type, the data, padding to a
multiple of 4 bytes, then a `u32` CRC-32 of type and data.

The type's letter case says what an old reader does with an unknown chunk:

- First letter **uppercase**: **required**. A reader that does not know it
  must refuse the file rather than show it wrong.
- First letter **lowercase**: **safe to ignore**. Readers skip it; editors
  keep it and write it back unchanged.

Version 2 chunks, in this order:

| Type | Holds |
| --- | --- |
| `HEAD` | name, author, version, ticks per second, counts of every list |
| `STRS` | every id and name, once; everything else refers to them by number |
| `VIEW` | views and canvases |
| `PART` | parts: name, parent, `extra` flag, `depth` |
| `DRAW` | drawings: name, and per view a sheet rectangle and offset (or none) |
| `PALS` | palettes: colors only |
| `MATS` | the material table: name, shades, finish, body per row |
| `VARS` | variants |
| `ANIM` | animations: per view, per part, per property, the key count then the ticks (`u32`), values (`f32`, or `u16` drawing number), eases (`u8`) as separate arrays |
| `SCOL` | one view's Color sheet (PNG stream: color number, opacity), one per view |
| `SLIT` | one view's Light sheet (PNG stream), when the sprite has lighting |
| `xtra` | one extra member: a path and its bytes (repeatable) |
| `IEND` | end of file |

Arrays inside chunks are aligned to 4 bytes so a reader can use them in place.

### How the format grows

New features are new chunks, never changes to existing ones. Something an
older game can safely ignore (a new kind of hint, editor data) is a lowercase
chunk. Something it must not ignore (a new kind of track) is uppercase, and
older readers refuse those files cleanly. The header version changes only if
the header or chunk framing itself changes, which should be never.

## The reference picture

As in version 1, with two simplifications: a part draws if it has a drawing
at the tick and is not hidden by a variant (no `visible`, no overlays); `extra`
parts draw only when a variant shows them. A pixel's opacity and its part's
`opacity` at the tick multiply, and parts are laid over one another back to
front. Materials, facing and glow never change this picture.

## Limits

As in version 1, with: 64 MiB per file; 256 views; 4096 parts and 4096
drawings per sprite; sheets up to 16384 pixels a side; 65,536 colors per
palette; 256 materials.

## The library

One Rust crate, `spu`, is the reference: read and write both forms, check a
file, evaluate any part's placement at a tick, draw the reference picture, and
pack sheets. It builds for the browser (WebAssembly) and with a plain C
interface, so thin wrappers cover Unity, Godot, Unreal, C++ and Python from
one repository. Nobody else needs to implement the binary layout.

## From version 1

The library converts every version 1 file:

- Parts and drawings with the same id in several views merge. Ones that differ
  stay as parts or drawings used by one view only.
- Animations with the same id merge, padded to the longest length. One a view
  lacks gets an empty entry for that view.
- Overlays become parts; `visible: false` keys become "no drawing" keys.
- Where a variant replaces a drawing whose overlays differ from its
  replacement's (a female form with its own eyes), the variant also replaces
  the overlay drawings. Version 1's reference picture kept the original's
  overlays, which is wrong for 47 library sprites.
- A palette color that carried its own opacity becomes a plain color, and
  that opacity moves to every pixel that used it. Colors left identical are
  merged, and shorter palettes are padded to the longest.
- Full-color drawings get a palette made of the colors they use.
- Lighting maps convert to the Light sheet: normals keep X and Y, the emission
  map becomes the glow amount, and each distinct material and surface-map value
  becomes a material row with the nearest finish and body. Shades are not in
  version 1 files; a converted row has an empty list until an editor fills it.
- Drawings are packed onto sheets; ZIP members that are not part of the
  sprite become extras.
- The reference picture is identical at every tick.

## Decided since the first draft

- **Sheets per view**, not one set for the whole sprite.
- **Sheets stay PNG** in the binary file. PNG is lossless compression of the
  raw pixels; decoding a sprite-sized PNG is cheap, browsers decode it natively,
  and the GPU formats engines use for big textures are lossy and smear pixel art.
- **A view may leave an animation empty.** The animation still exists for every
  view, so playing it never fails; a view with no tracks holds its rest pose.
  Validators may warn about it.
- **Two sheets per view: Color and Light.** Per pixel, only color, normal,
  glow and a material number; everything else is a row in the material table.
- **One way to store color: a palette.** Every sprite in the library (1,131)
  already is palette art. The ceiling is 65,536 colors, not 256, so busy art
  is never squeezed.
- **Opacity is per pixel, not part of a palette color.** One green at twenty
  strengths was twenty palette colors; one library sprite fell from 255
  colors to 123 with opacity counted separately.
- **A material is two dials, finish and body**, not nine values. The seven
  16-color sprites drew the same from two dials as from six separate values.
- **Each material lists its shades.** Without the list, lighting has to blend
  new colors, and the result stops looking like pixel art.
- **Every sprite has an author and a version**, beside its name.
- **No stored "lit from" direction.** It was drafted as a field and dropped:
  no sprite needed any value but upper left, so it is a rule for whoever makes
  the facing data.
- **No "painted" versus "flat" kinds of art.** Whether to keep the painted
  shading or light flat color was an open question until the facing data was
  refit to the painted shading. Lit from the upper left, flat color then landed
  on the artist's shade for 100% of shaded pixels across eleven sprites (98%
  once stored in 8 bits), against 24% from the blown-up shape alone, and the
  two ways of lighting gave nearly the same picture from every side.

## Tried and not tried

Tried in `lab/`, on twelve Black 2 / White 2 sprites (front and back) and four
Uranium sprites (front), in six lit places:

- Facing, material number and glow per pixel; the two dials; shades.
- Palettes of up to 192 colors, with shade lists gathered by a tool from a few
  sample colors and more than a hundred shades long.
- Per-pixel opacity over solid pixels (soft mist over a hull).

Not tried, and still open:

- **More than 256 colors.** No library sprite has them.
- **The Color sheet as a two-channel 16-bit PNG.** The lab keeps its own
  packed copy; nothing has written or read this layout.
- **`depth` per part.** The lab uses one depth for all of a sprite's glow.
- **Parts shared across views.** The lab reads version 1 files, where each
  view has its own list.
- **Markings painted as shades.** A stripe painted as a lighter shade of its
  surface is read as a bump and dims when lit from the far side. It needs its
  own material; nothing finds these automatically.
- **Turned parts.** The check above was made on unturned parts. A part that
  rotates carries its facing data with it, as it should, but the painted
  colors do not change, so the two drift apart as it turns.
- **How many sprites want a look the dials cannot say.** Eleven sprites did
  not; the library has not been surveyed.
