# Prototype 0 — implementation notes

> Historical. The format is now specified in [spec.md](spec.md) as SPU
> version 1. The document section below describes the earlier prototype; the
> rendering sections still describe the viewer.

These record the experiment implemented in code. They are not a normative
specification, a stability promise, or a decision on the permanent name.
The temporary `.sprite` extension is distinct from legacy Smeargle `.spu` v0.

## Document

ZIP contains `sprite.json` and relative PNG resources. `format: "sprite-lab"`,
`version: 0`, and required capability `core-0` identify this implementation.
Unsupported versions or required capabilities fail explicitly. Optional unfamiliar
object properties are preserved where the model exposes flattened metadata;
unfamiliar archive resources are retained on save. This does not guarantee safe
structural editing of unknown extensions. Keys and the canvas are closed records
in this prototype; a complete forward-compatible editing contract is future work.

Materials, views, parts, drawings and clips have stable string IDs. The loader
checks uniqueness, references, acyclic parenting, ordered finite keyframes,
resource bounds and matching image dimensions. References are compiled to indices
once. File paths are archive keys; nothing is extracted from a dropped file.

A view contains a canvas, parts, drawings and clips. Parts form an optional
hierarchy. Drawings have part-local origins and four aligned images: color,
normal, surface, and emission. A track targets one part and can key its drawing,
visibility, position, rotation, scale, draw order, opacity and emission multiplier.
Both frame-by-frame and articulated animation use the same drawing-switch tracks.

Clock units are ticks with a file-defined rate. The viewer can sample at integer
ticks (default for the Gen 5 studies) or at fractional ticks. Step, linear,
quadratic ease-in/out and cubic smoothstep interpolation are implemented. Before
a channel's first key, its first value is held; absent channels use defaults.
Looping clips wrap at their length; nonlooping clips hold their last frame.
Numeric defaults are position/rotation/z zero and scale/opacity/emission one;
visibility defaults true and drawing defaults absent.

Coordinates have +X right, +Y down, clockwise angles in degrees. Parents compose
affine transforms, including resulting shear. Parent visibility and opacity
propagate. Layer order sorts by z, then original part order for ties. Cross-sprite
order is the instance submission order. Normal vectors use the inverse transpose
of the part transform; negative scale mirrors their directions correctly. Normal
transformation currently treats the sprite's Z scale as one.

## Materials and rendering

Color and emission images use an sRGB-like transfer (`pow(2.2)` in this prototype).
Normal and surface maps are linear data. Normal RGB encodes XYZ, +X right, +Y up,
+Z toward the viewer. Surface RGBA stores roughness, metalness, transmission and
thickness **multipliers**. Missing normal is flat; missing surface is white;
missing emission is black. Factor defaults live in the Rust model.

The shader uses a metallic/roughness GGX-style direct highlight and an artistic
reflection multiplier. Normals redirect both direct lighting and scene-probe
reflection. Reflections now intersect finite-depth scene cards and select the
front/back view from the ray’s side of approach. Transmission samples the procedural stage with a normal/IOR/thickness
offset and a simple color tint. Opacity uses ordinary source-over composition.
The two are independent. Drawings use nearest-neighbor sampling and atlas gutters.

Materials are shaded and composited in a linear RGBA16F intermediate target. A
separable bright pass produces a quarter-resolution bloom texture. It includes
any HDR highlights, not only emission. A single final display pass adds the
controllable halo, then applies exposure, an ACES-fitted tone curve, and display
encoding. Original-color and map-debug modes bypass the tone curve. This keeps
alpha composition in linear light and prevents per-layer tone-mapping errors.

Six draw calls: stage, all sprite parts as instanced quads, foreground test
objects, two quarter-resolution HDR bloom passes, and display conversion. Rain
and depth, plus the Battle layout in every environment, add one draw capturing four animated sprite views into a 1024×256
RGBA16F texture. Captures are cleared each frame, including empty frames. The
browser uses WebGPU; native tests and benchmarks use the same wgpu implementation and WGSL (Metal on this Mac).

World uses a full-scene color/depth capture and tiled local lights rather than the
four pair captures. See [world rendering](world-rendering.md) for the current
algorithm, quality bounds and memory accounting.

### Deliberate limits

- This is a 2.5D material experiment. Reflections and refraction sample the
  procedural scene and, in the depth and Battle experiments, animated front/back cards.
  Reflection rays intersect flat planes; they do not intersect reconstructed
  Pokémon geometry. There is one bounce, with no recursive reflections,
  volumetric scattering, or physically calibrated thickness.
- Emission also lights the procedural cave: each drawing’s emission-map centroid,
  mean linear color, and luminous area are computed once; animated transforms
  and material/track factors move and scale the derived lights. Transparent
  texels do not contribute. The sixteen strongest drawing lights among the
  first eight sprites are retained. Falloff and cave geometry are artistic
  approximations, without shadows or light transport between sprites. HDR
  bloom is a separate screen-space effect; neither effect changes the saved
  sprite’s emission map.
- The light-card room uses HDR panels on a finite-depth plane. In the depth
  scene, the correct front/back capture is selected at the ray hit; roughness
  filters that captured image. The sample uses Front/Back view names as a lab
  convention, falling back to the first view if a direction is absent. This
  does not standardize game-facing directions or a scene format.
- Rain wetness, wind and depth placement are preview scene settings. They are
  separate from the editable sprite’s materials and animation. See
  [depth and weather](depth-and-weather.md).
- The source art already contains painted shading. Estimated normal maps cannot
  recover the original surface geometry or remove that shading.
- The UI changes material factors, selects clips and controls playback. It is a
  viewer and test bench; it is not yet a drawing or keyframe editor.
- Rigs have affine transforms, not mesh deformation/constraints. Clip blending,
  action/state/parameter composition, profiles, events, palette editing and
  additive animations are future design work.
- The prototype accepts at most 32 MiB compressed, 64 MiB archive data, 32 loaded
  assets, 256 parts/view, and a combined 1024×4096 atlas. These are implementation
  guardrails, not proposed permanent format limits.

## Path toward a formal standard

Before freezing a specification: settle the logical model, units and color
spaces; define profiles and extension ownership; define compatibility/error
behavior; publish a schema plus normative prose; provide a conformance corpus
and reference images; test independent implementations. Names, codecs and an
optimized runtime representation remain separate choices.

The source package should remain editable. A future compiled game representation
may use packed tracks, texture compression or offline atlases without replacing
the source or forcing every editor to use the runtime representation.
