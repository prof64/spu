# Showcase artwork

The new hero artwork was created with the built-in OpenAI `image_gen` tool on
2026-11-09, then converted to native pixels. It is original generated artwork,
not extracted game art or a claim of hand drawing. Exact prompts and original
transparent masters are in `web/art-source/prompts.json` and `*-generated.png`.

| Package | Views | Clips | Material demonstration |
| --- | --- | --- | --- |
| `web/examples/showcase-cindermaw.sprite` | Front, Back | Idle, Surge | Dark volcanic armor with spatially masked magma emission; animated emission drives scene lighting |
| `web/examples/showcase-prismcoil.sprite` | Front | Idle, Surge | Broad teal/violet glass facets with authored face normals, transmitting interiors and less transmitting bright bevels |

Both packages have a **192×192 canvas** and ground anchor **[96, 178]**. Cindermaw
occupies approximately 176×109 pixels in Front; Prismcoil approximately 158×160.
The two Cindermaw views are separately generated drawings of the same character:
the rear view shows the armored hindquarters and the head facing away. Back is
not a mirrored Front. Prismcoil faces left; Cindermaw faces right.

Each view contains a complete body drawing plus eight independently animated
ember/crystal effect parts. Idle has gentle breathing and emission pulses. Surge
compresses then raises the body, intensifies the emission and expands the effect
motion before returning. This is a body/effects prototype, not a fully separated
limb rig. The 180-tick loops run at 60 ticks per second. Crystal motes orbit the
body; embers rise from the spine. The live renderer applies emission and optical
maps; the PNG/GIF pose reviews show unlit artwork with actual transforms/opacity.

## Native conversion and editing

Run `python3 tools/make_showcase_art.py` with Pillow installed. Rebuild uses the
retained masters, with no generation or network requests. It never rewrites the
older example packages. Color is selected from explicit material-role palettes
using nearest palette mapping followed by majority-color voting per native cell.
Alpha votes separately and is binary. There is no dithering or image stretching.
The nearest-neighbor review preserves the same native pixels used by the package.

`web/art-source/*-palette.json` retains the conversion palette and its order.
`*-pixels.json` is an exact editable text map of the native body pixels, with a
symbol-to-color dictionary. `*-native.png` is the native transparent drawing.
The exported package contains all independent color, normal, surface and emission
PNGs, editable tracks and material properties. The readable full manifest is
also saved in `web/art-source/showcase-*.json`.

To edit a material, change the corresponding `mats` parameters or masks in the
builder. The optical masks follow the actual artwork's color regions. These are
artist-authored approximations, not reconstructed geometry: violet and teal glass
planes have distinct normals; pale glass edges transmit less; volcanic armor and
claws do not emit while molten orange/gold regions do. Custom game fields include
heat and material identity. The root `lighting` extension controls only scene spill:
Cindermaw uses power 5.0 / radius 1.5; Prismcoil uses power 0.15 / radius 0.8. Native pixels can be changed directly and imported by
an editor; regenerating from the masters will replace those manual native edits.

## Verification

All three native drawings were reviewed at integer 2× scale; their text maps
round-trip to exactly the same RGBA bytes. Every exported drawing has aligned
color/normal/surface/emission dimensions. Both archives passed the actual Rust
parser and native GPU renderer. Shaded and source-color snapshots were checked
for silhouettes and spatial emission. Final composition and material intensity
are controlled by the showcase scene settings.

The package license notes dedicate the original generated artwork and authored
data under CC0 to the extent copyright applies. No game artwork is included.
