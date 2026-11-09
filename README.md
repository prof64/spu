# SPU — Sprite Universal

An editable file format for animated 2D sprites (`.spu`), its reference
renderer, a conformance suite, and a WebGPU viewer that lights sprites with
material maps. One file holds drawings, the parts they are joined into, the
animation, alternate looks such as recolors, and optional surface maps.

- [Draft specification, version 1](docs/spec.md) — scope, rules and limits.
- [Conformance suite](conformance/README.md) — files every implementation
  must accept, the exact picture each must draw, and files it must refuse.
- `spu` command — `cargo run --bin spu -- info|validate|convert|render …`
  inspects, checks, upgrades older files, and draws a frame to PNG.

Older files open directly and save as version 1: Smeargle's `.spu` version 0
and this repository's earlier `.sprite` prototype. The crate and the viewer
still carry the working name Sprite Lab.

## Run

```sh
bash tools/build.sh
python3 tools/serve.py --port 8767
```

Open **http://127.0.0.1:8767**. Requires Rust, the `wasm32-unknown-unknown` target,
wasm-bindgen CLI **0.2.129**, Python 3 and WebGPU. Cargo dependencies are pinned.
The viewer runs locally; it does not upload files or need an account.

## Compare sprites

Choose Charizard, Cryogonal or Glalie in the left-hand list. **Original** shows
source colors; **With materials** shows the same animation time, view, placement
and zoom with the authored material maps. The reference path bypasses lighting,
material opacity, weather, bloom and tone mapping. It retains source animation
opacity. Pause or scrub the shared timeline to inspect a matching pose.

Use **View** for Front, Back, Battle or Stress test. Battle places the selected
Pokémon near the camera and the opponent facing it. Choose Studio, Cave or Rain
under **Environment**; drag in the effects pane to move the light. Material
sliders are always visible, with map inspection and renderer budgets under
**Advanced**. Disable **Enabled** for a direct source-color check in both panes.

Drop an `.spu` (or older `.sprite`) file or use **Open file…** to add an asset. **Save copy** exports
the selected asset with the material adjustments. Drawing and keyframe editing
remain editor work. [UI decisions and design references](docs/ui-design.md).

## Local Pokémon fixtures

The default library loads `web/fixtures/charizard.sprite`, `cryogonal.sprite` and
`glalie.sprite`, plus their PNG thumbnails. These local game-art studies are
ignored by Git and are not distributed with this repository. A checkout without
them shows an empty state with **Open file…**, rather than substituting artwork.

| Pokémon | Demonstration |
| --- | --- |
| Charizard | Front/Back Gen 5 animation, emissive tail flame and transmitting wing membrane |
| Cryogonal | Faceted ice, reflection, transmission and cyan emission |
| Glalie | Glossy ice, opaque facial details and emissive eyes |

With the original local SPUs under `artifacts/source/` and Python's Pillow/numpy
available, rebuild the archives with `python3 tools/prepare_fixtures.py`.
The importer expects native preview PNGs in the source directory; use
`--skip-previews` to rebuild archives without copying thumbnails. Material maps
are authored approximations added to the source sprites, not original game data.

## Retained examples

The previous generated artwork remains in the repository for format examples and
reproducible historical benchmarks. It is not loaded by the viewer by default.

| Example | Demonstration |
| --- | --- |
| Cindermaw | Separate Front/Back battle drawings, animated magma and scene lighting |
| Mirror knight | Polished convex shield, brass armor, rough cloth and Guard animation |
| Prismcoil | Faceted glass, transmitting interiors, bevel normals and animated motes |
| Dungeon scout | Eight directions, layered walking poses and a hand-held lantern |
| Obsidian hall | A material-shaded environment plate with stone architecture and plinths |

These original images were created with the built-in image-generation tool and
converted to native pixels. Source images, exact prompts, palettes and editable
maps are retained under `web/art-source/`; individual rebuild scripts are under
`tools/`. Materials are authored approximations. See [examples and rebuild commands](docs/examples.md).

## Measure

**View → Stress test** renders a fixed-size mixed population from the loaded
assets in both comparison panes. The selected count applies to each pane, so
10,000 means 10,000 in the reference plus 10,000 in the effects pane. Its FPS
readout measures browser display cadence across both renders, not GPU completion
or maximum native throughput. Lights use
64-pixel spatial tiles, at most eight contributors per tile and a configurable
512-light maximum. World reflections use visible scene color/depth.
[Rendering details](docs/world-rendering.md).

```sh
cargo test --all-targets
cargo test --test renderer -- --ignored
cargo build --locked --release --bin benchmark
python3 tools/benchmark_world.py
python3 tools/benchmark_world.py --size 1920x1080 --output web/measurements/world-1080.json
```

The native comparison script uses the repository's retained example collection;
changing the browser library does not change that benchmark dataset. It measures
base materials, local lights, then lights plus reflections. Completed-frame
timings include GPU completion; they are not the viewer's FPS readout.

The recorded 2026-11-09 mixed-world results use Cindermaw, Mirror knight, Prismcoil,
Dungeon scout, Lantern and Moss stone, with Obsidian hall also loaded. Those
results do not describe the current Pokémon comparison viewer. Earlier Pokémon
runs in the record use their own workloads and renderer revisions. Raw records
retain settings, asset hashes, hardware and source fingerprints; consult those
inputs before comparing runs.
Gameplay, physics, audio, streaming and networking require additional budgets.
See [method](docs/benchmarks.md) and [recorded results](benchmarks/README.md).

## Corpus check

`tools/corpus-check` upgrades real version 0 files and compares every tick of
every animation, view and variant with Smeargle's own renderer, pixel for
pixel. It needs a Smeargle checkout at `../pk-mono/smeargle` and is therefore
a separate package:

```sh
cd tools/corpus-check && cargo run --release -- <folder of .spu files>... [--write <output folder>]
```

## Code

| Location | Purpose |
| --- | --- |
| `src/format.rs` | Document model, archive validation and preservation of unknown fields |
| `src/migrate.rs` | Upgrading version 0 and prototype files when opened |
| `src/reference.rs` | Reference renderer: the exact lighting-free picture |
| `src/bin/spu.rs` | Command-line tool |
| `conformance/`, `src/bin/make_conformance.rs` | Conformance suite and its generator |
| `src/runtime.rs` | Texture atlas, compiled animation, material batching and emitted lights |
| `src/world.rs` | Composed scenes, population workload and spatial light lists |
| `src/render.rs`, `src/*.wgsl` | Shared native/WASM renderer |
| `src/browser.rs`, `web/` | Local viewer, examples and numerical measurements |
| `src/bin/benchmark.rs` | Completed-frame native measurements |

[Prototype notes](docs/prototype.md) · [Depth and weather](docs/depth-and-weather.md)
· [Name ideas](docs/name-ideas.md). Drawing and keyframe editing remain editor work;
the viewer adjusts materials and playback. Hosting and formal standardization
remain deferred.
