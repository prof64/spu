# SPU specification

SPU (`.spu`) is a file for one animated 2D character,
object or effect: its drawings, the parts they are joined into, how the parts
move, its alternate looks, and how every pixel takes light.

This document describes **format version 2**, the current one. It is
implemented by this repository (`src/`) and pinned down by the
[conformance suite](../conformance/README.md). The words *must*, *must not*
and *may* are requirements on files and readers.

The unlit picture is exact: every reader must draw the same pixels. The lit
picture is described, not fixed pixel for pixel: this document says what the
lighting data means, and each renderer decides how to draw it.

## Scope

**In**

- Pixel art in a palette of up to 65,536 colors, with per-pixel opacity.
- Frame-by-frame animation, cut-out animation with joined parts, and both
  mixed.
- Several views of one subject (front and back, eight walking directions).
- Looks that reuse the same animation: recolors, replaced drawings, parts
  shown or hidden.
- Lighting data: which way each pixel faces, its material, how much it glows.
- One exact, lighting-free picture for every tick, so any two tools can be
  compared pixel for pixel.
- Staying editable: a file opened and saved by any tool keeps what that tool
  does not understand.

**Out**

- Mesh deformation, bones separate from parts, constraints, physics.
- Blending between animations, state machines, events, sound.
- Full-color (non-palette) art, vector art, text, video.
- Scenes, cameras, lights and weather. The file describes the sprite; the
  place it stands in supplies the light.

## The sprite

Everything is defined once for the whole sprite. Lists are numbered from 0,
and everything refers to everything else by number. Ids are non-empty text,
unique within their list; names are for people and may be any text.

- **Basics**: `name`, `author`, `version` (the sprite's own version, a whole
  number from 1 that goes up each time it is published; not the format
  version), and ticks per second (more than 0).
- **Parts**: id, name, `extra`, `depth`. One list for all views. A part marked
  `extra` is hidden unless an active look shows it (a hat, a costume piece).
  `depth`, in pixels, is how far in front of (positive) or behind the rest of
  the sprite the part sits; it only matters to lighting.
- **Drawings**: id and name, one list for all views ("arm-down" is the same
  pose from every angle). Each view gives a drawing its own picture, or none.
- **Views**: id, name, a canvas, an ordered list of the parts the view shows,
  and the view's sheets.
- **Palettes**, **materials**, **looks** and **animations**: below.

### Views

Sprite space has +X right and +Y down, in pixels. **The floor is y = 0** in
every view: shadows and reflections are placed from it, and a creature that
hovers is drawn above it.

The canvas is the window that pictures show: the sprite-space point of its
top-left pixel (whole numbers) and its width and height (1 to 4096).

A view lists the parts it shows. Each entry names a part and, optionally, the
entry it is joined to (its parent) by position in the same list. Parents must
not form a loop. **The order of this list breaks draw-order ties.** A part
may be joined differently in different views, and a part a view does not list
does not exist from that angle.

### Pictures: two sheets per view

Each view's drawings are packed onto one **Color** sheet, so a game can load
only the views it uses. A view with lighting has a second sheet, **Light**,
the same size and layout, so one rectangle finds a drawing's color and its
lighting. A drawing's picture in a view is a rectangle on those sheets plus
an offset: the picture's top-left corner, measured from the origin of the
part that shows it.

Every pixel holds six numbers:

| Sheet | Per pixel |
| --- | --- |
| Color | color number · opacity |
| Light | facing, left to right · facing, bottom to top · material number · glow |

- **Color number** picks a color from the active palette. It is the only way
  a pixel gets a color.
- **Opacity**, 0 to 255, is how much of the pixel is there; 0 is empty. It is
  part of the drawing: a soft edge shows with lighting off.
- **Facing** is the direction the surface points, each number from −1 to 1,
  stored as `128 + 127·n`. The third part (toward the viewer) is not stored:
  it is √(1 − x² − y²). A part that turns or mirrors turns its facing with it.
- **Material number** picks a row of the material table.
- **Glow**, 0 to 255: the pixel gives off light in its own color.

Color numbers and material numbers are plain numbers and are never smoothed.
Writers must leave at least one empty pixel between drawings and should keep
positions stable when only one drawing changes.

### Palettes

A sprite has one or more palettes, all the same length: 1 to 65,536 colors.
A color is red, green, blue (sRGB) and an opacity.

A palette color's opacity multiplies the pixel's own. It exists for one case:
a color that one look has and another does not (a pattern only the shiny
form shows). Everywhere else it should be 255, and opacity belongs on the
pixel, so that one color at twenty strengths is one color.

