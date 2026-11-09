# Current collection — 2026-11-09

**10,000 objects at 1080p: 13.58 ms median, 17.50 ms p95** across 3,000 completed native frames on Apple M5 Max / Metal. The new artwork changes the workload; the older 10.09 ms result below belongs to the previous collection.

The benchmark loads all seven current packages. The World population mixes Cindermaw, Mirror knight, Prismcoil, Dungeon scout and Lantern, with separate Moss stone terrain tiles. Obsidian hall is loaded into the shared atlas for the showcase but marked `showcaseOnly`, so it is not repeated across the stress scene. Objects keep fixed 48px canvases, independent animation and material maps; none are shrunk or culled as population rises.

## 1080p sweep

Each case has 12 warm-up and 180 measured frames. Base retains material shading, emission, transmission, bloom and the scene capture; it disables local lights and reflection rays. Lights adds a 128-light budget. Full adds 16-step screen-space reflections. Each 64px light tile retains at most eight local lights. The 100-object cases have 100 active lights; larger lit cases have 128.

| Objects | Base median | + Lights median | Full median | Full p95 | Drawing parts |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 1.72 ms | 1.72 ms | 3.37 ms | 7.33 ms | 1,480 |
| 500 | 3.54 ms | 1.89 ms | 5.11 ms | 9.15 ms | 3,720 |
| 1,000 | 5.86 ms | 5.04 ms | 7.16 ms | 10.82 ms | 6,520 |
| 5,000 | 10.12 ms | 9.05 ms | 10.58 ms | 15.93 ms | 28,920 |
| 10,000 | 10.46 ms | 11.04 ms | 14.05 ms | 17.18 ms | 56,920 |

Raw 1080p: [base](2026-11-09-world-1920x1080-base.json), [lights](2026-11-09-world-1920x1080-lights.json), [full](2026-11-09-world-1920x1080-full.json).
Raw 720p: [base](2026-11-09-world-1280x720-base.json), [lights](2026-11-09-world-1280x720-lights.json), [full](2026-11-09-world-1280x720-full.json). Short-run timings vary; profile medians are not isolated shader-cost measurements.

## Sustained stress and limits

The [3,000-frame stress run](2026-11-09-world-1920x1080-sustained.json) uses 10,000 objects, 128 active lights, 16 reflection steps, wet terrain, rain and bloom at 1920×1080. Median **13.58 ms**, p95 **17.50 ms**, slowest observed frame **28.89 ms**. That p95 exceeds a 16.7 ms frame budget before adding game logic. This is not a reliable 60 FPS full-game result at 10,000 objects.

The scene has 56,920 drawing parts, including 920 terrain tiles, and 7 draw calls. The seven packages share 16.00 MiB atlas textures; render targets use 51.44 MiB. Per-frame part upload is 6.08 MiB. CPU animation/batching median is 3.57 ms and upload/submission median is 0.90 ms; these costs are already inside the completed-frame measurement.

This workload measures the Rust renderer, animation, sorting, light selection, uploads and GPU completion. Gameplay, physics, collision, audio, streaming, networking, shadow maps and browser composition need additional budgets. The 2.5D renderer uses billboards and bounded unshadowed lights. Reflections and transmission use a captured visible layer; offscreen objects, hidden faces and recursive reflections are unavailable. The composed Mirror demo has a dedicated reflection probe and is not what this World sweep measures. See [rendering details](../docs/world-rendering.md).

## Reproduce and verify

```sh
cargo build --locked --release --bin benchmark
python3 tools/benchmark_world.py --frames 180
python3 tools/benchmark_world.py --frames 180 --size 1920x1080 --output web/measurements/world-1080.json
target/release/benchmark --world --scene rain --lights 128 --reflection-steps 16 --counts 10000 --size 1920x1080 --frames 3000 --output benchmarks/sustained.json
```

All seven new native records use source fingerprint `fnv1a64:78e1e94c053a0ed8`. Every loaded asset now records its path, compressed byte count and content fingerprint. [Input verification](2026-11-09-world-inputs.json) also records SHA-256 values: source, catalog and all seven packages were unchanged before/after measurement, and all native load records match those exact inputs. Browser QA was closed during native measurement.

---

# Historical collections
# Mixed-world results — 2026-11-08

**10,000 animated objects at 1080p: 10.09 ms median, 10.39 ms p95** across 6,000 completed frames on Apple M5 Max / Metal. This includes a 128-light budget, 16-step reflections, wet floor, rain, and bloom.

These are native rendering/animation measurements, not complete game frame rates. Gameplay, physics, audio, streaming, shadows and browser composition are outside the timed workload. See [method](../docs/benchmarks.md) and [rendering limits](../docs/world-rendering.md).

