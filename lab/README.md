# Format lab

A tour of what the SPU format does, on real sprites: twelve Black 2 / White 2
battle sprites and four from Uranium, lit in six places (studio, cave, rain,
field, snow, deep water). The page has one tab per thing the format does:
light, materials, surface, glow, parts, motion, views, looks, and the stored
sheets.

The page reads `.spu` files with its own reader (`page/spu.js`), which shares
no code with the Rust library. Drop any `.spu` file on the stage.

```sh
cargo build --release --bin spu      # in the repository root: draws each sprite's button
cd lab
uv run --no-project --with pillow python sprites.py   # out/spu/*.spu -> out/sprites.json
python3 build.py                     # -> out/spu-material-lab.html, out/material-spheres.html
python3 standalone.py                # -> out/html/*.html, complete files that open from disk
open out/html/spu-format-lab.html
```

`out/` holds game art and is not committed. The page's sixteen sprites are lit
`.spu` files, `out/spu/<id>.spu` (set `SPU_SPRITES` to read them from another
folder), listed in `sprites.py`.

## Making an SPU from an existing sprite

Most sprites will be made by taking apart art that already exists:

1. **Split it into parts and animate them.**
2. **Take the lighting out.** For each surface, find its shades and which one
   is the base.
3. **Work out which way each pixel faces,** so that light from the upper left
   gives back the shading the artist painted.
4. **Decide what each surface is made of:** its finish and body, and what
   glows.
5. **Do the same for the back view.** If the game has none, draw one.