Because every lighting effect takes its color from the pixel's color (glow,
the tint of clear parts, light glowing inside), a palette swap recolors all
of them.

### Materials

A sprite has its own material table of up to 256 rows; a Light pixel's
material number picks a row, the way a color number picks a color. There is
no built-in list of materials. An editor may offer presets.

| A row holds | |
| --- | --- |
| name | for people ("wing membrane"); readers ignore it |
| shades | color numbers that are this surface, darkest first; may be empty |
| base | which of the shades the surface shows facing the viewer head-on |
| finish | −1 to 1: fuzzy (−1), matte (0), polished (1) |
| body | −1 to 1: metal (−1), solid (0), translucent (0.5), clear (1) |

**Finish** is what the surface is like. **Body** is what happens to light
that gets past the surface: it bounces off (metal), stops (solid), scatters
inside and glows back out (translucent), or passes through (clear). Each dial
puts things that cannot be true together on one line, so every combination
means something: matte clear is frosted glass, fuzzy translucent is backlit
fluff.

**Shades** keep lit pixel art looking drawn. A light should move a pixel to a
darker or lighter color *from its material's own list*, never blend a new
one. A color may appear in more than one material's list. A pixel whose color
is not in its material's list, or whose material lists none, is tinted by
light but never re-shaded; outlines are made this way.

Row 0 is used where a view has no Light sheet. A value that changes across a
surface uses several rows in steps ("steel", "scuffed steel").

### Looks (variants)

A look may, in any combination: choose a palette, replace drawings (per
view), show parts and hide parts. Any number can be active at once, in a
chosen order.

- The palette in use is the one named by the last active look that names
  one; otherwise the first palette.
- A drawing is replaced if an active look says so; when several do, the last
  wins. Replacements do not chain.
- A part is shown or hidden by the last active look that mentions it.
  Otherwise a part shows unless it is marked `extra`.

The format calls these *variants*.

### Animations

An animation has an id and a name, shared by every view, and for each view
either nothing or that view's **motion**: a length in ticks (more than 0),
whether it loops, and tracks. Each view keeps its own length, because a loop
seen from the back need not be as long as from the front. A view with no
motion for an animation draws nothing while it plays.

A track animates one part; a motion has at most one track per part, and only
for parts its view shows. A track holds:

| Property | Value | With no keys |
| --- | --- | --- |
| `image` | a drawing, or none | none |
| `x`, `y` | pixels, from the parent | 0 |
| `rotation` | degrees, clockwise on screen | 0 |
| `scaleX`, `scaleY` | factor; negative mirrors | 1 |
| `z` | draw order, higher is in front | 0 |
| `opacity` | 0 to 1 | 1 |
| `emission` | multiplier on the part's glow | 1 |

Keys are in strictly increasing tick order. Ticks are whole numbers. Numbers
are 64-bit, finite, and no larger than 1,000,000 in size. A part with no
drawing at a tick does not draw.

**Value at a tick.** Before a property's first key, the first key's value
holds. Otherwise take the last key at or before the tick. `image` holds that
key's value. Numbers move toward the next key (if any) by the earlier key's
ease, with *t* the fraction of the way between the two ticks:

| Ease | Amount moved |
| --- | --- |
| step | 0 |
| linear | t |
| ease in | t² |
| ease out | t(2 − t) |
| ease in-out | t²(3 − 2t) |

**Playback.** At playback tick *T*, a looping motion shows tick
*T* mod length; any other shows min(*T*, length − 1).

### Extras

History, provenance and original game data ride along as extras: a path and
bytes each. Games skip them; editors keep them.

## The reference picture

For a view, an animation, a tick and a list of active looks, every conforming
renderer must be able to produce exactly this picture: straight (not
premultiplied) 8-bit RGBA, the size of the canvas, transparent where nothing
draws.

1. **Place each part.** A part's own placement is its `x`, `y`, `rotation`,
   `scaleX`, `scaleY` at the tick. A part with a parent combines with the
   parent's final placement: its position is carried through the parent's
   whole placement (scale, then rotation, then position); its rotation is
   added to the parent's, or subtracted if exactly one of the parent's
   scales is negative; its scales are multiplied by the parent's. Parts
   therefore never skew. After each combination, any of the five numbers
   within 10⁻⁹ of a multiple of 2⁻²⁰ becomes that multiple. Arithmetic is
   64-bit floating point.
2. **Choose what draws.** A part draws if it has a track, the track shows a
   drawing at the tick, the looks leave the part showing, and the drawing
   (after replacement) has a picture in this view. A parent passes on
   placement only: a parent with no drawing, or a faded one, does not hide
   or fade its children.
