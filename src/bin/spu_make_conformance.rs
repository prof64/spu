//! Writes the conformance suite under `conformance/`: small files every
//! implementation must accept, the exact picture each must draw, and files
//! every implementation must refuse. Run from the repository root:
//! `cargo run --bin spu-make-conformance`.
//!
//! Most cases are written the way older files were (a ZIP of JSON and PNGs)
//! and converted, which also checks the converter: each must draw exactly
//! what the older reference renderer drew.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use spu::{
    binary, convert,
    legacy::{format::Package, reference::Reference},
    picture::picture_named,
    sprite::*,
};
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
    spu::legacy::format::parse_color(text).unwrap()
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

/// The simplest sprite, in the current format.
fn base() -> Sprite {
    convert::from_legacy(&Package::read(&zip(Some(&still()), &mark_files())).unwrap(), None, true).unwrap()
}

type Frames = Vec<(&'static str, &'static str, f64, Vec<&'static str>)>;

/// Cases for what only the current format can say.
fn native() -> Vec<(&'static str, &'static str, Sprite, Frames)> {
    let mut cases = Vec::new();

    // A hat: a part that stays hidden until a look shows it.
    let source = valid().into_iter().find(|c| c.name == "palettes-and-variants").unwrap();
    let mut s = convert::from_legacy(&Package::read(&zip(Some(&source.manifest), &source.files)).unwrap(), None, true).unwrap();
    let hat = s.drawings.iter().position(|d| d.id == "hat").unwrap() as u16;
    s.parts.push(Part { id: "hat".into(), name: "Hat".into(), extra: true, depth: 0.0 });
    s.views[0].parts.push(ViewPart { part: 1, parent: Some(0) });
    s.animations[0].views[0].as_mut().unwrap().tracks.push(Track { part: 1, image: vec![(0, Some(hat))], ..Track::default() });
    s.variants = vec![
        Variant { id: "with-hat".into(), name: "With hat".into(), show: vec![1], ..Variant::default() },
        Variant { id: "no-body".into(), name: "No body".into(), hide: vec![0], ..Variant::default() },
        Variant { id: "no-hat".into(), name: "No hat".into(), hide: vec![1], ..Variant::default() },
    ];
    cases.push((
        "extra-parts",
        "A part marked extra draws only when an active look shows it. A look can also hide a part. When two looks disagree about a part, the later one wins.",
        s,
        vec![("front", "main", 0.0, vec![]), ("front", "main", 0.0, vec!["with-hat"]), ("front", "main", 0.0, vec!["with-hat", "no-body"]), ("front", "main", 0.0, vec!["with-hat", "no-hat"]), ("front", "main", 0.0, vec!["no-hat", "with-hat"])],
    ));

    // Two views of one sprite, each with its own length for the same animation.
    let mut s = base();
    let mut back = s.views[0].clone();
    back.id = "back".into();
    back.name = "Back".into();
    s.views.push(back);
    let place = s.drawings[0].places[0];
    s.drawings[0].places.push(place);
    let step = |tick, value| Key { tick, value, ease: Ease::Step };
    let mut slide = Track { part: 0, image: vec![(0, Some(0))], ..Track::default() };
    slide.numbers[X] = vec![step(0, 0.0), step(1, 2.0), step(2, 4.0)];
    s.animations[0].views.push(Some(Motion { length: 3, looping: true, tracks: vec![slide] }));
    s.animations.push(Animation { id: "front-only".into(), name: "Front only".into(), views: vec![s.animations[0].views[0].clone(), None] });
    cases.push((
        "two-views",
        "Parts, drawings and animations are shared by every view. Each view keeps its own length and motion for an animation: here 'main' is 1 tick long from the front and 3 from the back. A view an animation has nothing for draws nothing.",
        s,
        vec![("front", "main", 2.0, vec![]), ("back", "main", 0.0, vec![]), ("back", "main", 1.0, vec![]), ("back", "main", 2.0, vec![]), ("back", "main", 4.0, vec![]), ("back", "front-only", 0.0, vec![])],
    ));

    // A color one look has and another does not.
    let mut m = still();
    m["palettes"] = json!([
        {"id": "normal", "name": "Normal", "colors": NORMAL},
        {"id": "ghost", "name": "Ghost", "colors": ["#00000000", "#2080d0", "#d0a02000", "#a020c040", "#ffffff80"]},
    ]);
    m["variants"] = json!([{"id": "ghost", "name": "Ghost", "palette": "ghost"}]);
    let files: Files = vec![("mark.png".into(), indexed_png(&MARK, &NORMAL))];
    let s = convert::from_legacy(&Package::read(&zip(Some(&m), &files)).unwrap(), None, true).unwrap();
    cases.push((
        "palette-opacity",
        "A palette color can carry an opacity of its own, which multiplies the pixel's. Under 'ghost', color 2 vanishes and color 3 is faint, while color 4 keeps the half opacity its pixels have.",
        s,
        vec![("front", "main", 0.0, vec![]), ("front", "main", 0.0, vec!["ghost"])],
    ));
    cases
}

fn motion(s: &mut Sprite) -> &mut Motion {
    s.animations[0].views[0].as_mut().unwrap()
}

fn invalid() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let good = binary::write(&base()).unwrap();
    // A file with something wrong in what it says: written without checking.
    let with = |edit: &dyn Fn(&mut Sprite)| {
        let mut s = base();
        edit(&mut s);
        binary::write_unchecked(&s).unwrap()
    };
    // A file with something wrong in its bytes.
    let bytes = |edit: &dyn Fn(&mut Vec<u8>)| {
        let mut b = good.clone();
        edit(&mut b);
        b
    };
    let relength = |b: &mut Vec<u8>| {
        let n = b.len() as u32;
        b[12..16].copy_from_slice(&n.to_le_bytes());
    };
    let key = |tick, value| Key { tick, value, ease: Ease::Step };
    let lit = |s: &mut Sprite| {
        let c = &s.views[0].color;
        s.views[0].light = Some(LightSheet { width: c.width, height: c.height, data: c.opacity.iter().flat_map(|&o| if o == 0 { [0; 4] } else { [128, 128, 0, 0] }).collect() });
        s.materials.push(Material { name: "plain".into(), ..Material::default() });
    };
    vec![
        ("not-spu", "The file does not start with the eight bytes every SPU file starts with.", b"{\"format\":\"spu\"}".to_vec()),
        ("text-mode-damage", "A line ending inside the first eight bytes was rewritten, as happens when a file is handled as text.", bytes(&|b| { b.remove(4); relength(b); })),
        ("future-version", "The header names a version newer than this specification.", bytes(&|b| b[8] = 3)),
        ("wrong-length", "The header says the file is longer than it is.", bytes(&|b| b[12] = b[12].wrapping_add(8))),
        ("truncated", "The file stops partway through.", bytes(&|b| { b.truncate(b.len() / 2); relength(b); })),
        ("damaged-chunk", "One byte of a chunk's data has changed, so its checksum no longer matches.", bytes(&|b| b[16 + 16 + 3] ^= 0x20)),
        ("unknown-required-chunk", "A chunk whose type starts with a capital letter is one a reader must understand. This one is unknown.", bytes(&|b| {
            let end = b.len() - 16;
            let mut extra = Vec::new();
            extra.extend(b"FOGS");
            extra.extend(8u32.to_le_bytes());
            extra.extend(binary::crc32(&[b"FOGS", &[1; 8]]).to_le_bytes());
            extra.extend([0; 4]);
            extra.extend([1; 8]);
            b.splice(end..end, extra);
            relength(b);
        })),
        ("missing-chunk", "The file has no list of drawings.", bytes(&|b| {
            let at = b.windows(4).position(|w| w == b"DRAW").unwrap();
            let length = u32::from_le_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
            b.drain(at..at + 16 + length.next_multiple_of(8));
            relength(b);
        })),
        ("after-the-end", "Bytes follow the last chunk.", bytes(&|b| { b.extend([0; 8]); relength(b); })),
        ("no-views", "There are no views.", with(&|s| { s.views.clear(); s.drawings[0].places.clear(); s.animations[0].views.clear(); })),
        ("zero-length-animation", "An animation is zero ticks long.", with(&|s| motion(s).length = 0)),
        ("empty-canvas", "The canvas has no width.", with(&|s| s.views[0].width = 0)),
        ("duplicate-part-id", "Two parts share an id.", with(&|s| s.parts.push(s.parts[0].clone()))),
        ("empty-id", "An id is empty.", with(&|s| s.views[0].id.clear())),
        ("parent-not-shown", "A part is joined to a part its view does not show.", with(&|s| s.views[0].parts[0].parent = Some(5))),
        ("parent-loop", "Two parts are each joined to the other.", with(&|s| {
            s.parts.push(Part { id: "tail".into(), name: "Tail".into(), ..Part::default() });
            s.views[0].parts[0].parent = Some(1);
            s.views[0].parts.push(ViewPart { part: 1, parent: Some(0) });
        })),
        ("part-listed-twice", "A view lists the same part twice.", with(&|s| { let p = s.views[0].parts[0]; s.views[0].parts.push(p); })),
        ("track-for-absent-part", "An animation moves a part its view does not show.", with(&|s| {
            s.parts.push(Part { id: "tail".into(), name: "Tail".into(), ..Part::default() });
            motion(s).tracks[0].part = 1;
        })),
        ("two-tracks-one-part", "An animation has two tracks for the same part.", with(&|s| { let t = motion(s).tracks[0].clone(); motion(s).tracks.push(t); })),
        ("unknown-drawing-in-key", "A key shows a drawing that does not exist.", with(&|s| motion(s).tracks[0].image = vec![(0, Some(9))])),
        ("keys-out-of-order", "Keys are not in increasing tick order.", with(&|s| motion(s).tracks[0].numbers[X] = vec![key(5, 1.0), key(2, 0.0)])),
        ("two-keys-one-tick", "Two keys share a tick.", with(&|s| motion(s).tracks[0].numbers[X] = vec![key(2, 1.0), key(2, 0.0)])),
        ("number-not-finite", "An animated number is not a finite number.", with(&|s| motion(s).tracks[0].numbers[X] = vec![key(0, f64::NAN)])),
        ("unknown-ease", "A key uses an easing curve that does not exist.", {
            // The only key in the file is the x key written here; its ease is the last byte of the animation's data.
            let mut s = base();
            motion(&mut s).tracks[0].numbers[EMISSION] = vec![key(0, 1.0)];
            let mut b = binary::write_stored(&s).unwrap();
            let at = b.windows(4).position(|w| w == b"ANIM").unwrap();
            let length = u32::from_le_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
            b[at + 16 + length - 1] = 9;
            let sum = binary::crc32(&[b"ANIM", &b[at + 16..at + 16 + length]]);
            b[at + 8..at + 12].copy_from_slice(&sum.to_le_bytes());
            b
        }),
        ("color-not-in-palette", "A pixel uses a color number the palette does not have.", with(&|s| { let i = s.views[0].color.opacity.iter().position(|&o| o > 0).unwrap(); s.views[0].color.number[i] = 400; })),
        ("palettes-of-different-length", "Two palettes have different numbers of colors.", with(&|s| { let mut p = s.palettes[0].clone(); p.id = "short".into(); p.colors.pop(); s.palettes.push(p); })),
        ("no-palette", "The sprite has no palette.", with(&|s| s.palettes.clear())),
        ("unknown-variant-palette", "A look names a palette that does not exist.", with(&|s| s.variants.push(Variant { id: "v".into(), name: "V".into(), palette: Some(7), ..Variant::default() }))),
        ("unknown-replacement-drawing", "A look replaces a drawing with one that does not exist.", with(&|s| s.variants.push(Variant { id: "v".into(), name: "V".into(), replace: vec![Replace { view: 0, from: 0, to: 9 }], ..Variant::default() }))),
        ("look-shows-unknown-part", "A look shows a part that does not exist.", with(&|s| s.variants.push(Variant { id: "v".into(), name: "V".into(), show: vec![9], ..Variant::default() }))),
        ("drawing-off-the-sheet", "A drawing's rectangle reaches past the edge of its sheet.", with(&|s| s.drawings[0].places[0].as_mut().unwrap().width = 4000)),
        ("light-sheet-wrong-size", "A view's Light sheet is not the size of its Color sheet.", with(&|s| { lit(s); let l = s.views[0].light.as_mut().unwrap(); l.height -= 1; l.data.truncate((l.width * l.height * 4) as usize); })),
        ("unknown-material", "A pixel names a material the sprite does not have.", with(&|s| { lit(s); let i = s.views[0].color.opacity.iter().position(|&o| o > 0).unwrap(); s.views[0].light.as_mut().unwrap().data[i * 4 + 2] = 3; })),
        ("material-dial-out-of-range", "A material's finish is above 1.", with(&|s| { lit(s); s.materials[0].finish = 4.0; })),
        ("shade-not-in-palette", "A material lists a shade the palette does not have.", with(&|s| { lit(s); s.materials[0].shades = vec![1, 400]; })),
        ("base-not-a-shade", "A material's base is not one of its shades.", with(&|s| { lit(s); s.materials[0].shades = vec![1, 2]; s.materials[0].base = 2; })),
    ]
}

