//! Runs the conformance suite in `conformance/` against this implementation,
//! then checks a few of its pictures from first principles so the expected
//! images are not merely whatever the reference renderer happened to draw.
use serde_json::Value;
use spu::{
    binary,
    picture::{Frame, picture_named},
    sprite::{Extra, Sprite},
};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("conformance")
}
fn manifest() -> Value {
    serde_json::from_slice(&std::fs::read(root().join("manifest.json")).unwrap()).unwrap()
}
fn open(name: &str) -> Sprite {
    binary::read(&std::fs::read(root().join(format!("valid/{name}.spu"))).unwrap()).unwrap()
}
fn draw(name: &str, animation: &str, tick: f64, variants: &[&str]) -> Frame {
    picture_named(&open(name), "front", animation, tick, variants).unwrap()
}

const MARK: [&str; 6] = ["1111.", "1....", "122..", "1....", "1..3.", "1..44"];
const NORMAL: [[u8; 4]; 5] = [[0; 4], [0xd0, 0x20, 0x20, 255], [0x20, 0xa0, 0x40, 255], [0x20, 0x40, 0xd0, 255], [0xf0, 0xc0, 0x20, 0x80]];
const SHINY: [[u8; 4]; 5] = [[0; 4], [0x20, 0x80, 0xd0, 255], [0xd0, 0xa0, 0x20, 255], [0xa0, 0x20, 0xc0, 255], [255, 255, 255, 0x80]];
const GHOST: [[u8; 4]; 5] = [[0; 4], [0x20, 0x80, 0xd0, 255], [0; 4], [0xa0, 0x20, 0xc0, 0x40], [255, 255, 255, 0x80]];

/// A 24×24 canvas centered on the origin with the mark's texel `(u, v)`
/// at sprite pixel `place(u, v)`.
fn expected(colors: &[[u8; 4]; 5], place: impl Fn(i32, i32) -> (i32, i32)) -> Frame {
    let mut pixels = vec![0; 24 * 24 * 4];
    for (v, row) in MARK.iter().enumerate() {
        for (u, c) in row.bytes().enumerate() {
            if c != b'.' {
                let (x, y) = place(u as i32, v as i32);
                let i = (((y + 12) * 24 + x + 12) * 4) as usize;
                pixels[i..i + 4].copy_from_slice(&colors[(c - b'0') as usize]);
            }
        }
    }
    Frame { width: 24, height: 24, pixels }
}

