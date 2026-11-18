# SPU

SPU (`.spu`) is a file format for animated 2D sprites. One file holds a
character's drawings, the parts they are joined into, how the parts move, its
alternate looks, and how every pixel takes light.

![Charizard in a field. The sun crosses the sky and the shading on its body follows.](docs/images/hero.gif)

[![A short tour: a sprite is taken apart into the flat pieces the file stores, then put back together and lit.](docs/images/tour-poster.png)](docs/spu-tour.mp4)

*[Watch the tour](docs/spu-tour.mp4) (73 seconds): the flat color a sprite is
stored as, the sheet its pieces live on, how an animation assembles them, and
how a light turns flat color and facing into shading. Then materials, glow,
places and looks. English captions are available in the player.*

This repository is the format's home:

- **[The specification](docs/spec.md)**: what a file holds and what every
  rule is.
- **The reference library** (`src/`, in Rust) and the `spu` command: read,
  write, check, convert and draw `.spu` files. "Reference" means it is the
  implementation the specification is checked against. If another program
  disagrees with it about a file, one of the two has a bug.
- **[The conformance suite](conformance/README.md)**: files every
  implementation must accept, the exact picture each must draw, and files it
  must refuse.
- **[The demo](lab/README.md)**: a browser page that lights `.spu` files, with
  its own independent reader. The pictures below come from it.