fn main() -> Result<()> {
    let root = Path::new("conformance");
    ensure!(Path::new("Cargo.toml").exists(), "Run from the repository root");
    for dir in ["valid", "invalid", "expected"] {
        let _ = std::fs::remove_dir_all(root.join(dir));
        std::fs::create_dir_all(root.join(dir))?;
    }
    let mut cases: Vec<(&str, &str, Sprite, Frames)> = Vec::new();
    for case in valid() {
        let package = Package::read(&zip(Some(&case.manifest), &case.files)).with_context(|| case.name)?;
        // Converted exactly, each must draw what the older renderer drew.
        let (old, exact) = (Reference::new(&package)?, convert::from_legacy(&package, None, false)?);
        for (animation, tick, variants) in &case.frames {
            ensure!(picture_named(&exact, "front", animation, *tick, variants)? == old.render("front", animation, *tick, variants)?, "{}: the converter changed a picture", case.name);
        }
        let mut sprite = convert::from_legacy(&package, None, true)?;
        if case.name == "unknown-fields" {
            sprite.unknown.push((*b"note", b"A chunk this reader does not know.".to_vec()));
        }
        cases.push((case.name, case.about, sprite, case.frames.into_iter().map(|(a, t, v)| ("front", a, t, v)).collect()));
    }
    cases.extend(native());

    let mut valid_list = Vec::new();
    for (name, about, sprite, frames) in cases {
        let bytes = binary::write(&sprite).with_context(|| name)?;
        let read = binary::read(&bytes).with_context(|| name)?;
        ensure!(read == sprite && binary::write(&read)? == bytes, "{name} does not survive being written and read");
        std::fs::write(root.join(format!("valid/{name}.spu")), &bytes)?;
        std::fs::create_dir_all(root.join("expected").join(name))?;
        let mut list = Vec::new();
        for (view, animation, tick, variants) in &frames {
            let mut file = format!("expected/{name}/{view}.{animation}.{tick}");
            for v in variants {
                file += &format!(".{v}");
            }
            file += ".png";
            std::fs::write(root.join(&file), picture_named(&read, view, animation, *tick, variants)?.png()?)?;
            list.push(json!({"view": view, "animation": animation, "tick": tick, "variants": variants, "expected": file}));
        }
        valid_list.push(json!({"file": format!("valid/{name}.spu"), "about": about, "frames": list}));
    }
    let mut invalid_list = Vec::new();
    for (name, about, bytes) in invalid() {
        ensure!(binary::read(&bytes).is_err(), "{name} should be refused");
        std::fs::write(root.join(format!("invalid/{name}.spu")), &bytes)?;
        invalid_list.push(json!({"file": format!("invalid/{name}.spu"), "about": about}));
    }
    let manifest = json!({"format": "spu", "version": binary::VERSION, "valid": valid_list, "invalid": invalid_list});
    std::fs::write(root.join("manifest.json"), serde_json::to_string_pretty(&manifest)? + "\n")?;
    println!("{} valid, {} invalid", manifest["valid"].as_array().unwrap().len(), manifest["invalid"].as_array().unwrap().len());
    Ok(())
}