## Comparable scene sweeps

Same six public packages, fixed 48px object canvases, no shrinking or culling. Each case has 12 warm-up and 180 measured frames. The floor is rendered from sprite tiles. All parts are captured for screen-space reflections. Base retains material shading, transmission, emission, bloom and the scene capture; it disables local lights and reflection rays. Lights adds a 128-light budget; full adds 16-step reflections. Each 64px tile shades with at most eight local lights. The 100-object cases have 100 active lights; larger cases have 128.

Short-run timings vary, including occasional inversions between profiles or resolutions. Do not subtract their medians as an isolated shader cost. The longer run below checks steady-state behavior.

### 720p (1280 × 720)

| Objects | Base median | + Lights median | + Reflections median | Full p95 |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 2.65 ms | 1.40 ms | 1.38 ms | 1.41 ms |
| 500 | 4.28 ms | 1.51 ms | 1.49 ms | 1.51 ms |
| 1,000 | 4.85 ms | 3.14 ms | 3.07 ms | 6.16 ms |
| 5,000 | 8.94 ms | 5.48 ms | 8.76 ms | 11.50 ms |
| 10,000 | 8.20 ms | 7.65 ms | 10.75 ms | 11.16 ms |

Raw data: [base](2026-11-08-world-1280x720-base.json), [lights](2026-11-08-world-1280x720-lights.json), [full](2026-11-08-world-1280x720-full.json).

### 1080p (1920 × 1080)

| Objects | Base median | + Lights median | + Reflections median | Full p95 |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 1.41 ms | 1.39 ms | 1.41 ms | 2.21 ms |
| 500 | 1.53 ms | 1.48 ms | 2.93 ms | 3.44 ms |
| 1,000 | 3.15 ms | 1.58 ms | 5.07 ms | 8.76 ms |
| 5,000 | 7.20 ms | 5.31 ms | 8.12 ms | 9.29 ms |
| 10,000 | 8.22 ms | 7.74 ms | 10.18 ms | 10.31 ms |

Raw data: [base](2026-11-08-world-1920x1080-base.json), [lights](2026-11-08-world-1920x1080-lights.json), [full](2026-11-08-world-1920x1080-full.json).

## Higher quality and longer run

The high-quality test uses 512 local lights and 32 reflection steps at 1080p. The spatial limit remains eight contributing lights per tile. All glowing objects still draw their own emission even when they are not chosen as light sources.

| Objects | Lights | Reflection steps | Median | p95 |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 512 | 32 | 2.93 ms | 3.02 ms |
| 5,000 | 512 | 32 | 7.71 ms | 7.92 ms |
| 10,000 | 512 | 32 | 11.24 ms | 12.54 ms |

[High-quality raw run](2026-11-08-world-1920x1080-stress.json).

The [6,000-frame run](2026-11-08-world-1920x1080-sustained.json) uses 10,000 objects, 128 lights and 16 steps. Median **10.09 ms**, p95 **10.39 ms**, slowest observed frame **13.47 ms**. CPU animation/batching median was 1.91 ms; buffer preparation/submission was 0.64 ms. These sub-medians should not be added to the completed-frame timing: they are already included.

This scene has 52,920 animated/terrain drawing instances (920 terrain tiles), seven draw calls, 2.00 MiB shared atlas textures and 51.44 MiB render targets. Per-frame part upload is 5.65 MiB.

The [browser check](2026-11-08-world-browser.json) held the same ~30 Hz headless display cadence from 100 through 10,000 objects at 1466 × 672 on hardware WebGPU. The identical cadence across counts does not establish maximum renderer throughput. Native tests wait for completed GPU work without a display cadence limit.

Reproduce:

```sh
cargo build --locked --release --bin benchmark
python3 tools/benchmark_world.py --frames 180
python3 tools/benchmark_world.py --frames 180 --size 1920x1080 --output web/measurements/world-1080.json
target/release/benchmark --world --scene rain --lights 512 --reflection-steps 32 --counts 1000,5000,10000 --size 1920x1080 --frames 180 --output benchmarks/stress.json
target/release/benchmark --world --scene rain --lights 128 --reflection-steps 16 --counts 10000 --size 1920x1080 --frames 6000 --output benchmarks/sustained.json
```

All current native runs use source fingerprint `fnv1a64:3193648571df1793`. The asset content hashes and complete settings are in every raw file. Historical experiments follow; their workloads and source revisions differ.

---

# Face-to-face battle runs

Two fully shaded Pokémon: Cryogonal Front and Charizard Back. Apple M5 Max /
Metal, release build, 1280×720, 12 warm-up and 90 measured frames. Includes
animation, four view captures, material shading, environment, bloom and blocking
GPU completion. This measures one battle; it is not a crowd throughput claim.

