# SPU conformance suite

Small files that pin down the [SPU format](../docs/spec.md). An implementation
conforms when it:

1. **opens every file in `valid/`** and, for each frame listed in
   `manifest.json`, draws a picture identical to the PNG in `expected/`
   (same size, every pixel's RGBA equal);
2. **refuses every file in `invalid/`**;
3. **keeps what it does not understand**: after opening and saving
   `valid/unknown-fields.spu`, the chunk it does not know and the extras are
   still there, and the saved file is byte for byte the one it opened.

`manifest.json` lists each file with a sentence saying what it demonstrates
or why it is invalid. Each frame names a view, an animation, a tick and the
active looks, in order.

```sh
cargo test --test conformance      # this repository's implementation
cargo run --bin make-conformance   # regenerate the suite (output is reproducible)
```

The art is a made-up 5×6 mark; nothing here comes from a game. The expected
pictures are drawn by the reference library, and several are also checked
from first principles in `tests/conformance.rs`.

Most cases are written the way older files were (a ZIP of JSON and PNGs) and
converted. Generating the suite therefore also checks the converter: each
case must draw exactly what the older reference renderer drew.
