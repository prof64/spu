# Dungeon scout

An original red-panda explorer with a blue scarf, backpack, ringed tail and a
hand-held amber lantern. This replaces the visual role of the earlier small fox
without overwriting that older benchmark fixture.

- Package: `web/examples/dungeon-scout.sprite`, thumbnail `dungeon-scout.png`.
- Eight views: South, SouthEast, East, NorthEast, North, NorthWest, West, SouthWest.
  These are distinct generated poses, not a front image rotated through eight angles.
- Canvas 72×72; approximately 56 pixels tall; ground anchor `[36,64]`.
- Idle: 100 ticks. Walk: 28 ticks, four held poses. Clock: 60 ticks/second.
- Four separately editable layers per direction: body, two feet, hand-held lamp.
  Walk alternates the two existing foot drawings, with one-pixel body bob. This
  modest four-pose walk does not claim newly drawn full-body locomotion cels.
- Fur/cloth/leather are rough; the lantern has an amber window emission mask.
  Its emission animates independently. Normal relief follows native color changes.
- 37-color material palette, binary alpha, nearest-neighbor display.

The original eight-view sheet was created with the built-in image-generation
 tool. The exact prompt, generated source, individual crops, conversion palette,
 native drawings and editable manifest are retained under `web/art-source/` with
 the `dungeon-scout` prefix. Conversion uses CIE-Lab palette mapping followed by
 majority votes on the native grid; color and alpha use separate votes. It does
 not blur or resize each view to a different body scale.

Rebuild with Pillow and NumPy:

```sh
uv run --with numpy --with pillow python tools/make_dungeon_scout.py
```

`web/examples/dungeon-scout-directions.png` shows the delivered views;
`dungeon-scout-walk.png` and `dungeon-scout-walk.gif` show the actual four held
walk poses, without invented preview interpolation. The package was loaded by
the native Rust parser, atlas builder and GPU renderer; all views and matching
maps were accepted. Front material/color/normal renders are retained in
`web/art-source/dungeon-scout-renders/`. Review confirmed the eight orientations,
consistent equipment and face design, lamp attachment, and native-scale leg motion.