| Environment | Completed frame median | p95 |
| --- | ---: | ---: |
| [rain](2026-11-09-battle-rain-apple-m5-max.json) | 2.63 ms | 2.67 ms |
| [cave](2026-11-09-battle-cave-apple-m5-max.json) | 2.63 ms | 2.67 ms |
| [depth](2026-11-09-battle-depth-apple-m5-max.json) | 1.36 ms | 1.38 ms |

Each frame uses 30 visible part instances, 53 capture instances, and seven draw
calls. The JSON files include source fingerprints, settings and timestamps.

# Previous crowd depth and rain runs

That renderer revision traces reflections against two-sided sprite cards and adds
rain, wetness, wind and puddles. Apple M5 Max / Metal, 1280×720, release build,
12 warm-up and 90 measured frames per case. All scenes use the same mixed
Charizard / Glalie / Cryogonal workload.

**10,000 sprites with overlapping 64-pixel canvases:**

| Scene | Completed frame median | p95 |
| --- | ---: | ---: |
| [studio](2026-11-08-depth-update-studio-apple-m5-max.json) | 8.60 ms | 12.78 ms |
| [cave](2026-11-08-depth-update-cave-apple-m5-max.json) | 10.91 ms | 12.18 ms |
| [reflections](2026-11-08-depth-update-reflections-apple-m5-max.json) | 8.44 ms | 9.67 ms |
| [rain](2026-11-08-depth-update-rain-apple-m5-max.json) | 9.66 ms | 12.30 ms |
| [depth](2026-11-08-depth-update-depth-apple-m5-max.json) | 8.59 ms | 9.99 ms |

Rain and depth include a bounded four-view capture pass. The crowd shares the
same reflection sources; these are not 10,000 independent battles. The wet floor
reflection is limited to the single-sprite preview. Exact settings, capture
counts, six/seven draw calls, memory and source fingerprints are in each JSON.
These are native completed-frame timings, not browser display FPS.

# Earlier cave/bloom runs

The current renderer includes Cryogonal, HDR bloom, the cave, and reflection cards.
These are native Apple M5 Max / Metal release measurements at 1280×720, 12 warm-up
and 90 measured frames per case. Every scene uses the same mixed Charizard,
Glalie, and Cryogonal workload. At 10,000 sprites, 120,004 drawing instances are
uploaded in the recorded final frame (about 11 MiB).

**10,000 sprites, overlapping 64-pixel canvases:**

| Scene | Completed frame median | p95 |
| --- | ---: | ---: |
| [studio](2026-11-08-three-sprites-studio-apple-m5-max.json) | 10.27 ms | 10.46 ms |
| [cave](2026-11-08-three-sprites-cave-apple-m5-max.json) | 12.53 ms | 13.88 ms |
| [reflections](2026-11-08-three-sprites-reflections-apple-m5-max.json) | 11.60 ms | 13.20 ms |

The full JSON also includes fit-mode results, all smaller counts, and exact scene
settings. Cave lighting is bounded to sixteen emission sources from the first
eight sprites; this is not a test of 10,000 separate shadow-casting lights.
Six draw calls include both bloom passes. These serial, GPU-completed native
measurements are not browser display FPS; power state and other work affect them.
See [method](../docs/benchmarks.md). Private game art is not included.

[Current Copperbot run](2026-11-08-copperbot-bloom-apple-m5-max.json) uses the same
pipeline with the repository's original redistributable fixture.

# Earlier baseline (before cave and bloom)

These JSON files contain measurements, not artwork. Both use the earlier four-draw-call Rust
renderer at 1280×720 on Apple M5 Max / Metal, in release mode, with 12 warm-up
and 90 measured frames per case. See [the method](../docs/benchmarks.md).

- [Gen 5 studies](2026-11-08-gen5-apple-m5-max.json): a mix of Charizard and
  Glalie. 10,000 sprites / 110,000 parts: 8.14 ms median (8.42 ms p95) in fit
  mode; 8.41 ms median (8.77 ms p95) with overlapping 64-pixel canvases.
  Reproducing these assets requires the user's local source SPUs; none are
  included in this repository.
- [Original Copperbot](2026-11-08-copperbot-apple-m5-max.json): the fully
  redistributable fixture in `web/examples/`, including a parented rig.
  10,000 sprites / 30,000 parts: 4.12 ms median in fit mode; 4.99 ms median
  in overlap mode. This run has greater scheduling variance; retain the p95
  and full range rather than treating the median as a guarantee.

The source fingerprint identifies the Rust runtime, shaders, benchmark code
and dependency lockfile. It is a reproducibility label, not a security digest.
These are initial single-machine baselines, not cross-device performance claims.
