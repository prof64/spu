//! Writes the conformance suite under `conformance/`: small files every
//! implementation must accept, the exact picture each must draw, and files
//! every implementation must refuse. Run from the repository root:
//! `cargo run --bin spu-make-conformance`.
use anyhow::{Context, Result, ensure};
use serde_json::json;
use spu::{binary, picture::picture_named, sprite::*};
use std::path::Path;

/// An asymmetric 5×6 mark, so turns and mirrors are unmistakable. Each digit
/// is a color number; `.` is no pixel. Color 4's pixels are half transparent.
const MARK: [&str; 6] = ["1111.", "1....", "122..", "1....", "1..3.", "1..44"];
const NORMAL: [[u8; 4]; 5] = [[0, 0, 0, 255], [0xd0, 0x20, 0x20, 255], [0x20, 0xa0, 0x40, 255], [0x20, 0x40, 0xd0, 255], [0xf0, 0xc0, 0x20, 255]];
const SHINY: [[u8; 4]; 5] = [[0, 0, 0, 255], [0x20, 0x80, 0xd0, 255], [0xd0, 0xa0, 0x20, 255], [0xa0, 0x20, 0xc0, 255], [255, 255, 255, 255]];
/// Color 2 vanishes and color 3 is faint.
const GHOST: [[u8; 4]; 5] = [[0, 0, 0, 255], [0x20, 0x80, 0xd0, 255], [0xd0, 0xa0, 0x20, 0], [0xa0, 0x20, 0xc0, 0x40], [255, 255, 255, 255]];

