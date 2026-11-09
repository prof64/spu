# Included examples

The visible collection contains five original examples. Their images were made
with the built-in OpenAI image-generation tool, then converted to native pixel
grids. The generated masters, exact prompts and conversion data are retained;
these are not claimed as hand-drawn sprites or extracted Pokémon art. Package
license notes dedicate the original generated artwork and authored data under
CC0 to the extent copyright applies.

| Package | Views / clips | What it demonstrates |
| --- | --- | --- |
| `web/examples/showcase-cindermaw.sprite` | Front, Back / Idle, Surge | Separate front/back drawings; masked magma emission, breathing and animated embers; emitted light follows the animation |
| `web/examples/mirror-knight.sprite` | Front / Idle, Guard | Separate convex shield, armor/cloth and luminous lenses; polished metal, rough cloth and a grounded brace |
| `web/examples/showcase-prismcoil.sprite` | Front / Idle, Surge | Broad glass planes, authored facet normals, transmitting interiors, less transmitting bevels and crystal motes |
| `web/examples/dungeon-scout.sprite` | Eight directions / Idle, Walk | Distinct directional drawings, body/feet/lamp layers, four held walking poses and an animated lantern |
| `web/examples/obsidian-hall.sprite` | Room / Still | Opaque 640×360 environment plate with flat normals, matte stone, no emission and room for live-lit foreground actors |

Cindermaw's Back is a separate rear drawing, not a mirrored Front. Both hero
creatures use 192×192 canvases and anchor `[96,178]`. Mirror knight uses a 192×192
canvas and anchor `[96,176]`. Dungeon scout uses a 72×72 canvas and anchor `[36,64]`;
its directions are South, SouthEast, East, NorthEast, North, NorthWest, West,
SouthWest. Obsidian hall uses anchor `[320,360]` and is fitted as one room plate.

The animation scope is explicit: Cindermaw and Prismcoil have a body drawing plus
independent effects, rather than complete limb rigs. Mirror knight braces its
assembled figure. Dungeon scout alternates foot layers with a small body bob,
rather than swapping newly drawn full-body walking cels. Detailed art contracts
and review notes are in [hero artwork](showcase-art.md), [Mirror knight](mirror-knight.md)
and [Dungeon scout](dungeon-scout.md).

## Materials and metadata

Each `.sprite` retains its editable manifest, separate PNG layers, matching color,
normal, surface and emission maps, and animation tracks. Alpha is binary for crisp
pixel boundaries; material transmission is separate. Normal fields, thickness,
reflection masks and emission masks are authored approximations, not recovered
3D geometry or physical measurements.

The catalog is `web/examples/catalog.json`. Root `role` fields distinguish actors,
props and terrain; `showcaseOnly` keeps the room plate out of the population test.
Unknown fields survive load/edit/save and are exposed through the browser metadata
API under `extras`. Custom `properties` hold example game data. The optional root
`lighting: {power, radius}` extension tunes scene spill derived from emission;
it does not change the visible emission map. These extensions are prototype
conventions, not a frozen standard.

## Source and rebuild

Generated source images, exact prompts, native drawings, conversion palettes and
readable manifests live in `web/art-source/`. Mirror knight also records its
manifest in `tools/example_art/mirror-knight.json`. The conversion palettes retain
their order, and the hero native text maps preserve exact editable pixels.

```sh
python3 tools/make_showcase_art.py
python3 tools/make_mirror_knight.py
python3 tools/make_obsidian_hall.py
uv run --with numpy --with pillow python tools/make_dungeon_scout.py
```

The first three scripts require Pillow. Rebuilding uses the retained source
images and does not call an image-generation service or the network. Editing a
native image and then rebuilding from its master can overwrite that native edit;
change the saved source or builder when a change should survive regeneration.

Review images and GIFs beside the packages show actual exported drawings and
poses. Artifact-specific notes record native-grid inspection and Rust parser,
atlas and GPU validation. Final lighting and scene composition are controlled by
the renderer and viewer, not painted into the source previews.

## Retained fixtures

The original six examples remain unchanged as legacy files: Emberwing, Moss
Scout, Lantern, Glass vial, Moss stone and Tin Beetle. Their deterministic native
art builder is `tools/make_examples.py`; readable manifests are in
`tools/example_art/`. Moss stone and Lantern remain hidden catalog entries because
the workload and showcase use them as support assets. Copperbot remains a stable
legacy test fixture outside the catalog. Historical benchmark files retain asset names and sizes. New runs additionally
record package content fingerprints; earlier records do not include these hashes.

The optional local Pokémon studies remain ignored under `web/fixtures/` and are
not included in the distributable artwork collection.
