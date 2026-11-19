# SPU

SPU (`.spu`) is a file format for animated 2D sprites. One file holds a
character's drawings, the parts they are joined into, how the parts move, its
alternate looks, and how every pixel takes light.

https://github.com/user-attachments/assets/8d6daa1c-b2e2-4372-9551-996d32052126

## What's here

- **[Specification](docs/spec.md)**
- **Reference library and `spu` command** (`src/`, Rust)
- **[Conformance suite](conformance/README.md)**
- **[Demo](lab/README.md)**: a browser page that lights `.spu` files

## Related

- **[pku-spu](https://github.com/prof64/pku-spu)**: a library of over 1,100
  ready-made `.spu` files, the animated battle sprites of Pokémon Black 2 and
  White 2, Uranium and Insurgence, each with its parts, animations and
  palettes.
- **[Smeargle](https://smeargle.prof64.net)**: an editor for `.spu` files.
  Open a sprite from pku-spu, or start your own, and change its drawings,
  parts, animations and colors.

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

On disk: a 16-byte header, then checksummed chunks. Files start with
`89 53 50 55 0D 0A 1A 0A`. The [specification](docs/spec.md#the-file) gives
every byte.

## License

[MIT](LICENSE): the specification, the library and the conformance suite.
Use them for anything.

The sprites in the video are from Pokémon Black 2 and White 2 (Nintendo, Game
Freak, Creatures) and Pokémon Uranium, and belong to their owners. The license
does not cover them.