3. **Order.** Parts draw from lowest `z` to highest; equal `z` in the order
   the view lists its parts.
4. **Paint.** For every canvas pixel, take its center, map it back into the
   part's space, and subtract the drawing's offset. A coordinate within 10⁻⁹
   of a whole number counts as that number; otherwise round down. If that
   lands on a pixel of the drawing's rectangle whose opacity is not 0, the
   pixel's palette color is blended "over" what is already there. Its alpha
   is the pixel's opacity times the palette color's opacity, as
   ⌊(a·b + 127) / 255⌋, multiplied by the part's opacity clamped to 0–1. A
   part whose scale is 0 draws nothing.

There is no smoothing: one sheet pixel is chosen per canvas pixel. Materials,
facing and glow never change this picture.

## Lit pictures

How a lit picture looks is the renderer's business, but two renderers should
agree on what the data means. This section is guidance, not a rule a
conformance test checks.

**Facing data agrees with the painted shading.** A sprite's colors already
hold its artist's highlights and shadows, painted as if lit from the upper
left. Facing data must be made so that light from there, on each material's
base shade alone, lands on the shade the artist painted. Then the painted
shading and the facing data say the same thing: a renderer can start from
either, and light from any other side stays consistent with the art.

**From facing to shade.** Let *f* be how much a pixel faces a light, from −1
(away) to 1 (straight at it). A surface facing the viewer head-on has
*f* = 0.6 under the art's own light and shows the base shade. Turning toward
the light climbs through the lighter shades, reaching the lightest by about
*f* = 0.92; turning away falls through the darker ones, reaching the darkest
by about *f* = −0.6.

**What the dials mean** in a standard material model:

| Standard name | From the dials |
| --- | --- |
| roughness | 1 − max(0, finish) |
| fuzz (sheen) | max(0, −finish) |
| metalness | max(0, −body) |
| transmission (seen through) | max(0, 2·body − 1) |
| subsurface (glows from inside) | 1 − \|2·body − 1\| when body > 0, else 0 |

A polished body between translucent and clear (ice) may glitter. Clear glass
and wax should not.

The demo in [`lab/`](../lab/README.md) is one renderer built this way.

## The file

Everything is little-endian. A file is a header and then chunks.

**Header**, 16 bytes:

| Bytes | |
| --- | --- |
| 8 | `89 53 50 55 0D 0A 1A 0A` (`\x89SPU\r\n\x1a\n`). Like PNG's, these catch a file damaged by being handled as text. |
| 2 | format version: 2 |
| 2 | flags: 0 |
| 4 | the file's total length in bytes |

**Chunk**: a 16-byte header, the data, and zero bytes up to a multiple of 8,
so every chunk's data starts on an 8-byte boundary.

| Bytes | |
| --- | --- |
| 4 | type: four ASCII letters |
| 4 | length of the data as stored |
| 4 | CRC-32 (as in PNG and ZIP) of the type and the stored data |
| 4 | flags. 1: the data is stored compressed, as a zlib stream. Other bits must be 0. |

A reader must refuse a file whose header, lengths, checksums or padding are
wrong.

The case of a type's first letter says what a reader does with a chunk it
does not know:

- **Uppercase**: required. The reader must refuse the file.
- **Lowercase**: safe to ignore. The reader skips it; a tool that saves the
  file keeps it and writes it back.

### Chunks

In this order. `text` is a 2-byte length and that many bytes of UTF-8.
`index` is a 2-byte number where 65535 means none. `flag` is one byte, 0 or 1.

| Type | Holds |
| --- | --- |
| `HEAD` | name `text`, author `text`, version `u32`, ticks per second `f32`, then seven `u16` counts: views, parts, drawings, palettes, materials, looks, animations |
| `VIEW` | per view: id `text`, name `text`, canvas x `i32`, y `i32`, width `u32`, height `u32`, part count `u16`, then per part shown: part `u16`, parent `index` |
| `PART` | per part: id `text`, name `text`, extra `flag`, depth `f32` |
| `DRAW` | per drawing: id `text`, name `text`, then per view a `flag` and, if set: sheet x, y, width, height (`u16` each), offset x, y (`f32` each) |
| `PALS` | colors per palette `u32`, then per palette: id `text`, name `text`, and 4 bytes per color (red, green, blue, opacity) |
| `MATS` | per material: name `text`, finish `f32`, body `f32`, base `u16`, shade count `u16`, shades `u16` each |
| `VARS` | per look: id `text`, name `text`, palette `index`, replacement count `u16` and per replacement view, from, to (`u16` each), then a count and list of shown parts, and a count and list of hidden parts (`u16` each) |
| `ANIM` | one chunk per animation: see below |
| `SCOL` | one per view, in view order: view `u16`, then a PNG stream |
| `SLIT` | one per view that has lighting: view `u16`, then a PNG stream |
| `xtra` | one per extra: path `text`, then its bytes |
| `IEND` | empty; the last chunk |

