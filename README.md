# SPU

SPU (`.spu`) is a file format for animated 2D sprites. One file holds a
character's drawings, the parts they are joined into, how the parts move, its
alternate looks, and how every pixel takes light.

https://github.com/user-attachments/assets/8d6daa1c-b2e2-4372-9551-996d32052126

*An 82-second tour: one sprite built up a layer at a time, from the flat
color it is stored as to its pieces, animation, facing, light, materials and
glow; then places, variants and views. [Download it with English
captions](docs/spu-tour.mp4).*

## What's here

- **[The specification](docs/spec.md)**: what a file holds and every rule it
  must keep.
- **The reference library** (`src/`, Rust) and the `spu` command: read, write,
  check, convert and draw `.spu` files. If another program disagrees with it
  about a file, one of the two has a bug.
- **[The conformance suite](conformance/README.md)**: files every
  implementation must accept, the exact picture each must draw, and files it
  must refuse.
- **[The demo](lab/README.md)**: a browser page that lights `.spu` files, with
  its own independent reader. The video is made from it.

No sprite files live here. They are in
[pku-spu](https://github.com/prof64/pku-spu), and they are edited in
[Smeargle](https://smeargle.prof64.net).

## Quick start

```sh
cargo build --release --bin spu
target/release/spu info     sprite.spu                 # what is in it
target/release/spu validate sprite.spu ...             # is it a correct file
target/release/spu render   sprite.spu out.png --view front --animation idle --tick 0
target/release/spu animate  sprite.spu out.png --variant shiny
target/release/spu convert  old.spu new.spu --verify   # upgrade an older file; check every picture
cargo test                                             # includes the conformance suite
```

To run the demo, see [`lab/README.md`](lab/README.md).

## What a file holds

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

| | |
| --- | --- |
| Extension | `.spu` |
| First 8 bytes | `89 53 50 55 0D 0A 1A 0A` |
| Older files | a ZIP archive whose `sprite.json` has `"format": "spu"` |
| Media type | none registered yet |

## Where it stands

- **Done and tested:** the file layout, the reference library, the exact
  unlit picture, the conformance suite (15 files to accept, 36 to refuse) and
  the converter from older files.
- **Proven on real sprites:** all 1,134 sprites in pku-spu convert, and every
  picture of every tick, view and look is unchanged: 2,115,922 pictures. The
  converted files total 42 MiB against 75 MiB before.
- **Described but not pinned down:** how a lit picture should look. The demo
  is one renderer; the specification says what the data means.
- **Not built yet:** a folder form for hand editing, bindings for the browser
  and C, and palettes above 256 colors in practice.

This is format version 2. Versions 0 and 1 were a ZIP of JSON and PNGs; that
form is at the tag `v1`, and `spu convert` upgrades those files. The
specification ends with [what has been tried](docs/spec.md#what-has-been-tried).

## Credits

The sprites shown are from Pokémon Black 2 and White 2 (Nintendo, Game Freak,
Creatures) and from the fan game Pokémon Uranium. They are used as test
subjects and belong to their owners; nothing in this repository grants a
license to them. Their rigging and animation come from the pku-spu library.
