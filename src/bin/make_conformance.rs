//! Writes the conformance suite under `conformance/`: small files every
//! implementation must accept, the exact picture each must draw, and files
//! every implementation must refuse. Run from the repository root:
//! `cargo run --bin make-conformance`.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sprite_lab::{format::Package, reference::Reference};
use std::{
    io::{Cursor, Write},
    path::Path,
};

type Files = Vec<(String, Vec<u8>)>;

/// An asymmetric 5×6 mark, so turns and mirrors are unmistakable.
const MARK: [&str; 6] = ["1111.", "1....", "122..", "1....", "1..3.", "1..44"];
const NORMAL: [&str; 5] = ["#00000000", "#d02020", "#20a040", "#2040d0", "#f0c02080"];
const SHINY: [&str; 5] = ["#00000000", "#2080d0", "#d0a020", "#a020c0", "#ffffff80"];

fn indices(rows: &[&str]) -> (u32, u32, Vec<u8>) {
    let pixels = rows
        .iter()
        .flat_map(|r| r.bytes().map(|b| if b == b'.' { 0 } else { b - b'0' }))
        .collect();
    (rows[0].len() as u32, rows.len() as u32, pixels)
}
fn color(text: &str) -> [u8; 4] {
    sprite_lab::format::parse_color(text).unwrap()
}
fn indexed_png(rows: &[&str], palette: &[&str]) -> Vec<u8> {
    let (w, h, pixels) = indices(rows);
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(palette.iter().flat_map(|c| color(c)[..3].to_vec()).collect::<Vec<_>>());
    encoder.set_trns(palette.iter().map(|c| color(c)[3]).collect::<Vec<_>>());
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    drop(writer);
    out
}
fn rgba_png(rows: &[&str], palette: &[&str]) -> Vec<u8> {
    let (w, h, pixels) = indices(rows);
    let rgba: Vec<u8> = pixels.iter().flat_map(|&i| color(palette[i as usize])).collect();
    let mut out = Vec::new();
    image::write_buffer_with_format(&mut Cursor::new(&mut out), &rgba, w, h, image::ColorType::Rgba8, image::ImageOutputFormat::Png).unwrap();
    out
}
fn flat_png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    image::write_buffer_with_format(&mut Cursor::new(&mut out), &rgba.repeat((w * h) as usize), w, h, image::ColorType::Rgba8, image::ImageOutputFormat::Png).unwrap();
    out
}

