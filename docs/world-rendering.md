# Scene rendering and population workload

The renderer uses the same `.sprite` package, animation, atlas and material paths
for composed demonstrations and the population test. Objects are ordered by their
ground point; each rig retains its part order. Stable object IDs preserve animation
phases when depth order changes.

## Composed demonstrations

`world: true` with `showcase: 1..4` chooses a composition from `src/world.rs`:

| Value | Scene | Interaction |
| --- | --- | --- |
| 1 | Firelight | Cindermaw and a knight share the room; emission and Surge change the light cast by the creature |
| 2 | Mirror | An animated source faces the knight's convex shield; moving the source changes its reflected image |
| 3 | Glass | An opaque creature moves behind Prismcoil; transmission reveals the captured subject through the glass |
| 4 | Dungeon | The scout follows a route, selecting directional views and Walk while carrying its lantern |

`showcaseMotion` ranges from zero to one and controls source position or route
phase. The room is a single Obsidian hall sprite, fitted to cover the viewport.
Its architecture is authored into that environment plate; its illumination is
computed at runtime. The two plinths and foreground actors form a deliberate
composition. The room's `showcaseOnly` field excludes it from the tiled workload.

`showcase: 0` retains the population test. It uses the repeatable Moss stone floor,
fixed-size actors and props, independent phases, and directional walking where
available. Props stay in place while actors move. No automatic shrink-to-fit,
frustum culling or LOD reduction hides the growing overlap.

## Emitted lighting

Emission-map analysis records a drawing's luminous centroid, radiance and emitting
area. Pose, visibility, opacity, material factors and emission animation affect
the derived light. Per-object selection ranks integrated luminous area multiplied
by emission strength and transformed area, before applying the minimum display
light radius. A broad luminous body can therefore contribute instead of a tiny
bright mote. Turning emission off removes the corresponding light.

An optional root `lighting` object is compiled once with the asset. `power`
(default 1, clamped 0–16) scales scene spill; `radius` (default 1, clamped 0.1–4)
scales the local-light range. These do not brighten the visible emission map.
The global `spill` setting also scales world local-light power. Thus source art
can retain bright magma detail while separately controlling how far and strongly
that magma illuminates the environment.

If more objects emit than the requested budget, selection is distributed through
the depth-ordered luminous objects. Every object still draws its own emission and
bloom. The total light budget is at most 512. CPU spatial binning retains up to
eight strong nearby lights per 64×64 screen tile; fragment shading reads this
bounded list. Terrain, props and actors receive the same local lights and material
highlights. These are unshadowed lights. The global movable light is separate.

## Transmission and the opaque capture

An extra instanced draw writes color/emission (RGBA16F) and depth/object ID (RG32F)
for the scene's opaque layer. Transmitting pixels are excluded from this source
capture, so a glass object does not replace the very subject it needs to reveal.
The source includes the current animated drawings, ambient/base color and emission;
it does not contain recursive reflections or the complete local-light shading pass.

Transmission samples this capture with a normal-dependent screen offset, with a
stronger bend in the composed Glass demonstration. Self hits are rejected. This
lets the opaque moving creature remain visible behind the crystal, rather than
substituting a generic procedural background. It is a single opaque layer and a
screen-space approximation: stacked transparent volumes, physical absorption and
multi-layer refraction are not simulated.

## Two reflection methods

Ordinary world reflections trace a configurable 0/8/16/32-step ray through the
captured depth field. Objects are upright billboards at their ground depth; the
floor is a sloped plane. Normal maps change the ray direction, self hits are
rejected, and roughness filters successful lookups. Roughness at least 0.85 skips
coherent reflection rays. Rain lowers the effective floor roughness so the wet
workload exercises full-screen floor reflections. Rays stop at screen boundaries.
Hidden sides, offscreen geometry, recursive reflections and shadows are absent.

The **Mirror showcase** instead uses a focused convex probe sampling the actual
animated opaque source capture. Shield normals project and curve that live image;
roughness controls its filtering. The image is not painted onto the shield, and
source movement and animation change it. This deliberately arranged probe is not
a general screen-space ray intersection or a physically complete environment map.
It is confined to the composed mirror scene; the population test uses the ordinary
screen-space reflection path.

The separate Pair inspector retains Front/Back captures and finite-depth cards for
the face-to-face directional reflection experiment.

## Workload, storage and interpretation

The world path uses seven draws: opaque color/depth capture, stage, instanced
materials, foreground/weather, two bloom passes and display conversion. More
objects increase animation, sorting, upload, capture and shaded pixel overlap.
In the fixed-size test, 48px floor tiles add `ceil(width/48) × ceil(height/48)`
material instances. The composed scene uses one environment instance instead.

Drawing instances are 112 bytes. Atlas images are decoded and uploaded once.
The viewport capture uses 16 bytes per pixel; HDR frame/bloom and fixed Pair
capture allocations are also included in `framebufferBytes`. Light lists use nine
u32 values per tile. Completed native frame timing includes animation preparation,
sorting, light selection/binning, buffer uploads, draw submission and GPU completion.

Earlier numeric results used the smaller legacy collection and older capture
behavior. They remain historical records; new artwork, map coverage and source
fingerprints must accompany fresh measurements. Native rendering timings do not
include game logic, physics, audio, networking, streaming or browser composition.
A full game needs headroom for those systems, and richer shadowing or reflection
quality would add further cost.
