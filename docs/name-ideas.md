# Name ideas

**Working decision, 2026-11-09: SPU — Sprite Universal (`.spu`), version 1.**
It continues the name the existing files, repositories and editor already
use, and version 0 files upgrade automatically. The owner has not yet
confirmed it; the name lives in two constants in `src/format.rs`.

Brainstorm, 2026-11-08. No name or extension was selected at the time. These describe
editable sprites, parts, animation and materials, without a particular game or
art style. Extensions below are proposals, not registered or reserved names.

## Three favorites

1. **SPU — Sprite Universal** (`.spu`). The clearest continuation of the working
   name. Short, literal and broad enough for characters, objects and effects.
   Its drawback is searchability: [SecretFlow already calls its computation
   system SPU](https://github.com/secretflow/spu). A future incompatible format
   would also need an explicit version and migration path from Smeargle's SPU v0.
2. **Celpack** (`.clpk`). Immediately suggests an editable package of drawings.
   Friendly to artists and useful whether the package holds drawn frames or
   articulated parts. It emphasizes packaging more than lighting or motion.
   [Celpack is already a physical-packaging business name](https://celulosa.com.ar/quienes-somos-2/).
3. **Articel** (`.acel`). “Articulation” + “cel”: captures moving, editable
   drawings without referring to Pokémon. Distinctive, but people may spell it
   “article”; [an existing GitHub repository uses the exact name](https://github.com/Neelima-Rapeti/Articel).

## Other directions

| Name | Possible extension | Appeal / reservation |
| --- | --- | --- |
| Kinocel | `.kcel` | Motion + drawing; compact, though pronunciation needs teaching. Not screened beyond a broad exact-name search. |
| Sprocket | `.spkt` | Articulated movement and animation machinery. An existing [video editor uses Sprocket](https://sprocketvideo.org/docs/get-started/getting-started/), so media-tool confusion is likely. |
| Celweave | `.cwev` | Layers and motion woven together. Already used by an [animation workflow product](https://celweave.com/); deprioritize. |
| Sprig | `.sprg` | Small, lively, easy to say. Already a [game-making platform](https://github.com/hackclub/sprig); deprioritize. |
| Skein | `.skein` | Connected strands suggest parts and animation tracks. Already a [hash-function family](https://www.schneier.com/academic/skein/); weak searchability. |
| Facet | `.fct` | Surface properties, multiple views and a whole assembled from pieces. Does not communicate animation; collision screening remains open. |
| Ply | `.ply` | Layers, compactness, and simple pronunciation. Already a [graphics file format](https://paulbourke.net/dataformats/ply/index.html); reject the exact name/extension combination. |

The first three received targeted exact-name and primary-source checks. These
are limited discoverability checks, not an availability finding or legal
clearance. The editor and reference renderer can have their own names; choosing
a format name does not require renaming Smeargle.