#[test]
fn every_valid_file_draws_its_expected_pictures() {
    let mut frames = 0;
    for case in manifest()["valid"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let sprite = binary::read(&std::fs::read(root().join(file)).unwrap()).unwrap_or_else(|e| panic!("{file}: {e:#}"));
        for frame in case["frames"].as_array().unwrap() {
            let variants: Vec<&str> = frame["variants"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
            let got = picture_named(&sprite, frame["view"].as_str().unwrap(), frame["animation"].as_str().unwrap(), frame["tick"].as_f64().unwrap(), &variants).unwrap();
            let path = frame["expected"].as_str().unwrap();
            let want = Frame::from_png(&std::fs::read(root().join(path)).unwrap()).unwrap();
            assert!(got == want, "{file} does not match {path}");
            frames += 1;
        }
    }
    assert!(frames >= 70);
}

#[test]
fn every_invalid_file_is_refused() {
    let cases = manifest();
    let cases = cases["invalid"].as_array().unwrap();
    assert!(cases.len() >= 35);
    for case in cases {
        let file = case["file"].as_str().unwrap();
        let bytes = std::fs::read(root().join(file)).unwrap();
        assert!(binary::read(&bytes).is_err(), "{file} must be refused: {}", case["about"]);
    }
}

#[test]
fn a_still_drawing_is_itself_on_the_canvas() {
    let still = expected(&NORMAL, |u, v| (u, v));
    assert!(draw("still", "main", 0.0, &[]) == still);
    assert!(draw("unknown-fields", "main", 0.0, &[]) == still);
    assert!(draw("materials-do-not-change-the-reference", "main", 0.0, &[]) == still);
}

#[test]
fn palettes_come_from_the_file_and_variants_swap_them() {
    assert!(draw("palettes-and-variants", "main", 0.0, &[]) == expected(&NORMAL, |u, v| (u, v)));
    assert!(draw("palettes-and-variants", "main", 0.0, &["shiny"]) == expected(&SHINY, |u, v| (u, v)));
    // The last variant naming a palette wins, whatever else is active.
    assert!(draw("palettes-and-variants", "main", 0.0, &["shiny", "hat"]) == draw("palettes-and-variants", "main", 0.0, &["hat", "shiny"]));
    // A palette color's own opacity multiplies the pixel's.
    assert!(draw("palette-opacity", "main", 0.0, &["ghost"]) == expected(&GHOST, |u, v| (u, v)));
}

#[test]
fn quarter_turns_and_mirrors_move_whole_pixels() {
    let turn = |tick| draw("rotation-and-scale", "main", tick, &[]);
    assert!(turn(0.0) == expected(&NORMAL, |u, v| (u, v)));
    // Clockwise on screen, +Y down: (u, v) lands at (-v - 1, u).
    assert!(turn(1.0) == expected(&NORMAL, |u, v| (-v - 1, u)));
    assert!(turn(2.0) == expected(&NORMAL, |u, v| (-u - 1, -v - 1)));
    assert!(turn(3.0) == expected(&NORMAL, |u, v| (v, -u - 1)));
    assert!(turn(6.0) == turn(3.0));
    assert!(turn(8.0) == expected(&NORMAL, |u, v| (-u - 1, v)));
    // Scaled to nothing, a part draws nothing.
    assert!(turn(12.0).pixels.iter().all(|&b| b == 0));
}

#[test]
fn loops_wrap_and_other_animations_hold() {
    let at = |animation, tick| draw("looping-and-holding", animation, tick, &[]);
    assert!(at("main", 0.0) == expected(&NORMAL, |u, v| (u - 8, v)), "first key holds before its tick");
    assert!(at("main", 8.0) == at("main", 0.0));
    assert!(at("main", 21.0) == expected(&NORMAL, |u, v| (u - 5, v)), "tick 21 wraps to 5");
    assert!(at("once", 8.0) == at("once", 7.0) && at("once", 500.0) == at("once", 7.0));
    assert!(at("once", 7.0) == expected(&NORMAL, |u, v| (u - 3, v)), "moving toward a key past the end");
}

#[test]
fn extra_parts_show_only_when_a_look_shows_them() {
    let s = open("extra-parts");
    let draw = |looks: &[&str]| picture_named(&s, "front", "main", 0.0, looks).unwrap();
    assert!(draw(&[]) == expected(&NORMAL, |u, v| (u, v)), "an extra part is hidden by default");
    assert!(draw(&["with-hat"]) != draw(&[]));
    assert!(draw(&["with-hat", "no-hat"]) == draw(&[]) && draw(&["no-hat", "with-hat"]) == draw(&["with-hat"]), "the later look wins");
}

#[test]
fn each_view_keeps_its_own_length() {
    let s = open("two-views");
    let back = |tick| picture_named(&s, "back", "main", tick, &[]).unwrap();
    assert!(back(1.0) == expected(&NORMAL, |u, v| (u + 2, v)));
    assert!(back(4.0) == back(1.0), "the back view loops every 3 ticks");
    assert!(picture_named(&s, "front", "main", 2.0, &[]).unwrap() == expected(&NORMAL, |u, v| (u, v)), "the front view loops every tick");
    assert!(picture_named(&s, "back", "front-only", 0.0, &[]).unwrap().pixels.iter().all(|&b| b == 0));
}

#[test]
fn what_a_reader_does_not_understand_survives_saving() {
    let bytes = std::fs::read(root().join("valid/unknown-fields.spu")).unwrap();
    let sprite = binary::read(&bytes).unwrap();
    assert_eq!(sprite.unknown, vec![(*b"note", b"A chunk this reader does not know.".to_vec())]);
    assert_eq!(sprite.extras, vec![Extra { path: "notes/readme.txt".into(), bytes: b"Extras are kept.".to_vec() }]);
    assert_eq!(binary::write(&sprite).unwrap(), bytes, "reading and writing a file gives the same bytes");
}