This is what the editor, [Smeargle](https://smeargle.prof64.net), is for. Step 3
is automatic there: its baker is a port of the one that lit the sprites here.

## Files

Still pictures can be made from the built page: `node readme.cjs` captures
them and `python readme.py` lays them out into `../docs/images/`. The capture scripts
(`readme.cjs`, `shot.cjs`) drive a headless Chromium through playwright-core;
set `PLAYWRIGHT_CORE` and `CHROME_PATH` to where yours are.

| File | What it does |
| --- | --- |
| `sprites.py` | The page's sprites: each one's title, the place it opens in and the views it shows. Packs their files into `out/sprites.json`. |
| `page/spu.js` | Reads `.spu` files in the browser and lays a sprite out for drawing. Checked against `spu render`: Charizard, Klinklang and Arceus match on every pixel; Jellicent differs on 8 of 2,603, at the edge of a turned part, where the GPU and the reference round differently. |
| `page/` | The WebGL page. `shaders.js`: the six places and the lighting. `app.js`: drawing a frame. `balls.js`: the two-dial material map and its test ball. `spheres.html`, `spheres.js`: the material spheres page. `ui.js`: the nine tabs; each entry in `PANELS` is one tab. `index.html`: layout and styles. |
| `build.py`, `standalone.py` | Assemble the pages into single files. |
| `readme.cjs`, `readme.py`, `shot.cjs` | Headless captures of the page, and still pictures made from them. |
| `video.cjs`, `video.py`, `video_numbers.py`, `video_looks.cjs` | The tour video; [VIDEO.md](VIDEO.md) says how it is made and changed. `video.cjs` holds the storyboard (each scene: chapter, length, what the page shows, its note and caption) and captures every frame. `video.py` lays them out with labels at integer pixel scales, adds selectable English captions and encodes `out/spu-tour.mp4` with ffmpeg. `video_numbers.py` draws the palette and material numbers and the material lists; `video_looks.cjs` assembles Jellicent's two variants from one sheet. The page can lay a sprite's pieces out as a sheet and move them between sheet and pose (`S.sheet`, 0 to 1), which the video uses. |

## How the facing data was made

No hand painting. The sprite is posed (at the tick that shows most of each
drawing) and the assembled picture is inflated with a Poisson solve, so a disc
becomes a hemisphere: the silhouette gives the overall roundness, and each area
enclosed by outline swells on its own, so drawn lines become creases. Each
drawing keeps the piece that fell on it; texels hidden under another part copy
their nearest visible neighbor.

Shaping each drawing by itself was tried first and is wrong: it rounds off edges
that are not edges (a neck running into a body, an eye patch on a head), which
shows up as dark bands. Two more rules came out of the same fix:

- A line counts as an edge only if it is part of the outline. A pupil or wrinkle
  standing alone is a mark and does not dent the surface.
- After the shape is blown up, every pixel is refit to the painted shading:
  it is turned toward or away from the upper left just far enough that light
  from there, on the material's base shade, picks the shade the artist
  painted. A pixel the shape already gets right is left alone. This is what
  lets light alone redraw the art: lit from the upper left, flat base colors
  land on the painted shade for every shaded pixel, against about 24% for the
  blown-up shape alone. It replaced an earlier rule that left painted shading
  out of the facing data.

A sprite whose contour lines are not black (Arcanine's are dark brown) needs
the baker told that color, and markings in the outline color (stripes) need a
material of their own.

## How the light stays on the palette

Every material row lists its shades, dark to light. A light moves a pixel up or
down that list by how much more or less it faces the new light than the art's
light, then tints it. Nothing is blended between shades.

## Adding a place

A place is an entry in `PLACES` in `app.js` (its light, colors and where the
idle light wanders) plus a branch in the room shader `FS_BG` in `shaders.js`,
and anything drawn in front of the sprite (rain, snow, bubbles) in `FS_COMP`.
A place with no `lamp` hands the pointer to its sun or moon through `live()`.

## What each test sprite is there for

| Sprite | Shows |
| --- | --- |
| Charizard | light through a wing, a flame that lights the room, two animations |
| Magneton, Klinklang | metal; Klinklang's gears turn all the way round, so facing data turns with a part |
| Cryogonal, Kyurem | ice: facets, glitter, the room seen through it |
| Arcanine, Whimsicott | fur and fluff |
| Rotom, Chandelure | glow, glass |
| Arceus | 34 palettes in one file (17 types, each with a shiny) |
| Jellicent, Reuniclus | jelly; Jellicent's female look swaps every drawing |
| Halloween Titanice | soft see-through pixels, a 90-shade material |
| Christmas Corsoreef | 172 colors, 37 parts |
| Nuclear Gyarados, Metalynx | glow on a dark body; metal beside hide; a second animation |

The page shows every animation a view has (`idle`, `alert`, `alternate-1`) and
every look: palette swaps, drawing swaps, or both.

## Big palettes and soft pixels

Four Uranium sprites test what the 16-color Black 2 / White 2 ones cannot:
Halloween Titanice (160 colors in its front view, 138 of them partly
see-through), Christmas Corsoreef (192 colors), Nuclear Gyarados and Metalynx.

- **Shades were gathered automatically.** A surface was named by a few sample
  colors or by which drawings it is; every color the sprite uses joined the
  surface it most resembles, and each surface's shades were sorted dark to
  light. Lists of over a hundred shades work, and the light may move a pixel
  about half way along a long list.
- **Opacity is per pixel.** Solid pixels and soft ones are drawn as two
  layers; a soft pixel blends over the solid one beneath. Soft over soft keeps
  only the front one.
- **A material can have no surface** (mist): it stays flat and does not join the
  shape that is blown up.
- The page holds at most 256 colors and 64 material rows per sprite. A ceiling
  above 256 colors is not tested here.

Color alone cannot always tell surfaces apart: on Christmas Corsoreef the
whites of the tinsel, the candy canes and the eyes all land in one material.
A real editor needs per-drawing hints or painting on top of the automatic pass.

## Adding a sprite

Light it in Smeargle, put the file at `out/spu/<id>.spu`, and add a line to
`SPRITES` in `sprites.py`.

A material's shades must all be the same surface. Rotom's eye once listed iris
blue and eye white together, and the white shine on its head was filed under
it, so a dimmer light slid the shine down into blue. Give each surface its own
material, and where one color (white, here) belongs to different surfaces in
different drawings, give those pixels different materials.

## A material is two dials

`page/balls.js` defines a material as two numbers, and the sprite shader
(`page/shaders.js`) derives the six values the lighting works on from them:

- **finish**: -1 fuzzy, 0 matte, +1 polished. What the surface is like.
- **body**: -1 metal, 0 solid, 0.5 translucent, +1 clear. What happens to light
  that gets past the surface.

Every combination means something (matte clear is frosted glass, fuzzy
translucent is backlit fluff), which the six separate values did not manage:
metal cancelled see-through, coat duplicated low roughness, fuzz switched the
highlight off. The seven sprites render the same from two dials as they did
from six. Given up: a glossy coat over a matte base, anything both metal and
see-through, hair-like shine on fuzz, and rainbow sheen (never had it).
Glitter is not stored: it appears where a polished body is between translucent
and clear, which is ice, and not on glass or wax.

## What makes ice read as ice

No extra fields: it follows from see-through, low roughness and light passing
through. The room shows through bent, with each color bent a little
differently; light that enters comes out on the side facing away from the lamp;
edges turned away from the viewer mirror the sky and the lamps; and single
texels flash as the lamp or the part moves. The glitter is see-through times
light-passing-through times smoothness, less under a coat, so clear glass
(nothing inside it) and wet slime have none and ice and snow do, with no value
of its own.