/// A drawing: id, name, rows of color numbers, and where its top-left corner
/// sits from the origin of the part showing it.
type Art<'a> = (&'a str, &'a str, &'a [&'a str], f32, f32);
const MARK_ART: Art = ("mark", "Mark", &MARK, 0.0, 0.0);

fn palette(id: &str, name: &str, colors: &[[u8; 4]]) -> Palette {
    Palette { id: id.into(), name: name.into(), colors: colors.to_vec() }
}

/// One view, a 24×24 canvas centered on the origin. `parts` are (id, name,
/// parent), shown in this order. The drawings sit in a row on the view's
/// sheet, with an empty pixel around each.
fn sprite(parts: &[(&str, &str, Option<u16>)], art: &[Art], animations: Vec<Animation>) -> Sprite {
    let (width, height) = (64u32, art.iter().map(|a| a.2.len() as u32).max().unwrap() + 2);
    let n = (width * height) as usize;
    let mut color = ColorSheet { width, height, number: vec![0; n], opacity: vec![0; n] };
    let mut drawings = Vec::new();
    let mut x = 1;
    for &(id, name, rows, offset_x, offset_y) in art {
        for (row, line) in rows.iter().enumerate() {
            for (column, b) in line.bytes().enumerate() {
                if b != b'.' {
                    let i = (1 + row) * width as usize + x + column;
                    color.number[i] = u16::from(b - b'0');
                    color.opacity[i] = if b == b'4' { 128 } else { 255 };
                }
            }
        }
        let (w, h) = (rows[0].len(), rows.len());
        drawings.push(Drawing { id: id.into(), name: name.into(), places: vec![Some(Place { x: x as u16, y: 1, width: w as u16, height: h as u16, offset_x, offset_y })] });
        x += w + 1;
    }
    assert!(x <= width as usize, "The drawings do not fit on the sheet");
    Sprite {
        name: "case".into(),
        version: 1,
        ticks_per_second: 60.0,
        views: vec![View {
            id: "front".into(),
            name: "Front".into(),
            x: -12,
            y: -12,
            width: 24,
            height: 24,
            parts: parts.iter().enumerate().map(|(i, p)| ViewPart { part: i as u16, parent: p.2 }).collect(),
            color,
            light: None,
        }],
        parts: parts.iter().map(|p| Part { id: p.0.into(), name: p.1.into(), ..Part::default() }).collect(),
        drawings,
        palettes: vec![palette("normal", "Normal", &NORMAL)],
        animations,
        ..Sprite::default()
    }
}

/// An animation with motion for the one view.
fn animation(id: &str, name: &str, length: u32, looping: bool, tracks: Vec<Track>) -> Animation {
    Animation { id: id.into(), name: name.into(), views: vec![Some(Motion { length, looping, tracks })] }
}
fn main_animation(length: u32, looping: bool, tracks: Vec<Track>) -> Animation {
    animation("main", "Main", length, looping, tracks)
}
/// Keys for one number, as (tick, value, ease).
type Keys<'a> = &'a [(u32, f64, Ease)];
/// A part's drawings from each tick on, and keys for some of its numbers.
fn track(part: u16, image: &[(u32, Option<u16>)], numbers: &[(usize, Keys)]) -> Track {
    let mut t = Track { part, image: image.to_vec(), ..Track::default() };
    for &(property, keys) in numbers {
        t.numbers[property] = keys.iter().map(|&(tick, value, ease)| Key { tick, value, ease }).collect();
    }
    t
}
/// Keys that each hold until the next.
fn held(keys: &[(u32, f64)]) -> Vec<(u32, f64, Ease)> {
    keys.iter().map(|&(tick, value)| (tick, value, Ease::Step)).collect()
}
/// The mark, still, on one part.
fn still() -> Sprite {
    sprite(&[("body", "Body", None)], &[MARK_ART], vec![main_animation(1, true, vec![track(0, &[(0, Some(0))], &[])])])
}
/// A Light sheet the size of the Color sheet, `pixel` wherever a pixel is.
fn light(c: &ColorSheet, pixel: [u8; 4]) -> LightSheet {
    LightSheet { width: c.width, height: c.height, data: c.opacity.iter().flat_map(|&o| if o == 0 { [0; 4] } else { pixel }).collect() }
}

/// (view, animation, tick, variants)
type Frames = Vec<(&'static str, &'static str, f64, Vec<&'static str>)>;

struct Valid {
    name: &'static str,
    about: &'static str,
    sprite: Sprite,
    frames: Frames,
}

fn palettes_and_variants() -> Sprite {
    let hat: Art = ("hat", "Hat", &["..33..", ".3333.", "222222"], -2.0, -3.0);
    let mut s = sprite(&[("body", "Body", None)], &[MARK_ART, hat], vec![main_animation(1, true, vec![track(0, &[(0, Some(0))], &[])])]);
    s.palettes.push(palette("shiny", "Shiny", &SHINY));
    s.variants = vec![
        Variant { id: "shiny".into(), name: "Shiny".into(), palette: Some(1), ..Variant::default() },
        Variant { id: "hat".into(), name: "Hat".into(), replace: vec![Replace { view: 0, from: 0, to: 1 }], ..Variant::default() },
    ];
    s
}

fn valid() -> Vec<Valid> {
    use Ease::*;
    let ticks = |list: &[u32]| list.iter().map(|&t| ("front", "main", f64::from(t), vec![])).collect::<Frames>();
    let front = |list: &[(&'static str, f64, &[&'static str])]| list.iter().map(|&(a, t, v)| ("front", a, t, v.to_vec())).collect::<Frames>();
    // Part 0 showing the mark.
    let mark = |numbers: &[(usize, Keys)]| track(0, &[(0, Some(0))], numbers);
    let mut cases = Vec::new();
    cases.push(Valid {
        name: "still",
        about: "One drawing at the origin. The picture is the drawing itself, placed on the canvas.",
        sprite: still(),
        frames: ticks(&[0, 5]),
    });

    cases.push(Valid {
        name: "palettes-and-variants",
        about: "A drawing's pixels are color numbers into the active palette; color 4's pixels are half transparent. One variant swaps the palette, another swaps the drawing, and both can be active together.",
        sprite: palettes_and_variants(),
        frames: front(&[("main", 0.0, &[]), ("main", 0.0, &["shiny"]), ("main", 0.0, &["hat"]), ("main", 0.0, &["shiny", "hat"]), ("main", 0.0, &["hat", "shiny"])]),
    });

    let x = [(0, -10.0, Step), (10, -10.0, Linear), (20, 0.0, EaseIn), (30, 5.0, EaseOut), (40, -5.0, EaseInOut), (50, 3.0, Step)];
    cases.push(Valid {
        name: "easing",
        about: "Each key moves toward the next along its own curve: step, linear, easeIn, easeOut, easeInOut. Positions between pixels are sampled, not rounded.",
        sprite: sprite(&[("body", "Body", None)], &[MARK_ART], vec![main_animation(60, true, vec![mark(&[(X, &x), (Y, &[(0, -3.0, Linear), (50, 2.5, Step)])])])]),
        frames: ticks(&[0, 9, 10, 13, 15, 20, 23, 27, 30, 33, 37, 40, 43, 45, 47, 50, 59]),
    });

    let rotation = held(&[(0, 0.0), (1, 90.0), (2, 180.0), (3, 270.0), (4, 45.0), (5, 33.0), (6, -90.0), (7, 0.0), (11, 30.0)]);
    let scale_x = held(&[(0, 1.0), (7, 2.0), (8, -1.0), (9, 0.5), (10, 1.5), (11, -2.0), (12, 0.0)]);
    let scale_y = held(&[(0, 1.0), (7, 2.0), (8, 1.0), (9, 3.0), (10, -1.5), (11, 1.25)]);
    cases.push(Valid {
        name: "rotation-and-scale",
        about: "A part turns clockwise about its origin and scales per axis; a negative scale mirrors. Pixels come from nearest-neighbor sampling at pixel centers.",
        sprite: sprite(&[("body", "Body", None)], &[MARK_ART], vec![main_animation(13, false, vec![mark(&[(ROTATION, &rotation), (SCALE_X, &scale_x), (SCALE_Y, &scale_y)])])]),
        frames: ticks(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
    });

    // The view lists arm, body, hand; the parent is listed after its child.
    let (arm, body, hand) = (0, 1, 2);
    let tracks = vec![
        track(body, &[(0, Some(0))], &[
            (X, &held(&[(0, -4.0)])),
            (Y, &held(&[(0, -6.0)])),
            (ROTATION, &held(&[(0, 0.0), (1, 90.0), (2, 20.0), (3, 0.0)])),
            (SCALE_X, &held(&[(0, 1.0), (3, 2.0), (4, -1.0), (5, -2.0)])),
            (SCALE_Y, &held(&[(0, 1.0), (5, 0.5)])),
        ]),
        track(arm, &[(0, Some(0))], &[(X, &held(&[(0, 5.0)])), (Y, &held(&[(0, 1.0)])), (ROTATION, &held(&[(0, 0.0), (2, 45.0)]))]),
        track(hand, &[(0, Some(0))], &[(X, &held(&[(0, 2.0)])), (Y, &held(&[(0, 6.0)])), (ROTATION, &held(&[(0, 0.0), (4, 30.0)])), (SCALE_X, &held(&[(0, 1.0), (1, 0.5)]))]),
    ];
    cases.push(Valid {
        name: "joined-parts",
        about: "A child follows its parent: its position goes through the parent's whole placement, its rotation adds (reversed under a mirrored parent), its scale multiplies. A child never skews, even under a parent squashed on one axis. A parent may be listed after its child.",
        sprite: sprite(&[("arm", "Arm", Some(body)), ("body", "Body", None), ("hand", "Hand", Some(arm))], &[MARK_ART], vec![main_animation(6, true, tracks)]),
        frames: ticks(&[0, 1, 2, 3, 4, 5]),
    });

    // Drawing 0 is the mark, 1 the block. Part a shows no drawing from tick 2
    // (hidden) and from tick 4.
    let tracks = vec![
        track(0, &[(0, Some(0)), (2, None), (3, Some(0)), (4, None), (5, Some(0))], &[(Z, &held(&[(0, 0.0), (1, 2.0)]))]),
        track(1, &[(0, Some(1))], &[(X, &held(&[(0, -1.0)])), (Y, &held(&[(0, 1.0)])), (Z, &held(&[(0, 0.0), (5, 2.0)]))]),
        track(2, &[(0, Some(1))], &[(X, &held(&[(0, 2.0)])), (Y, &held(&[(0, 3.0)])), (Z, &held(&[(0, -1.0), (3, 1.0)]))]),
    ];
    cases.push(Valid {
        name: "draw-order-and-visibility",
        about: "Parts draw from lowest z to highest; equal z draws in the order parts are listed. A hidden part or a part with no drawing draws nothing, but its children still draw: a parent passes on placement only.",
        sprite: sprite(&[("a", "A", None), ("b", "B", None), ("c", "C", Some(0))], &[MARK_ART, ("block", "Block", &["222", "232", "222"], 0.0, 0.0)], vec![main_animation(6, true, tracks)]),
        frames: ticks(&[0, 1, 2, 3, 4, 5]),
    });

    let tracks = vec![
        mark(&[(OPACITY, &held(&[(0, 1.0), (2, 0.25)]))]),
        track(1, &[(0, Some(0))], &[(X, &held(&[(0, 2.0)])), (Y, &held(&[(0, 1.0)])), (OPACITY, &[(0, 0.5, Linear), (3, 0.0, Step)])]),
    ];
    cases.push(Valid {
        name: "opacity",
        about: "A part's opacity multiplies its drawing's own transparency and blends over what is behind. A parent's opacity does not fade its children.",
        sprite: sprite(&[("back", "Back", None), ("front", "Front", Some(0))], &[MARK_ART], vec![main_animation(4, true, tracks)]),
        frames: ticks(&[0, 1, 2, 3]),
    });

    let slide = mark(&[(X, &[(2, -8.0, Linear), (12, 2.0, Step)])]);
    cases.push(Valid {
        name: "looping-and-holding",
        about: "Before a property's first key, the first key's value holds. A looping animation wraps at its length; one that does not loop holds its last tick. A key past the end is never reached but still shapes the motion toward it.",
        sprite: sprite(&[("body", "Body", None)], &[MARK_ART], vec![main_animation(8, true, vec![slide.clone()]), animation("once", "Once", 8, false, vec![slide])]),
        frames: front(&[("main", 0.0, &[]), ("main", 2.0, &[]), ("main", 7.0, &[]), ("main", 8.0, &[]), ("main", 21.0, &[]), ("once", 7.0, &[]), ("once", 8.0, &[]), ("once", 500.0, &[])]),
    });

    // The eyes are a part of their own, joined to the body. A look swaps their
    // drawing (1, open) for another (2, closed).
    let eyes: [Art; 2] = [("open", "Eyes open", &["3.3"], 1.0, 1.0), ("closed", "Eyes closed", &["222"], 1.0, 2.0)];
    let tracks = vec![mark(&[(ROTATION, &held(&[(0, 0.0), (1, 90.0)]))]), track(1, &[(0, Some(1))], &[])];
    let mut s = sprite(&[("body", "Body", None), ("body-overlay", "Body overlay", Some(0))], &[MARK_ART, eyes[0], eyes[1]], vec![main_animation(2, true, tracks)]);
    s.variants = vec![Variant { id: "blink".into(), name: "Blink".into(), replace: vec![Replace { view: 0, from: 1, to: 2 }], ..Variant::default() }];
    cases.push(Valid {
        name: "overlays",
        about: "Eyes painted over the body as a part of their own, joined to it, turn with it. A variant replaces the eyes' drawing, which is how a blink is stored.",
        sprite: s,
        frames: front(&[("main", 0.0, &[]), ("main", 0.0, &["blink"]), ("main", 1.0, &["blink"])]),
    });

    let mut s = still();
    s.extras = vec![Extra { path: "notes/readme.txt".into(), bytes: b"Extras are kept.".to_vec() }];
    s.unknown.push((*b"note", b"A chunk this reader does not know.".to_vec()));
    cases.push(Valid {
        name: "unknown-fields",
        about: "A chunk a reader does not know, and the extras a file carries, are ignored when drawing and kept when saving. The picture equals the 'still' case.",
        sprite: s,
        frames: ticks(&[0]),
    });

    let numbers: [(usize, Keys); 4] = [(X, &held(&[(0, 3.0)])), (Y, &held(&[(0, -2.0)])), (SCALE_X, &held(&[(0, 2.0)])), (SCALE_Y, &held(&[(0, 2.0)]))];
    let mut s = sprite(&[("body", "Body", None)], &[MARK_ART], vec![main_animation(1, true, vec![mark(&numbers)])]);
    (s.views[0].x, s.views[0].y, s.views[0].width, s.views[0].height) = (5, 0, 7, 9);
    cases.push(Valid {
        name: "canvas",
        about: "The canvas is a window onto sprite space: its x and y say which point is the top-left pixel, and anything outside it is cut off.",
        sprite: s,
        frames: ticks(&[0]),
    });

    let mut s = sprite(&[("body", "Body", None)], &[MARK_ART], vec![main_animation(1, true, vec![mark(&[(EMISSION, &held(&[(0, 4.0)]))])])]);
    s.materials = vec![Material { name: "default".into(), finish: 0.35, ..Material::default() }, Material { name: "Glass".into(), finish: 0.9, body: 0.9, ..Material::default() }];
    // Every pixel faces the viewer, is glass, and glows fully.
    s.views[0].light = Some(light(&s.views[0].color, [128, 128, 1, 255]));
    cases.push(Valid {
        name: "materials-do-not-change-the-reference",
        about: "Materials and the Light sheet describe how light treats a drawing. The reference picture has no light, so it equals the 'still' case.",
        sprite: s,
        frames: ticks(&[0]),
    });

    // A hat: a part that stays hidden until a look shows it.
    let mut s = palettes_and_variants();
    s.parts.push(Part { id: "hat".into(), name: "Hat".into(), extra: true, depth: 0.0 });
    s.views[0].parts.push(ViewPart { part: 1, parent: Some(0) });
    s.animations[0].views[0].as_mut().unwrap().tracks.push(track(1, &[(0, Some(1))], &[]));
    s.variants = vec![
        Variant { id: "with-hat".into(), name: "With hat".into(), show: vec![1], ..Variant::default() },
        Variant { id: "no-body".into(), name: "No body".into(), hide: vec![0], ..Variant::default() },
        Variant { id: "no-hat".into(), name: "No hat".into(), hide: vec![1], ..Variant::default() },
    ];
    cases.push(Valid {
        name: "extra-parts",
        about: "A part marked extra draws only when an active look shows it. A look can also hide a part. When two looks disagree about a part, the later one wins.",
        sprite: s,
        frames: front(&[("main", 0.0, &[]), ("main", 0.0, &["with-hat"]), ("main", 0.0, &["with-hat", "no-body"]), ("main", 0.0, &["with-hat", "no-hat"]), ("main", 0.0, &["no-hat", "with-hat"])]),
    });

    // Two views of one sprite, each with its own length for the same animation.
    let mut s = still();
    let mut back = s.views[0].clone();
    back.id = "back".into();
    back.name = "Back".into();
    s.views.push(back);
    let place = s.drawings[0].places[0];
    s.drawings[0].places.push(place);
    s.animations[0].views.push(Some(Motion { length: 3, looping: true, tracks: vec![mark(&[(X, &held(&[(0, 0.0), (1, 2.0), (2, 4.0)]))])] }));
    s.animations.push(Animation { id: "front-only".into(), name: "Front only".into(), views: vec![s.animations[0].views[0].clone(), None] });
    cases.push(Valid {
        name: "two-views",
        about: "Parts, drawings and animations are shared by every view. Each view keeps its own length and motion for an animation: here 'main' is 1 tick long from the front and 3 from the back. A view an animation has nothing for draws nothing.",
        sprite: s,
        frames: vec![("front", "main", 2.0, vec![]), ("back", "main", 0.0, vec![]), ("back", "main", 1.0, vec![]), ("back", "main", 2.0, vec![]), ("back", "main", 4.0, vec![]), ("back", "front-only", 0.0, vec![])],
    });

    // A color one look has and another does not.
    let mut s = still();
    s.palettes.push(palette("ghost", "Ghost", &GHOST));
    s.variants = vec![Variant { id: "ghost".into(), name: "Ghost".into(), palette: Some(1), ..Variant::default() }];
    cases.push(Valid {
        name: "palette-opacity",
        about: "A palette color can carry an opacity of its own, which multiplies the pixel's. Under 'ghost', color 2 vanishes and color 3 is faint, while color 4 keeps the half opacity its pixels have.",
        sprite: s,
        frames: front(&[("main", 0.0, &[]), ("main", 0.0, &["ghost"])]),
    });
    cases
}

fn motion(s: &mut Sprite) -> &mut Motion {
    s.animations[0].views[0].as_mut().unwrap()
}

fn invalid() -> Vec<(&'static str, &'static str, Vec<u8>)> {
    let good = binary::write(&still()).unwrap();
    // A file with something wrong in what it says: written without checking.
    let with = |edit: &dyn Fn(&mut Sprite)| {
        let mut s = still();
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
        s.views[0].light = Some(light(&s.views[0].color, [128, 128, 0, 0]));
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
            let mut s = still();
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
    let mut valid_list = Vec::new();
    for Valid { name, about, sprite, frames } in valid() {
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