The sprites pictured are from Pokémon Black 2 and White 2 and from Pokémon
Uranium, used as examples; see [Credits](#credits). No sprite files are kept
in this repository.

## What the format does

### Parts that move

A sprite is a set of parts. Each part shows one drawing at a time and can
move, turn, stretch, fade and swap to another drawing. Time is counted in
ticks, and every tick has one exact picture, so any two programs can be
compared pixel for pixel.

![Charizard assembled, with each part in its own color, and with the parts pulled apart.](docs/images/parts.png)

*The sprite, each part in its own color, and the parts pulled apart.*

![Klinklang's gears turning under a moving lamp.](docs/images/gears.gif)

*Lighting data belongs to a drawing, so it turns with the part that shows it.*

### Views

One sprite holds several views of the same character: front and back, or
eight walking directions. A game loads only the ones it uses.

![Charizard and Arceus, each from the front and from the back.](docs/images/views.png)

### Looks

A look reuses the same animation. It can swap the palette, swap drawings, or
both, and looks combine.

![Arceus in eight of its palettes.](docs/images/palettes.png)

*One Arceus file carries 34 palettes: seventeen types, each with a shiny.*

![Jellicent: male, female, female with eyes closed, female shiny.](docs/images/looks.png)

*Jellicent's female look swaps every drawing. Eyes closed swaps one. Shiny
swaps the palette.*

### Lit by wherever it stands

The file says which way each pixel faces, what it is made of and how much it
glows. The place supplies the light. Nothing is redrawn between places.

![Six sprites in six places: a studio, a cave, rain, a field at dusk, snow under an aurora, and deep water.](docs/images/places.png)

![Charizard lit from the left, from the right, from behind in a cave, and in rain during a lightning flash.](docs/images/light.png)

*The same file lit from the left, from the right, from behind (light comes
through the wings), and in rain.*

Light never invents a color. Each material lists its own shades, dark to
light, and a light only moves a pixel along that list. That is what keeps a
lit sprite looking like pixel art.

### Flat color + facing + light

What goes into the file is flat color and, for each pixel, which way it
faces. Shading is what a light makes of those two.

![Flat color, plus which way each pixel faces, plus a light from the upper left, gives the art as the artist drew it. The same data is then shown lit from the right, from below, from behind and in rain.](docs/images/equation.png)

Existing sprites have highlights and shadows painted into their colors. The
facing data is built to agree with them: put the light where the artist
imagined it, the upper left, and the painted shading comes back. On sixteen
test sprites it landed on the artist's shade for every shaded pixel. Move the
light anywhere else and the sprite stays consistent with how it was drawn.

### What each pixel stores

Six numbers, in two images per view:

| Image | Per pixel |
| --- | --- |
| **Color** | color number (into the palette) · opacity |
| **Light** | facing, left to right · facing, bottom to top · material number · glow |

![Charizard four ways: its colors, which way each pixel faces, its materials, and its glow.](docs/images/pixel.png)

*Color, facing, material and glow.*

![The Color sheet, the Light sheet and the glow channel for Charizard's front view.](docs/images/sheets.png)

*The sheets as stored. Every drawing sits at the same place on both, so one
rectangle finds its color and its lighting.*

### Materials are two dials

Each sprite carries a short list of materials, and every pixel names one. A
material is a name, its shades, and two dials:

- **Finish**: what the surface is like. Fuzzy, matte or polished.
- **Body**: what happens to light that gets past the surface. It bounces off
  (metal), stops (solid), glows back out (translucent) or passes through
  (clear).

![Six sprites, each showing one material: polished metal, ice, fur, a wing membrane lit from behind, glass and flame, and jelly.](docs/images/made-of.png)

*Metal mirrors the grass and sky around it. Ice shows the scene through
itself and glitters. A wing lets light through from behind.*

<img src="docs/images/materials.png" width="760" alt="A grid of clay-colored balls: four finishes down the side, six bodies across the top.">

*Every ball is the same color under the same light. Only the dials differ.
Ice, glass, steel, fur, skin and wax are all points on this map.*

### Many colors, soft pixels

Color is always a palette, with room for up to 65,536 colors. Opacity is
stored per pixel, so soft mist does not use up colors.

![Halloween Titanice and Christmas Corsoreef, each as drawn and lit.](docs/images/soft.png)

*Halloween Titanice: soft see-through mist over a solid hull. Christmas
Corsoreef: 172 colors and 37 parts.*

## The shape of a file

```
SPRITE  (one .spu file)
├─ BASICS      name · author · version · ticks per second
├─ PALETTES    normal, shiny, ...            lists of colors
├─ MATERIALS   name · shades · finish · body
├─ PARTS       head, wing, hat, ...          one list for all views
├─ DRAWINGS    head-open, head-shut, ...     one list for all views
├─ VIEWS       front, back, ...
│   ├─ Color sheet    per pixel: color number · opacity
│   ├─ Light sheet    per pixel: facing x · facing y · material number · glow
│   └─ where each drawing sits on the sheets
├─ ANIMATIONS  idle, attack, ...
│   └─ per view, per part: drawing shown · position · turn · stretch
│                          draw order · opacity · glow strength
├─ LOOKS       shiny, eyes closed, hat       palette · swapped drawings · shown and hidden parts
└─ EXTRAS      history, original game data   games skip these
```

On disk that is a 16-byte header and a list of chunks, each with a checksum.
The [specification](docs/spec.md#the-file) gives every byte.

## Where it stands

- **Done and tested:** the file layout, the reference library, the exact
  unlit picture, the conformance suite (15 files to accept, 36 to refuse) and
  the converter from older files.
- **Proven on real sprites:** all 1,134 sprites in the
  [pku-spu](https://github.com/prof64/pku-spu) library convert, and every
  picture of every tick, view and look is unchanged: 2,115,922 pictures. The
  converted files total 42 MiB against 75 MiB before.
- **Described but not pinned down:** how a lit picture should look. The demo
  is one renderer; the specification says what the data means.
- **Not built yet:** a folder form for hand editing, bindings for the browser
  and C, and palettes above 256 colors in practice.

The specification ends with a fuller list of
[what has been tried](docs/spec.md#what-has-been-tried).

**Format versions.** This is format version 2. Before it, an `.spu` file was a
ZIP of JSON and PNGs (versions 0 and 1). That form, its specification and its
renderer are in this repository's history at the tag `v1`, and `spu convert`
upgrades those files.

## In this repository

| | |
| --- | --- |
| [`docs/spec.md`](docs/spec.md) | The specification |
| `src/sprite.rs` | What a sprite holds, and every rule a file must keep |
| `src/binary.rs` | Reading and writing `.spu` files |
| `src/picture.rs` | The exact unlit picture at a tick |
| `src/convert.rs`, `src/legacy/` | Upgrading older files, and checking the result |
| `src/bin/spu.rs` | The `spu` command |
| [`conformance/`](conformance/README.md) | The conformance suite |
| [`lab/`](lab/README.md) | The demo, and the tools that make lighting data for a sprite |

The reference library is deliberately bare. It exists so that anyone can check
a file and see exactly what it should look like. Editing is done in
[Smeargle](#related).

### The `spu` command

```sh
cargo build --release --bin spu
target/release/spu info     sprite.spu                 # what is in it
target/release/spu validate sprite.spu ...             # is it a correct file
target/release/spu render   sprite.spu out.png --view front --animation idle --tick 0
target/release/spu animate  sprite.spu out.png --variant shiny
target/release/spu convert  old.spu new.spu --verify   # upgrade an older file; check every picture
target/release/spu convert  old.spu new.spu --light folder   # ...and add lighting made elsewhere
cargo test                                             # includes the conformance suite
```

### The demo

```sh
cd lab
uv run --no-project --with numpy --with scipy --with pillow python export.py
python3 build.py && python3 standalone.py
open out/html/spu-format-lab.html
```

It needs the sprite library ([pku-spu](https://github.com/prof64/pku-spu))
checked out; set `SPU_LIBRARY` to its folder. Any `.spu` file can be dropped
on the page. See [`lab/README.md`](lab/README.md).

## Telling an SPU file apart

| | |
| --- | --- |
| Extension | `.spu` |
| First 8 bytes | `89 53 50 55 0D 0A 1A 0A` |
| Older files | a ZIP archive whose `sprite.json` has `"format": "spu"` |
| Media type | none registered yet |

## Making an SPU from an existing sprite

Most sprites will be made by taking apart art that already exists. The steps:

1. **Split it into parts and animate them.**
2. **Take the lighting out.** For each surface, find its shades and which one
   is the base.
3. **Work out which way each pixel faces,** so that light from the upper left
   gives back the shading the artist painted.
4. **Decide what each surface is made of:** its finish and body, and what
   glows.
5. **Do the same for the back view.** If the game has none, draw one.

Steps 2 and 3 are automatic in [`lab/`](lab/README.md). Steps 1, 4 and 5 need
a person or a model, and are what the editor is for.

## Related

- **[Smeargle](https://smeargle.prof64.net)**: the editor for SPU files.
- **[pku-spu](https://github.com/prof64/pku-spu)**: the library of Pokémon
  battle sprites as SPU files. Sprite files live there, not here.

## Credits

The sprites shown here are from Pokémon Black 2 and White 2 (Nintendo, Game
Freak, Creatures) and from the fan game Pokémon Uranium. They are used as
test subjects and belong to their owners; nothing in this repository grants a
license to them. Their rigging and animation come from the pku-spu library.
