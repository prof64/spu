# The tour video

`docs/spu-tour.mp4` is made from the demo page, in two steps.

```sh
cd lab
python3 build.py && python3 standalone.py      # only if anything under page/ changed
node video.cjs                                  # capture every frame from the page: about 35 seconds
uv run --no-project --with pillow --with numpy python video.py 692 1780   # lay out, caption, encode; the numbers save those frames as out/video/still-N.png
open ../docs/spu-tour.mp4
```

`video.cjs` needs playwright-core and a Chromium: set `PLAYWRIGHT_CORE` and
`CHROME_PATH`. `video.py` needs ffmpeg. The page needs `out/sprites.json`,
which `export.py` makes (see [README.md](README.md)).

## Where things are

| To change | Edit |
| --- | --- |
| The order of scenes, how long each lasts, its optional caption and chapter | `SCENES` in `video.cjs`. One entry per scene. |
| What a scene shows | The scene's `s`: the state handed to the page (sprite, place, and the switches below). |
| Where the light is | `STILL` in `video.cjs` (per place), a scene's own `lamp`, or `lamp: SWING` for the one scene where it moves. |
| Short labels beside the sprites | `overlay` in `video.py`. A scene asks for one with `ov`. |
| The numbers moving from a key to the sprite | `video_numbers.py` sets the paths and timing. `info` in `video.cjs` reads the displayed palette colors and the material IDs for the held pose. |
| Jellicent's shared sheet assembling into two forms | `video_looks.cjs` reads the actual front-view sheet, piece swaps and idle animation. `picture` in `video.py` places the labels. |
| The side-by-side, the "flat color + facing = shading" layout, the place and palette grids | `picture` in `video.py`. A scene asks with `s2`, `sum` or `grid`. |
| Selectable English captions | The `cap` on each scene supplies the text. `write_subtitles` in `video.py` times it to the captured frames, then ffmpeg adds it as an optional track. |
| The wipe between scenes | `compose` in `video.py`; `cut: true` on a scene skips it. |
| Size and compression | `S` (pixels per sprite pixel) and `CRF` in `video.py`. |
| Anything the page itself draws | `page/app.js` and `page/shaders.js`, then rebuild the page. |

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

- Show what the file stores first (flat color, the sheet of pieces, the facing
  sheet), then how it becomes a picture, then materials, glow, places, looks.
- Keep the picture full-frame. Place short labels beside the subject or inside comparison cells. Put longer explanations in the optional caption track.
- Say "pieces" and "flat color". Not "drawings".
- The light stays still unless the scene is about light.
- Every scene change wipes; nothing cuts abruptly.
- Panels are compact, with no empty space.
- Each sprite pixel is an exact block of video pixels, so pixels stay crisp.
- Keep minor details out of the captions.
- The end card reads “SPU” and “Universal Sprite Format.”