The seven chunks from `HEAD` to `VARS` each appear exactly once.

**`ANIM`**: id `text`, name `text`, then per view a `flag` and, if set: length
`u32`, looping `flag`, track count `u16`, and per track:

- part `u16`;
- image keys: count `u32`, zero bytes to a multiple of 4, the ticks (`u32`
  each), then the drawings (`index` each);
- then for each of `x`, `y`, `rotation`, `scaleX`, `scaleY`, `z`, `opacity`,
  `emission`, in that order: zero bytes to a multiple of 4, count `u32`, the
  ticks (`u32` each), zero bytes to a multiple of 8, the values (`f64` each),
  then the eases (one byte each: 0 step, 1 linear, 2 ease in, 3 ease out,
  4 ease in-out).

Padding is counted from the start of the chunk's data.

**`SCOL`**: a PNG, 16-bit gray with alpha, not interlaced. Gray is the color
number. Alpha is the opacity times 257.

**`SLIT`**: a PNG, 8-bit RGBA, not interlaced: facing x, facing y, material
number, glow.

### How the format grows

New features are new chunks, never changes to existing ones. Something an
older reader can safely ignore is a lowercase chunk. Something it must not
ignore is uppercase, and older readers refuse those files cleanly. The
header version changes only if the header or chunk framing itself changes.

## Limits

A reader may refuse files beyond these, and writers must stay within them:
64 MiB per file, and as much again unpacked; 256 views; 4,096 parts and 4,096
drawings; 65,536 colors per palette; 256 materials; sheets up to 16,384
pixels a side; canvases up to 4,096 pixels a side; names up to 65,535 bytes.

## Older files

Before version 2, an `.spu` file was a ZIP archive of `sprite.json` and one
PNG per drawing: Smeargle's version 0, the `.sprite` prototype, and version 1
(this repository at the tag `v1`, with its own specification). They are not
this format, and a reader need not open them. `spu convert` upgrades them:

- Parts and drawings with the same id in several views become one.
- Each view keeps its own order of parts, its own joins, and its own length
  for each animation.
- A drawing's overlay (eyes painted over a head) becomes a part of its own,
  joined to the part it sat on and listed straight after it. Where a look
  replaces a drawing, it now replaces the overlay too; version 1 kept the
  original's overlay, which was a bug.
- "Hidden" keys become "no drawing" keys. Keys that repeat the value before
  them are dropped.
- Where every palette gives a color the same opacity, that opacity moves to
  the pixels. Where palettes disagree, it stays in the palettes.
- Full-color drawings get a palette made of the colors they use.
- Version 1's lighting maps become a Light sheet as far as they translate:
  normals and emission carry over, and each material becomes a row with the
  nearest finish and body and no shades.
- Whatever else the archive carried becomes extras.

Leaving aside the overlay fix, the reference picture is identical at every
tick.

## What has been tried

**Tried**

- The whole sprite library (1,134 files) converts, and every picture of every
  tick, view and look matches the older renderer: 2,115,922 pictures. The
  converted files total 42 MiB against 75 MiB before.
- The conformance suite: 15 files that must be accepted, each with its
  expected pictures, and 36 that must be refused.
- The lighting design, in `lab/`, on sixteen sprites lit in six places:
  facing, material number and glow per pixel; the two dials; shades; palettes
  of up to 192 colors; soft pixels over solid ones. Light from the upper left
  on flat color landed on the artist's shade for every shaded pixel.

**Not tried, or not built**

- **More than 256 colors.** No library sprite has them.
- **A folder form for editing** (the same sprite as loose files). Planned.
- **Bindings** for the browser and for C. The reference library is Rust only.
- **`depth` per part**, in a real file. The demo uses one depth per sprite.
- **Markings painted as shades.** A stripe painted as a lighter shade of its
  surface reads as a bump and dims when lit from the far side. It needs its
  own material; nothing finds these automatically.
- **Turned parts.** A part that rotates carries its facing data with it, but
  its painted colors do not change, so the two drift apart as it turns.
- **How many sprites want a look the two dials cannot say** (a rainbow film,
  a glossy coat over a matte base, shine on fuzz).
