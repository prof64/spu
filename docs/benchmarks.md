# Current World comparisons

Run `python3 tools/benchmark_world.py` after building the native benchmark. It
loads the current seven-package catalog: five populated object types, the floor
tile, and a showcase-only hall plate. The plate is resident but excluded from
the population scene. Objects and floor tiles use fixed 48-pixel canvases in a wet scene. Three identical population sweeps isolate base material rendering, local
lights, and lights plus screen-space reflections. The requested light budget is
128, the per-tile budget is eight, and reflection quality is 16 steps. Every raw
file records actual active lights, source/asset fingerprints and completed-frame
median/p95. The browser Performance dialog reads the combined numeric output.

See [World rendering](world-rendering.md) for inclusion/exclusion and quality
limits. These timings include capture of the full scene and real local-light
selection/binning, unlike the older shared-card crowd experiment below. The
first 12 frames warm up; the default is 90 measured frames. The recorded World
comparison uses 180 measured frames per case, plus a separate 3,000-frame
sustained case for the November 9 collection (the November 8 run used 6,000 frames) at 10,000 objects. `--frames` accepts 5–36,000 frames and rejects
out-of-range values. Browser QA is closed during
native measurements. Each frame blocks for GPU completion, so this measures
serial frame throughput rather than pipelined CPU/GPU overlap.

The fixed 128-light setting is a budget, not a promise that every glowing object
casts a light. Drawn emission/bloom still covers all luminous objects. Physics,
AI, collision, streaming, networking, sound and browser compositor/display timing
are not included. Compare rendering headroom rather than claiming a complete
game's frame rate from these numbers.

# Historical crowd benchmark method

The native executable measures the same Rust animation preparation, batch
construction, wgpu pipelines and WGSL used by the browser. Run a release build;
debug numbers do not characterize game performance.

```sh
cargo run --release --bin benchmark -- \
  --frames 90 --counts 1,100,1000,5000,10000 --size 1280x720 \
  --output web/fixtures/benchmark.json --snapshots artifacts/renders
```

Pass one or more `.sprite` paths before the options to choose a workload. Those historical runs passed local study files or Copperbot explicitly. The
current executable defaults to the public catalog. Texture uploads and pipeline compilation happen before steady-state
measurement. Twelve warm-up frames precede each case. Independent animation
phases exercise key lookup and drawing changes; sprites do not share an evaluated
pose. The full material shader is active in both layouts.

Use `--scene cave`, `--scene reflections`, `--scene rain`, or `--scene depth` to measure the new experiments; the exact settings are saved with the results. The cave evaluates up to sixteen animated emission lights from the first eight sprites. That bounded lighting workload is intentional; these numbers do not represent thousands of independently shadowed lights. Bloom is active in every scene. Store scene runs at `web/fixtures/benchmark-cave.json` and `benchmark-reflections.json` for the website’s selector.

## Measurements

| Field | Includes |
| --- | --- |
| Archive/texture decode | ZIP read, JSON validation, PNG decode, reference compilation and CPU atlas construction; excludes file I/O and initial GPU upload |
| Animation and batch | Track sampling, hierarchy evaluation, layer ordering and instance buffer construction |
| Upload and submit | CPU time writing buffers and encoding/submitting commands; not GPU execution time |
| Completed frame | Animation, batch, upload, submit, and a blocking wait for the GPU to finish the submitted frame |

Measurements use a monotonic clock and report median, p95, min, max and sample
count. Completed-frame time is **not an isolated GPU timestamp**. GPU waiting
serializes frames; pipelined game throughput can differ. Native rendering is
offscreen without v-sync. Browser display cadence is measured separately and is
limited by refresh rate, page visibility and scheduling.

## Workloads

- **Fit:** all sprites remain on screen. More sprites means smaller sprites;
  this mainly increases pose, batching, upload and triangle costs at bounded
  screen coverage. Do not interpret it as 10,000 full-size characters.
- **Overlap 64 px:** all sprite centers are in the viewport, with a fixed
  64-pixel canvas height. This also increases overlap and fragment work.

There are six main draw calls regardless of instance count: backdrop, batched parts,
foreground probes, two quarter-resolution HDR bloom passes, and display conversion. Texture atlas bytes are reported
separately from the RGBA16F framebuffer plus two quarter-resolution RGBA16F buffers and per-frame instance upload bytes.
Rain and depth add one capture draw for four animated front/back views and a fixed 2 MiB RGBA16F texture. Captures are evaluated, uploaded, and rendered inside the timed frame. The reflection scene shares these bounded source cards across a crowd; this is not a benchmark of 10,000 separate battle scenes. The single-sprite puddle reflection is disabled for crowds.

The per-frame upload is 112 bytes per visible drawing instance; the actual part count is recorded for each workload. This upload cost is a candidate for
future GPU pose evaluation or more compact per-instance data.

The former browser benchmark control sampled 90 display frames per count after
warm-up. The current Performance dialog shows completed native measurements;
separate browser QA records display cadence without treating `requestAnimationFrame`
or CPU submission time as GPU completion.

## Reproduction and interpretation

JSON records the hardware/backend, resolution, build mode, workload, file sizes,
sample count, shared texture memory, framebuffer size, source fingerprint and
measurement method. [Recorded local runs](../benchmarks/README.md) include an
original fixture that can be reproduced without game artwork.
Results vary with power mode, thermal state, other workloads and adapter. These
three sprites establish an initial scale test, not a universal performance guarantee
for every sprite, material, engine or device. The new World runs add diverse public fixtures and local lights. Startup GPU
upload timing, timestamp queries and mobile hardware remain useful next experiments.

## Face-to-face pair

`--battle` measures exactly two fully shaded animated rigs with independent Front
and Back drawings, in the same placement and camera as the Battle tab. Supply
opponent then player asset paths. The option replaces crowd counts with one
two-sprite case, includes all four color/emission captures, and skips overlap
cases. Use `--scene rain`, `--scene cave`, or `--scene depth` for the respective
experiments. Results label the layout `battle`; they do not imply crowd scaling.