/// A ZIP with fixed dates, so the suite is byte-for-byte reproducible.
fn zip(manifest: Option<&Value>, files: &Files) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    if let Some(manifest) = manifest {
        archive.start_file("sprite.json", options).unwrap();
        archive.write_all(&serde_json::to_vec_pretty(manifest).unwrap()).unwrap();
    }
    for (name, bytes) in files {
        archive.start_file(name.as_str(), options).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

fn key(tick: u32, value: impl Into<Value>) -> Value {
    json!({"tick": tick, "value": value.into()})
}
fn eased(tick: u32, value: f64, ease: &str) -> Value {
    json!({"tick": tick, "value": value, "ease": ease})
}
fn mark_files() -> Files {
    vec![("mark.png".into(), rgba_png(&MARK, &NORMAL))]
}
/// One view, a 24×24 canvas centered on the origin, one drawing `mark`.
fn document(name: &str, parts: Value, animations: Value) -> Value {
    json!({
        "format": "spu", "version": 1, "name": name, "ticksPerSecond": 60,
        "views": [{
            "id": "front", "name": "Front",
            "canvas": {"x": -12, "y": -12, "width": 24, "height": 24},
            "images": [{"id": "mark", "name": "Mark", "x": 0, "y": 0, "color": "mark.png"}],
            "parts": parts, "animations": animations,
        }],
    })
}
fn one_part(tracks: Value, length: u32, looping: bool) -> Value {
    document(
        "case",
        json!([{"id": "body", "name": "Body"}]),
        json!([{"id": "main", "name": "Main", "length": length, "looping": looping, "tracks": [tracks]}]),
    )
}
fn still() -> Value {
    one_part(json!({"part": "body", "image": [key(0, "mark")]}), 1, true)
}

struct Valid {
    name: &'static str,
    about: &'static str,
    manifest: Value,
    files: Files,
    /// (animation, tick, variants)
    frames: Vec<(&'static str, f64, Vec<&'static str>)>,
}

fn valid() -> Vec<Valid> {
    let ticks = |list: &[u32]| list.iter().map(|&t| ("main", f64::from(t), vec![])).collect::<Vec<_>>();
    let mut cases = Vec::new();
    cases.push(Valid {
        name: "still",
        about: "One full-color drawing at the origin. The picture is the drawing itself, placed on the canvas.",
        manifest: still(),
        files: mark_files(),
        frames: ticks(&[0, 5]),
    });

    let mut m = still();
    m["palettes"] = json!([
        {"id": "normal", "name": "Normal", "colors": NORMAL},
        {"id": "shiny", "name": "Shiny", "colors": SHINY},
    ]);
    m["variants"] = json!([
        {"id": "shiny", "name": "Shiny", "palette": "shiny"},
        {"id": "hat", "name": "Hat", "replacements": [{"view": "front", "image": "mark", "with": "hat"}]},
    ]);
    m["views"][0]["images"].as_array_mut().unwrap().push(json!({"id": "hat", "name": "Hat", "x": -2, "y": -3, "color": "hat.png"}));
    cases.push(Valid {
        name: "palettes-and-variants",
        about: "Indexed drawings take their colors from the file's palettes, not from the PNG. Index 0 is transparent; index 4 is half transparent. One variant swaps the palette, another swaps the drawing, and both can be active together.",
        manifest: m,
        // The PNG's own colors are deliberately wrong: they must be ignored.
        files: vec![
            ("mark.png".into(), indexed_png(&MARK, &["#ffffff", "#000000", "#000000", "#000000", "#000000"])),
            ("hat.png".into(), indexed_png(&["..33..", ".3333.", "222222"], &NORMAL)),
        ],
        frames: vec![("main", 0.0, vec![]), ("main", 0.0, vec!["shiny"]), ("main", 0.0, vec!["hat"]), ("main", 0.0, vec!["shiny", "hat"]), ("main", 0.0, vec!["hat", "shiny"])],
    });

    cases.push(Valid {
        name: "easing",
        about: "Each key moves toward the next along its own curve: step, linear, easeIn, easeOut, easeInOut. Positions between pixels are sampled, not rounded.",
        manifest: one_part(json!({"part": "body", "image": [key(0, "mark")],
            "x": [eased(0, -10.0, "step"), eased(10, -10.0, "linear"), eased(20, 0.0, "easeIn"), eased(30, 5.0, "easeOut"), eased(40, -5.0, "easeInOut"), eased(50, 3.0, "step")],
            "y": [eased(0, -3.0, "linear"), eased(50, 2.5, "step")]}), 60, true),
        files: mark_files(),
        frames: ticks(&[0, 9, 10, 13, 15, 20, 23, 27, 30, 33, 37, 40, 43, 45, 47, 50, 59]),
    });

    cases.push(Valid {
        name: "rotation-and-scale",
        about: "A part turns clockwise about its origin and scales per axis; a negative scale mirrors. Pixels come from nearest-neighbor sampling at pixel centers.",
        manifest: one_part(json!({"part": "body", "image": [key(0, "mark")],
            "rotation": [key(0, 0.0), key(1, 90.0), key(2, 180.0), key(3, 270.0), key(4, 45.0), key(5, 33.0), key(6, -90.0), key(7, 0.0), key(11, 30.0)],
            "scaleX": [key(0, 1.0), key(7, 2.0), key(8, -1.0), key(9, 0.5), key(10, 1.5), key(11, -2.0), key(12, 0.0)],
            "scaleY": [key(0, 1.0), key(7, 2.0), key(8, 1.0), key(9, 3.0), key(10, -1.5), key(11, 1.25)]}), 13, false),
        files: mark_files(),
        frames: ticks(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
    });

    let joined = document(
        "case",
        json!([{"id": "arm", "name": "Arm", "parent": "body"}, {"id": "body", "name": "Body"}, {"id": "hand", "name": "Hand", "parent": "arm"}]),
        json!([{"id": "main", "name": "Main", "length": 6, "looping": true, "tracks": [
            {"part": "body", "image": [key(0, "mark")], "x": [key(0, -4.0)], "y": [key(0, -6.0)],
             "rotation": [key(0, 0.0), key(1, 90.0), key(2, 20.0), key(3, 0.0)],
             "scaleX": [key(0, 1.0), key(3, 2.0), key(4, -1.0), key(5, -2.0)], "scaleY": [key(0, 1.0), key(5, 0.5)]},
            {"part": "arm", "image": [key(0, "mark")], "x": [key(0, 5.0)], "y": [key(0, 1.0)], "rotation": [key(0, 0.0), key(2, 45.0)]},
            {"part": "hand", "image": [key(0, "mark")], "x": [key(0, 2.0)], "y": [key(0, 6.0)], "rotation": [key(0, 0.0), key(4, 30.0)], "scaleX": [key(0, 1.0), key(1, 0.5)]},
        ]}]),
    );
    cases.push(Valid {
        name: "joined-parts",
        about: "A child follows its parent: its position goes through the parent's whole placement, its rotation adds (reversed under a mirrored parent), its scale multiplies. A child never skews, even under a parent squashed on one axis. A parent may be listed after its child.",
        manifest: joined,
        files: mark_files(),
        frames: ticks(&[0, 1, 2, 3, 4, 5]),
    });

    let order = document(
        "case",
        json!([{"id": "a", "name": "A"}, {"id": "b", "name": "B"}, {"id": "c", "name": "C", "parent": "a"}]),
        json!([{"id": "main", "name": "Main", "length": 6, "looping": true, "tracks": [
            {"part": "a", "image": [key(0, "mark"), key(4, Value::Null), key(5, "mark")], "visible": [key(0, true), key(2, false), key(3, true)], "z": [key(0, 0.0), key(1, 2.0)]},
            {"part": "b", "image": [key(0, "block")], "x": [key(0, -1.0)], "y": [key(0, 1.0)], "z": [key(0, 0.0), key(5, 2.0)]},
            {"part": "c", "image": [key(0, "block")], "x": [key(0, 2.0)], "y": [key(0, 3.0)], "z": [key(0, -1.0), key(3, 1.0)]},
        ]}]),
    );
    let mut order = order;
    order["views"][0]["images"].as_array_mut().unwrap().push(json!({"id": "block", "name": "Block", "x": 0, "y": 0, "color": "block.png"}));
    cases.push(Valid {
        name: "draw-order-and-visibility",
        about: "Parts draw from lowest z to highest; equal z draws in the order parts are listed. A hidden part or a part with no drawing draws nothing, but its children still draw: a parent passes on placement only.",
        manifest: order,
        files: vec![mark_files().remove(0), ("block.png".into(), rgba_png(&["222", "232", "222"], &NORMAL))],
        frames: ticks(&[0, 1, 2, 3, 4, 5]),
    });

    let fade = document(
        "case",
        json!([{"id": "back", "name": "Back"}, {"id": "front", "name": "Front", "parent": "back"}]),
        json!([{"id": "main", "name": "Main", "length": 4, "looping": true, "tracks": [
            {"part": "back", "image": [key(0, "mark")], "opacity": [key(0, 1.0), key(2, 0.25)]},
            {"part": "front", "image": [key(0, "mark")], "x": [key(0, 2.0)], "y": [key(0, 1.0)], "opacity": [eased(0, 0.5, "linear"), eased(3, 0.0, "step")]},
        ]}]),
    );
    cases.push(Valid {
        name: "opacity",
        about: "A part's opacity multiplies its drawing's own transparency and blends over what is behind. A parent's opacity does not fade its children.",
        manifest: fade,
        files: mark_files(),
        frames: ticks(&[0, 1, 2, 3]),
    });

    let track = json!({"part": "body", "image": [key(0, "mark")], "x": [eased(2, -8.0, "linear"), eased(12, 2.0, "step")]});
    let mut timing = one_part(track.clone(), 8, true);
    timing["views"][0]["animations"].as_array_mut().unwrap().push(json!({"id": "once", "name": "Once", "length": 8, "looping": false, "tracks": [track]}));
    cases.push(Valid {
        name: "looping-and-holding",
        about: "Before a property's first key, the first key's value holds. A looping animation wraps at its length; one that does not loop holds its last tick. A key past the end is never reached but still shapes the motion toward it.",
        manifest: timing,
        files: mark_files(),
        frames: vec![("main", 0.0, vec![]), ("main", 2.0, vec![]), ("main", 7.0, vec![]), ("main", 8.0, vec![]), ("main", 21.0, vec![]), ("once", 7.0, vec![]), ("once", 8.0, vec![]), ("once", 500.0, vec![])],
    });

    let mut over = still();
    over["views"][0]["images"] = json!([
        {"id": "mark", "name": "Mark", "x": 0, "y": 0, "color": "mark.png", "overlays": ["open"]},
        {"id": "open", "name": "Eyes open", "x": 1, "y": 1, "color": "open.png"},
        {"id": "closed", "name": "Eyes closed", "x": 1, "y": 2, "color": "closed.png"},
    ]);
    over["views"][0]["animations"][0]["tracks"][0]["rotation"] = json!([key(0, 0.0), key(1, 90.0)]);
    over["views"][0]["animations"][0]["length"] = 2.into();
    over["variants"] = json!([{"id": "blink", "name": "Blink", "replacements": [{"view": "front", "image": "open", "with": "closed"}]}]);
    cases.push(Valid {
        name: "overlays",
        about: "A drawing can carry overlays: other drawings painted straight over it, moving with the same part. A variant can replace an overlay, which is how a blink is stored.",
        manifest: over,
        files: vec![mark_files().remove(0), ("open.png".into(), rgba_png(&["3.3"], &NORMAL)), ("closed.png".into(), rgba_png(&["222"], &NORMAL))],
        frames: vec![("main", 0.0, vec![]), ("main", 0.0, vec!["blink"]), ("main", 1.0, vec!["blink"])],
    });

    let mut extra = still();
    extra["futureTool"] = json!({"anything": [1, 2, 3]});
    extra["required"] = json!([]);
    extra["views"][0]["parts"][0]["physics"] = json!({"mass": 2});
    extra["views"][0]["animations"][0]["tracks"][0]["wobble"] = json!([key(0, 1.0)]);
    let mut files = mark_files();
    files.push(("notes/readme.txt".into(), b"Unknown members are kept.".to_vec()));
    cases.push(Valid {
        name: "unknown-fields",
        about: "Fields and archive members a reader does not know are ignored when drawing and kept when saving. The picture equals the 'still' case.",
        manifest: extra,
        files,
        frames: ticks(&[0]),
    });

    let mut clipped = one_part(json!({"part": "body", "image": [key(0, "mark")], "x": [key(0, 3.0)], "y": [key(0, -2.0)], "scaleX": [key(0, 2.0)], "scaleY": [key(0, 2.0)]}), 1, true);
    clipped["views"][0]["canvas"] = json!({"x": 5, "y": 0, "width": 7, "height": 9});
    cases.push(Valid {
        name: "canvas",
        about: "The canvas is a window onto sprite space: its x and y say which point is the top-left pixel, and anything outside it is cut off.",
        manifest: clipped,
        files: mark_files(),
        frames: ticks(&[0]),
    });

    let mut lit = still();
    lit["materials"] = json!([{"id": "glass", "name": "Glass", "roughness": 0.1, "transmission": 0.8, "emission": 3.0, "opacity": 0.3}]);
    lit["views"][0]["images"][0]["material"] = "glass".into();
    lit["views"][0]["images"][0]["normal"] = "mark-normal.png".into();
    lit["views"][0]["images"][0]["surface"] = "mark-surface.png".into();
    lit["views"][0]["images"][0]["emission"] = "mark-emission.png".into();
    lit["views"][0]["animations"][0]["tracks"][0]["emission"] = json!([key(0, 4.0)]);
    let mut files = mark_files();
    files.push(("mark-normal.png".into(), flat_png(5, 6, [128, 128, 255, 255])));
    files.push(("mark-surface.png".into(), flat_png(5, 6, [40, 0, 200, 255])));
    files.push(("mark-emission.png".into(), flat_png(5, 6, [255, 120, 0, 255])));
    cases.push(Valid {
        name: "materials-do-not-change-the-reference",
        about: "Materials and their maps describe how light treats a drawing. The reference picture has no light, so it equals the 'still' case.",
        manifest: lit,
        files,
        frames: ticks(&[0]),
    });
    cases
}

fn invalid() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let with = |edit: &dyn Fn(&mut Value)| {
        let mut m = still();
        edit(&mut m);
        zip(Some(&m), &mark_files())
    };
    let two_parts = |a: Value, b: Value| {
        let mut m = still();
        m["views"][0]["parts"] = json!([{"id": "body", "name": "Body", "parent": a}, {"id": "tail", "name": "Tail", "parent": b}]);
        zip(Some(&m), &mark_files())
    };
    vec![
        ("not-a-zip", "The file is not a ZIP archive.", b"{\"format\":\"spu\"}".to_vec()),
        ("no-manifest", "The archive has no sprite.json.", zip(None, &mark_files())),
        ("manifest-not-json", "sprite.json is not JSON.", zip(None, &vec![("sprite.json".into(), b"format: spu".to_vec())])),
        ("other-format", "The format name is not 'spu'.", with(&|m| m["format"] = "sprite".into())),
        ("future-version", "The version is newer than this specification.", with(&|m| m["version"] = 2.into())),
        ("unknown-required-capability", "The file requires a capability the reader does not have.", with(&|m| m["required"] = json!(["volumetric-fog"]))),
        ("no-views", "There are no views.", with(&|m| m["views"] = json!([]))),
        ("no-animations", "A view has no animations.", with(&|m| m["views"][0]["animations"] = json!([]))),
        ("zero-length-animation", "An animation is zero ticks long.", with(&|m| m["views"][0]["animations"][0]["length"] = 0.into())),
        ("empty-canvas", "The canvas has no width.", with(&|m| m["views"][0]["canvas"]["width"] = 0.into())),
        ("fractional-canvas", "The canvas is not on whole pixels.", with(&|m| m["views"][0]["canvas"]["x"] = 0.5.into())),
        ("duplicate-part-id", "Two parts share an id.", with(&|m| m["views"][0]["parts"] = json!([{"id": "body", "name": "A"}, {"id": "body", "name": "B"}]))),
        ("empty-id", "An id is empty.", with(&|m| m["views"][0]["id"] = "".into())),
        ("unknown-parent", "A part names a parent that does not exist.", two_parts(Value::Null, "wing".into())),
        ("parent-cycle", "Two parts are each other's parent.", two_parts("tail".into(), "body".into())),
        ("unknown-track-part", "A track names a part that does not exist.", with(&|m| m["views"][0]["animations"][0]["tracks"][0]["part"] = "wing".into())),
        ("two-tracks-one-part", "An animation has two tracks for the same part.", with(&|m| {
            let t = m["views"][0]["animations"][0]["tracks"][0].clone();
            m["views"][0]["animations"][0]["tracks"].as_array_mut().unwrap().push(t);
        })),
        ("unknown-drawing-in-key", "A key shows a drawing that does not exist.", with(&|m| m["views"][0]["animations"][0]["tracks"][0]["image"] = json!([key(0, "ghost")]))),
        ("keys-out-of-order", "Keys are not in increasing tick order.", with(&|m| m["views"][0]["animations"][0]["tracks"][0]["x"] = json!([key(5, 1.0), key(2, 0.0)]))),
        ("two-keys-one-tick", "Two keys share a tick.", with(&|m| m["views"][0]["animations"][0]["tracks"][0]["x"] = json!([key(2, 1.0), key(2, 0.0)]))),
        ("negative-tick", "A key is before tick 0.", with(&|m| m["views"][0]["animations"][0]["tracks"][0]["x"] = json!([{"tick": -1, "value": 0.0}]))),
        ("unknown-ease", "A key uses an easing curve that does not exist.", with(&|m| m["views"][0]["animations"][0]["tracks"][0]["x"] = json!([eased(0, 0.0, "bounce")]))),
        ("missing-image-file", "A drawing's image is not in the archive.", with(&|m| m["views"][0]["images"][0]["color"] = "gone.png".into())),
        ("image-not-png", "A drawing's image is not a PNG.", zip(Some(&still()), &vec![("mark.png".into(), b"GIF89a".to_vec())])),
        ("map-size-mismatch", "A material map is not the size of its drawing.", {
            let mut m = still();
            m["views"][0]["images"][0]["normal"] = "n.png".into();
            let mut files = mark_files();
            files.push(("n.png".into(), flat_png(4, 4, [128, 128, 255, 255])));
            zip(Some(&m), &files)
        }),
        ("unknown-material", "A drawing names a material that does not exist.", with(&|m| m["views"][0]["images"][0]["material"] = "chrome".into())),
        ("material-factor-out-of-range", "A material's roughness is above 1.", with(&|m| m["materials"] = json!([{"id": "m", "name": "M", "roughness": 4.0}]))),
        ("bad-palette-color", "A palette color is not #rrggbb or #rrggbbaa.", with(&|m| m["palettes"] = json!([{"id": "p", "name": "P", "colors": ["#000000", "red"]}]))),
        ("empty-palette", "A palette has no colors.", with(&|m| m["palettes"] = json!([{"id": "p", "name": "P", "colors": []}]))),
        ("unknown-variant-palette", "A variant names a palette that does not exist.", with(&|m| m["variants"] = json!([{"id": "v", "name": "V", "palette": "gold"}]))),
        ("unknown-replacement-drawing", "A variant replaces a drawing with one that does not exist.", with(&|m| m["variants"] = json!([{"id": "v", "name": "V", "replacements": [{"view": "front", "image": "mark", "with": "ghost"}]}]))),
        ("unknown-overlay", "A drawing lists an overlay that does not exist.", with(&|m| m["views"][0]["images"][0]["overlays"] = json!(["ghost"]))),
        ("overlay-on-itself", "A drawing lists itself as an overlay.", with(&|m| m["views"][0]["images"][0]["overlays"] = json!(["mark"]))),
    ]
}

fn main() -> Result<()> {
    let root = Path::new("conformance");
    ensure!(Path::new("Cargo.toml").exists(), "Run from the repository root");
    for dir in ["valid", "invalid", "expected"] {
        let _ = std::fs::remove_dir_all(root.join(dir));
        std::fs::create_dir_all(root.join(dir))?;
    }
    let mut valid_list = Vec::new();
    for case in valid() {
        let bytes = zip(Some(&case.manifest), &case.files);
        let package = Package::read(&bytes).with_context(|| case.name)?;
        let reference = Reference::new(&package).with_context(|| case.name)?;
        std::fs::write(root.join(format!("valid/{}.spu", case.name)), &bytes)?;
        std::fs::create_dir_all(root.join("expected").join(case.name))?;
        let mut frames = Vec::new();
        for (animation, tick, variants) in &case.frames {
            let frame = reference.render("front", animation, *tick, variants)?;
            let mut file = format!("expected/{}/{animation}.{tick}", case.name);
            for v in variants {
                file += &format!(".{v}");
            }
            file += ".png";
            std::fs::write(root.join(&file), frame.png()?)?;
            frames.push(json!({"view": "front", "animation": animation, "tick": tick, "variants": variants, "expected": file}));
        }
        valid_list.push(json!({"file": format!("valid/{}.spu", case.name), "about": case.about, "frames": frames}));
    }
    let mut invalid_list = Vec::new();
    for (name, about, bytes) in invalid() {
        let outcome = Package::read(&bytes).and_then(|p| Reference::new(&p).map(|_| ()));
        ensure!(outcome.is_err(), "{name} should be refused");
        std::fs::write(root.join(format!("invalid/{name}.spu")), &bytes)?;
        invalid_list.push(json!({"file": format!("invalid/{name}.spu"), "about": about}));
    }
    let manifest = json!({"format": "spu", "version": 1, "valid": valid_list, "invalid": invalid_list});
    std::fs::write(root.join("manifest.json"), serde_json::to_string_pretty(&manifest)? + "\n")?;
    println!("{} valid, {} invalid", manifest["valid"].as_array().unwrap().len(), manifest["invalid"].as_array().unwrap().len());
    Ok(())
}
