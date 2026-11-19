# The tour video

`out/spu-tour.mp4` is made from the demo page, in two steps. It is not
committed: the README plays a copy uploaded to GitHub (drag the file into any
issue or comment box on github.com and use the link it gives).

```sh
cd lab
python3 build.py && python3 standalone.py      # only if anything under page/ changed
node video.cjs                                  # capture every frame from the page: about 35 seconds
uv run --no-project --with pillow --with numpy python video.py 692 1780   # lay out, caption, encode; the numbers save those frames as out/video/still-N.png
open out/spu-tour.mp4
```

`video.cjs` needs playwright-core and a Chromium: set `PLAYWRIGHT_CORE` and
`CHROME_PATH`. `video.py` needs ffmpeg. The page needs `out/sprites.json`,
which `export.py` makes (see [README.md](README.md)).

## Where things are

| To change | Edit |
| --- | --- |
| The order of scenes, how long each lasts, its visible explanation, optional caption and chapter | `SCENES` in `video.cjs`. One entry per scene. Use `note` for the on-screen explanation and `cap` for a longer selectable caption. |
| What a scene shows | The scene's `s`: the state handed to the page (sprite, place, and the switches below). |
| Where the light is | `STILL` in `video.cjs` sets each place’s default. The shading scene uses `swing`, which starts and ends at that default. The glow scenes share a cave pose with its torch out of the picture. |
| Short labels beside the sprites | `overlay` in `video.py`. A scene asks for one with `ov`. |
| The numbers moving from a key to the sprite | `video_numbers.py` sets the paths and timing. `info` in `video.cjs` reads the displayed palette colors and the material IDs for the held pose. |
| Materials on other sprites | `MATS` in `video.cjs` lists each cell: sprite, place, the material's name in that sprite's file, and its label. The scene's `grow.at` says when each cell arrives. `growing_grid` in `video.py` re-fits the grid (`grid_rects`) and `material_card` draws the sample ball and the two dials, read from the file. |
| Charizard’s material samples | `materialSamples` in `video.cjs` renders a whole sphere for each material using its shades, finish, body, and the glow on its pixels. `material_list` in `video_numbers.py` draws the one list both material scenes share: flat samples in the map colors, then the spheres. `draw_surfaces` puts each number back on the lit pose. |
| Jellicent's shared sheet assembling into two forms | `video_looks.cjs` reads the actual front-view sheet, piece swaps and idle animation. `render` in `video.py` places the labels. |
| The side-by-side, the "flat color + facing = shading" layout, the place and palette grids | `render` in `video.py`. A scene asks with `s2`, `sum` or `grid`. |
| Selectable English captions | The track uses each scene’s `cap` or falls back to its `note`. `write_subtitles` in `video.py` times it to the captured frames, then ffmpeg adds it as an optional track. |
| How one scene becomes the next | See the next section. `compose` in `video.py` draws the wipe; `render` slides the side panels; `opacity` fades the labels. |
| When Charizard holds still | `settle` and `depart` in `video.cjs` slow the animation to rest on the pose the numbers are pinned to (`HOLD`) and start it again. Scenes that wipe into each other while the sprite moves count down to the same moment (`settle(u, d + …)`), so both sides of the wipe show the same pose. `blinks` sets when the eyes close. |
| Size and compression | `S` (pixels per sprite pixel) and `CRF` in `video.py`. |
| Anything the page itself draws | `page/app.js` and `page/shaders.js`, then rebuild the page. |

## How one scene becomes the next

Two scenes are joined in one of two ways, and the choice says how they relate.

- **A wipe** (the default) is for a new picture. A line sweeps the new scene in over the old one, which keeps
  running underneath. Where both scenes hold the same pose in the same room, the sprite does not move and only
  the layer under the line changes: lit to flat color, flat color to facing, facing to lit, lit to the material
  map and back. To set one up, the scene before must end on the full frame in the same pose, with the light
  where the next scene has it. The pose can be the held one, or a moving one that both scenes count down from.
- **`join: 'flow'`** is for one continuous shot. The scene starts on the exact picture the last one ended on
  and something then moves: the pieces leave for their sheet and come back, the "Glow off" half pushes in. Only the labels change, so the scene before clears its own with `show: [null, 0]` and the
  new one fades its in with `show: [from, null]`.

A side panel never appears or vanishes by itself: `open` (0 to 1) slides it. The title's text slides away to
leave the sprite alone in the frame, the two stored sheets beside the light do the same, and the end card's
text slides back in.

Inside a scene, the order is: arrive, a moment to read, the action, a short hold. Nothing should move while a
wipe is still crossing, and the numbers wait (`from` in `ov`) until their key has been read.

## What a scene can ask the page for

`sprite`, `place` (studio, cave, rain, field, snow, deep), `view`, and:

| Switch | Does |
| --- | --- |
| `mode` | 0 lit, 1 colors only, 2 facing, 3 materials, 4 glow, 5 each part in its own color |
| `pure` | start from each material's base shade instead of the painted color; with `mode: 1` this is flat color |
| `plain` | treat every material as matte and solid |
| `noGlow`, `glowLights` | turn glow off; stop glow lighting the room |
| `sheet` | 0 to 1: move the pieces from the pose to their places on a sheet |
| `explode` | 0 to 1: pull the parts apart |
| `behind` | put the light behind the sprite |
| `palBase`, `shiny`, `looks`, `blink` | which palette and looks are on |

## Rules this video has settled on

- Build one sprite up a layer at a time, and show nothing before the layer that explains it. In order: flat color and the palette; the pieces and the animation, still in flat color; facing, then the light that uses it, with every material matte and no glow; materials; glow. Then what else one file holds: the same sprite in six places, variants, views. End on the finished Charizard.
- Show each thing once. If two scenes make the same point, keep the one that makes it on the sprite being built, or fold one into the other (Charizard’s wing membrane is one cell of the materials grid, not a scene of its own).
- Keep the picture full-frame. Put one short explanation beside the feature it describes. Use a shared heading for a grid and keep the longer explanation in the optional caption track.
- Open with SPU as a file format for a complete 2D sprite. The title lists what one file holds as short items, not a sentence, in the order the video shows them: palettes, pieces, animations, surface facing, materials, variants, views.
- Say outright, on the first flat picture, that only flat color is stored and shading comes later from the light. The palette key follows once that has been read.
- Before the sprite comes apart, let it move for a moment under the label that says it is built from pieces.
- Say what a thing is before showing many of it: a palette is a list of colors and swapping it recolors the same pixels; a variant swaps pieces.
- Follow the material map with the same lit pose and show the numbered spheres together. Then show the materials of other sprites together, metal first, in one grid that grows a cell at a time: one cell fills the frame, the next splits it, and from the third on the picture zooms out to fit them. A cell never changes rows or passes another as the grid grows, each newcomer gets the same second and a half with a frame around it, and the full grid is not held long. Each cell has a sample ball of its material and its two dials, fuzzy to polished and metal to clear. The last cell is Charizard’s wing membrane, lit from behind in the cave, where the glow scenes then stay.
- Say "pieces" and "flat color". Not "drawings".
- The light stays still unless the scene is about light.
- Nothing cuts abruptly. A new picture wipes in; a continuous change plays as one shot (see above).
- A wipe between two views of the same sprite keeps the sprite in place, so only what the scene is about changes.
- Labels fade; side panels slide. Nothing pops on or off.
- Panels are compact, with no empty space.
- Each sprite pixel is an exact block of video pixels, so pixels stay crisp.
- Keep minor details out of the captions.
- The end card reads “SPU” and “Universal Sprite Format.”
