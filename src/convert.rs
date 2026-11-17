//! Turning an older file (a ZIP of `sprite.json` and PNGs) into the current
//! format, and checking that it still draws the same pictures.
use crate::legacy::{
    format::{self as old, Package, parse_color},
    reference::{Pixels, Reference, decode},
};
use crate::picture::picture;
use crate::sprite::*;
use anyhow::{Context, Result, bail, ensure};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

/// Lighting made outside this crate, to add while converting: the material
/// table, and for each drawing an RGBA image of facing x, facing y, material
/// number and glow.
#[derive(Default)]
pub struct Lighting {
    pub materials: Vec<Material>,
    /// Part id to depth.
    pub depth: HashMap<String, f32>,
    /// (view id, drawing id) to RGBA pixels the size of the drawing.
    pub drawings: HashMap<(String, String), Vec<u8>>,
}

impl Lighting {
    /// Read a folder holding `materials.json` and `<view>/<drawing>.png`.
    pub fn load(folder: &Path) -> Result<Self> {
        let text = std::fs::read(folder.join("materials.json")).context("The lighting folder has no materials.json")?;
        let json: serde_json::Value = serde_json::from_slice(&text)?;
        let mut light = Lighting::default();
        for m in json["materials"].as_array().context("materials.json has no materials")? {
            light.materials.push(Material {
                name: m["name"].as_str().unwrap_or_default().to_owned(),
                finish: m["finish"].as_f64().unwrap_or(0.0) as f32,
                body: m["body"].as_f64().unwrap_or(0.0) as f32,
                base: m["base"].as_u64().unwrap_or(0) as u16,
                shades: m["shades"].as_array().map(|a| a.iter().filter_map(|c| c.as_u64()).map(|c| c as u16).collect()).unwrap_or_default(),
            });
        }
        if let Some(depth) = json["depth"].as_object() {
            for (part, z) in depth {
                light.depth.insert(part.clone(), z.as_f64().unwrap_or(0.0) as f32);
            }
        }
        for view in std::fs::read_dir(folder)? {
            let view = view?;
            if !view.file_type()?.is_dir() {
                continue;
            }
            for file in std::fs::read_dir(view.path())? {
                let path = file?.path();
                if path.extension().is_some_and(|e| e == "png") {
                    let image = image::load_from_memory(&std::fs::read(&path)?)?.to_rgba8();
                    let drawing = path.file_stem().unwrap().to_string_lossy().into_owned();
                    light.drawings.insert((view.file_name().to_string_lossy().into_owned(), drawing), image.into_raw());
                }
            }
        }
        Ok(light)
    }
}

fn whole(tick: f64, what: &str) -> Result<u32> {
    ensure!(tick >= 0.0 && tick <= u32::MAX as f64 && tick.fract() == 0.0, "{what}: tick {tick} is not a whole number");
    Ok(tick as u32)
}
fn ease(e: old::Ease) -> Ease {
    match e {
        old::Ease::Step => Ease::Step,
        old::Ease::Linear => Ease::Linear,
        old::Ease::EaseIn => Ease::EaseIn,
        old::Ease::EaseOut => Ease::EaseOut,
        old::Ease::EaseInOut => Ease::EaseInOut,
    }
}
/// Keys as stored: a key that repeats the value before it, with both held
/// rather than eased, says nothing and is dropped.
fn numbers(keys: &[old::Key<f64>], what: &str) -> Result<Vec<Key>> {
    let mut out: Vec<Key> = Vec::new();
    for k in keys {
        let key = Key { tick: whole(k.tick, what)?, value: k.value, ease: ease(k.ease) };
        if out.last().is_some_and(|p| p.ease == Ease::Step && key.ease == Ease::Step && p.value == key.value) {
            continue;
        }
        out.push(key);
    }
    Ok(out)
}
fn held<T: Clone>(keys: &[old::Key<T>], tick: f64, default: T) -> T {
    let i = keys.partition_point(|k| k.tick <= tick);
    keys.get(i.saturating_sub(1)).map_or(default, |k| k.value.clone())
}

/// One drawing's pixels, ready for a sheet.
struct Art {
    width: u32,
    height: u32,
    number: Vec<u16>,
    opacity: Vec<u8>,
    light: Option<Vec<u8>>,
}

/// Pack rectangles onto a sheet in shelves, tallest first, with an empty
/// pixel around each. Returns each rectangle's corner and the sheet's size.
fn pack(sizes: &[(u32, u32)]) -> Result<(Vec<(u32, u32)>, u32, u32)> {
    let area: u64 = sizes.iter().map(|&(w, h)| u64::from(w + 1) * u64::from(h + 1)).sum();
    let widest = sizes.iter().map(|s| s.0).max().unwrap_or(0) + 2;
    let mut width = 64u32;
    while width < widest || u64::from(width) * u64::from(width) < area {
        width *= 2;
    }
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(sizes[i].1), i));
    let mut at = vec![(0, 0); sizes.len()];
    let (mut x, mut y, mut shelf) = (1, 1, 0);
    for i in order {
        let (w, h) = sizes[i];
        if x + w + 1 > width {
            x = 1;
            y += shelf + 1;
            shelf = 0;
        }
        at[i] = (x, y);
        x += w + 1;
        shelf = shelf.max(h);
    }
    let height = y + shelf + 1;
    ensure!(width <= MAX_SHEET && height <= MAX_SHEET, "The drawings do not fit on one sheet");
    Ok((at, width, height))
}

/// The current form of an older file.
///
/// `fix_overlays`: version 1 painted the *original* drawing's overlay (its
/// eyes) over a drawing that a variant had replaced. When true, a variant
/// that replaces a drawing also replaces its overlay with the replacement's.
/// Pass false to get exactly version 1's pictures.
pub fn from_legacy(package: &Package, light: Option<&Lighting>, fix_overlays: bool) -> Result<Sprite> {
    let doc = &package.document;
    let mut s = Sprite { name: doc.name.clone(), version: 1, ticks_per_second: doc.ticks_per_second, ..Sprite::default() };

    // Palettes. Where every palette gives a color number the same opacity, that
    // opacity moves to the pixels and the color becomes solid. Where palettes
    // disagree (a color one look has and another does not), it stays put.
    let indexed = !doc.palettes.is_empty();
    // For each color number: the opacity its pixels take, or none if it stays in the palettes.
    let mut lifted: Vec<Option<u8>> = Vec::new();
    if indexed {
        let longest = doc.palettes.iter().map(|p| p.colors.len()).max().unwrap();
        let mut all: Vec<Vec<[u8; 4]>> = Vec::new();
        for p in &doc.palettes {
            let mut colors = p.colors.iter().map(|c| parse_color(c)).collect::<Result<Vec<_>>>()?;
            colors.resize(longest, [0, 0, 0, 0]);
            colors[0][3] = 0;
            all.push(colors);
        }
        lifted = (0..longest).map(|i| Some(all[0][i][3]).filter(|&a| all.iter().all(|p| p[i][3] == a))).collect();
        for (p, mut colors) in doc.palettes.iter().zip(all) {
            for (i, c) in colors.iter_mut().enumerate() {
                if lifted[i].is_some() {
                    c[3] = 255;
                }
            }
            s.palettes.push(Palette { id: p.id.clone(), name: p.name.clone(), colors });
        }
    } else {
        s.palettes.push(Palette { id: "normal".into(), name: "Normal".into(), colors: vec![[0, 0, 0, 255]] });
    }
    // Full-color pixels get a color number of their own, added to every palette.
    let mut added: HashMap<[u8; 3], u16> = HashMap::new();
    let mut number_of = |s: &mut Sprite, rgb: [u8; 3]| -> Result<u16> {
        if let Some(&n) = added.get(&rgb) {
            return Ok(n);
        }
        let n = s.palettes[0].colors.len();
        ensure!(n < MAX_COLORS, "More than {MAX_COLORS} colors");
        for p in &mut s.palettes {
            p.colors.push([rgb[0], rgb[1], rgb[2], 255]);
        }
        added.insert(rgb, n as u16);
        Ok(n as u16)
    };

    // Materials, when the file has lighting or lighting is being added.
    let has_maps = doc.views.iter().any(|v| v.images.iter().any(|i| i.normal.is_some() || i.emission.is_some()));
    let mut material_row: HashMap<&str, u8> = HashMap::new();
    if let Some(light) = light {
        s.materials = light.materials.clone();
    } else if has_maps {
        s.materials.push(Material { name: "default".into(), finish: 0.35, ..Material::default() });
        material_row.insert("default", 0);
        for m in &doc.materials {
            let body = if m.metalness > 0.5 { -m.metalness } else if m.transmission > 0.0 { 0.5 + 0.5 * m.transmission } else { 0.0 };
            if m.id == "default" {
                s.materials[0] = Material { name: m.name.clone(), finish: (1.0 - m.roughness).clamp(0.0, 1.0), body: body.clamp(-1.0, 1.0), ..Material::default() };
                continue;
            }
            ensure!(s.materials.len() < MAX_MATERIALS, "More than {MAX_MATERIALS} materials");
            material_row.insert(&m.id, s.materials.len() as u8);
            s.materials.push(Material { name: m.name.clone(), finish: (1.0 - m.roughness).clamp(0.0, 1.0), body: body.clamp(-1.0, 1.0), ..Material::default() });
        }
    }
    let lit = light.is_some() || has_maps;

    // Drawings and parts are shared by every view, by id.
    let mut drawing_of: HashMap<&str, u16> = HashMap::new();
    for v in &doc.views {
        for image in &v.images {
            if !drawing_of.contains_key(image.id.as_str()) {
                drawing_of.insert(&image.id, s.drawings.len() as u16);
                s.drawings.push(Drawing { id: image.id.clone(), name: image.name.clone(), places: vec![None; doc.views.len()] });
            }
        }
    }
    let mut part_of: HashMap<String, u16> = HashMap::new();
    for v in &doc.views {
        for p in &v.parts {
            if !part_of.contains_key(&p.id) {
                part_of.insert(p.id.clone(), s.parts.len() as u16);
                s.parts.push(Part { id: p.id.clone(), name: p.name.clone(), extra: false, depth: light.and_then(|l| l.depth.get(&p.id).copied()).unwrap_or(0.0) });
            }
        }
    }
    let taken: HashSet<String> = part_of.keys().cloned().collect();
    // A part whose drawings carry overlays gets a second part to show them.
    let overlay_part = |s: &mut Sprite, part_of: &mut HashMap<String, u16>, base: &old::Part| -> u16 {
        let mut id = format!("{}-overlay", base.id);
        while taken.contains(&id) {
            id.push('~');
        }
        *part_of.entry(id.clone()).or_insert_with(|| {
            s.parts.push(Part { id, name: format!("{} overlay", base.name), extra: false, depth: light.and_then(|l| l.depth.get(&base.id).copied()).unwrap_or(0.0) });
            (s.parts.len() - 1) as u16
        })
    };

    let mut referenced: HashSet<&str> = HashSet::from(["sprite.json"]);
    // Per view: for each of its old parts, where the part (and its overlay part) sit in the view's list.
    let mut slots: Vec<Vec<(u16, Option<u16>)>> = Vec::new();
    for (vi, v) in doc.views.iter().enumerate() {
        // Pictures.
        let mut arts = Vec::new();
        for image in &v.images {
            referenced.insert(&image.color);
            let bytes = package.entries.get(&image.color).with_context(|| format!("Missing image {}", image.color))?;
            let art = decode(bytes, indexed).with_context(|| format!("Invalid PNG {}", image.color))?;
            let n = (art.width * art.height) as usize;
            let (mut number, mut opacity) = (vec![0u16; n], vec![0u8; n]);
            match &art.pixels {
                Pixels::Indexed(indices) => {
                    for (i, &k) in indices.iter().enumerate() {
                        if k != 0 && (k as usize) < lifted.len() {
                            number[i] = u16::from(k);
                            opacity[i] = lifted[k as usize].unwrap_or(255);
                        }
                    }
                }
                Pixels::Rgba(rgba) => {
                    for (i, p) in rgba.chunks_exact(4).enumerate() {
                        if p[3] != 0 {
                            number[i] = number_of(&mut s, [p[0], p[1], p[2]])?;
                            opacity[i] = p[3];
                        }
                    }
                }
            }
            // Lighting for this drawing: given from outside, or from the old maps.
            let mut pixels = None;
            if lit {
                let mut out = vec![0u8; n * 4];
                let given = light.and_then(|l| l.drawings.get(&(v.id.clone(), image.id.clone())));
                if let Some(given) = given {
                    ensure!(given.len() == n * 4, "Lighting for {}/{} is not the size of the drawing", v.id, image.id);
                    out.copy_from_slice(given);
                } else {
                    let map = |path: &Option<String>| -> Result<Option<Vec<u8>>> {
                        let Some(path) = path else { return Ok(None) };
                        let bytes = package.entries.get(path).with_context(|| format!("Missing image {path}"))?;
                        let image = image::load_from_memory(bytes)?.to_rgba8();
                        ensure!((image.width(), image.height()) == (art.width, art.height), "Material map {path} must be the size of its drawing");
                        Ok(Some(image.into_raw()))
                    };
                    let (normal, emission) = (map(&image.normal)?, map(&image.emission)?);
                    let row = material_row.get(image.material.as_str()).copied().unwrap_or(0);
                    for i in 0..n {
                        let facing = |c: usize| normal.as_ref().map_or(128, |m| ((f32::from(m[i * 4 + c]) / 255.0 * 2.0 - 1.0) * 127.0 + 128.0).round().clamp(1.0, 255.0) as u8);
                        let glow = emission.as_ref().map_or(0, |m| (u16::from(m[i * 4].max(m[i * 4 + 1]).max(m[i * 4 + 2])) * u16::from(m[i * 4 + 3]) / 255) as u8);
                        out[i * 4..i * 4 + 4].copy_from_slice(&[facing(0), facing(1), row, glow]);
                    }
                }
                for i in 0..n {
                    if opacity[i] == 0 {
                        out[i * 4..i * 4 + 4].fill(0);
                    }
                }
                pixels = Some(out);
            }
            for map in [&image.normal, &image.surface, &image.emission].into_iter().flatten() {
                referenced.insert(map);
            }
            arts.push(Art { width: art.width, height: art.height, number, opacity, light: pixels });
        }
        let sizes: Vec<(u32, u32)> = arts.iter().map(|a| (a.width, a.height)).collect();
        let (at, width, height) = pack(&sizes)?;
        let n = (width * height) as usize;
        let mut color = ColorSheet { width, height, number: vec![0; n], opacity: vec![0; n] };
        let mut sheet = lit.then(|| LightSheet { width, height, data: vec![0; n * 4] });
        for (i, art) in arts.iter().enumerate() {
            let (x, y) = at[i];
            for row in 0..art.height {
                let (from, to) = ((row * art.width) as usize, ((y + row) * width + x) as usize);
                let w = art.width as usize;
                color.number[to..to + w].copy_from_slice(&art.number[from..from + w]);
                color.opacity[to..to + w].copy_from_slice(&art.opacity[from..from + w]);
                if let (Some(sheet), Some(light)) = (&mut sheet, &art.light) {
                    sheet.data[to * 4..(to + w) * 4].copy_from_slice(&light[from * 4..(from + w) * 4]);
                }
            }
            let image = &v.images[i];
            s.drawings[drawing_of[image.id.as_str()] as usize].places[vi] =
                Some(Place { x: x as u16, y: y as u16, width: art.width as u16, height: art.height as u16, offset_x: image.x, offset_y: image.y });
        }

        // Parts, in this view's order; an overlay part follows the part it sits on.
        let overlay: HashMap<&str, Option<&str>> = v
            .images
            .iter()
            .map(|i| {
                ensure!(i.overlays.len() <= 1, "Drawing {} has more than one overlay", i.id);
                Ok((i.id.as_str(), i.overlays.first().map(String::as_str)))
            })
            .collect::<Result<_>>()?;
        let mut needs_overlay: HashSet<&str> = HashSet::new();
        for a in &v.animations {
            for t in &a.tracks {
                if t.image.iter().any(|k| k.value.as_deref().is_some_and(|d| overlay.get(d).copied().flatten().is_some())) {
                    needs_overlay.insert(&t.part);
                }
            }
        }
        let mut list: Vec<ViewPart> = Vec::new();
        let mut slot: Vec<(u16, Option<u16>)> = Vec::new();
        for p in &v.parts {
            let base = list.len() as u16;
            list.push(ViewPart { part: part_of[&p.id], parent: None });
            let over = needs_overlay.contains(p.id.as_str()).then(|| {
                let part = overlay_part(&mut s, &mut part_of, p);
                list.push(ViewPart { part, parent: Some(base) });
                base + 1
            });
            slot.push((base, over));
        }
        for (i, p) in v.parts.iter().enumerate() {
            if let Some(parent) = &p.parent {
                let j = v.parts.iter().position(|q| &q.id == parent).with_context(|| format!("Part {} is joined to a part that does not exist", p.id))?;
                list[slot[i].0 as usize].parent = Some(slot[j].0);
            }
        }
        s.views.push(View {
            id: v.id.clone(),
            name: v.name.clone(),
            x: v.canvas.x.floor() as i32,
            y: v.canvas.y.floor() as i32,
            width: v.canvas.width.ceil() as u32,
            height: v.canvas.height.ceil() as u32,
            parts: list,
            color,
            light: sheet,
        });
        slots.push(slot);
    }

    // Animations are shared by id; each view keeps its own length and tracks.
    let mut animation_of: BTreeMap<&str, usize> = BTreeMap::new();
    for (vi, v) in doc.views.iter().enumerate() {
        let overlay: HashMap<&str, Option<&str>> = v.images.iter().map(|i| (i.id.as_str(), i.overlays.first().map(String::as_str))).collect();
        for a in &v.animations {
            let what = format!("Animation {} in view {}", a.id, v.id);
            let at = *animation_of.entry(&a.id).or_insert_with(|| {
                s.animations.push(Animation { id: a.id.clone(), name: a.name.clone(), views: vec![None; doc.views.len()] });
                s.animations.len() - 1
            });
            let mut motion = Motion { length: whole(a.length, &what)?, looping: a.looping, tracks: Vec::new() };
            for t in &a.tracks {
                let pi = v.parts.iter().position(|p| p.id == t.part).with_context(|| format!("{what} moves a part that does not exist"))?;
                let (base, over) = slots[vi][pi];
                let mut track = Track { part: s.views[vi].parts[base as usize].part, ..Track::default() };
                // "Hidden" is no longer a property: a hidden part shows no drawing.
                let mut ticks: Vec<f64> = t.image.iter().map(|k| k.tick).chain(t.visible.iter().map(|k| k.tick)).collect();
                ticks.sort_by(f64::total_cmp);
                ticks.dedup();
                let mut shown: Vec<(u32, Option<&str>)> = Vec::new();
                if !t.image.is_empty() {
                    for tick in ticks {
                        let value = if held(&t.visible, tick, true) { held(&t.image, tick, None) } else { None };
                        let id = value.as_deref().map(|id| v.images.iter().find(|i| i.id == id).map(|i| i.id.as_str()).with_context(|| format!("{what} shows a drawing that does not exist"))).transpose()?;
                        if shown.last().is_none_or(|l| l.1 != id) {
                            shown.push((whole(tick, &what)?, id));
                        }
                    }
                }
                track.image = shown.iter().map(|&(tick, id)| (tick, id.map(|id| drawing_of[id]))).collect();
                let all = [&t.x, &t.y, &t.rotation, &t.scale_x, &t.scale_y, &t.z, &t.opacity, &t.emission];
                for (i, keys) in all.into_iter().enumerate() {
                    track.numbers[i] = numbers(keys, &what)?;
                }
                if let Some(over) = over {
                    // The overlay part rides on its part and shows whatever overlay the part's drawing has.
                    let mut top = Track { part: s.views[vi].parts[over as usize].part, ..Track::default() };
                    for &(tick, id) in &shown {
                        let value = id.and_then(|id| overlay[id]).map(|id| drawing_of[id]);
                        if top.image.last().is_none_or(|l| l.1 != value) {
                            top.image.push((tick, value));
                        }
                    }
                    for i in [Z, OPACITY, EMISSION] {
                        top.numbers[i] = track.numbers[i].clone();
                    }
                    motion.tracks.push(track);
                    motion.tracks.push(top);
                } else {
                    motion.tracks.push(track);
                }
            }
            s.animations[at].views[vi] = Some(motion);
        }
    }

    for variant in &doc.variants {
        let mut out = Variant { id: variant.id.clone(), name: variant.name.clone(), ..Variant::default() };
        if let Some(palette) = &variant.palette {
            out.palette = Some(doc.palettes.iter().position(|p| &p.id == palette).with_context(|| format!("Variant {} names a palette that does not exist", variant.id))? as u16);
        }
        for r in &variant.replacements {
            let Some(vi) = doc.views.iter().position(|v| v.id == r.view) else { continue };
            let find = |id: &str| doc.views[vi].images.iter().find(|i| i.id == id);
            let (Some(from), Some(to)) = (find(&r.image), find(&r.with)) else { bail!("Variant {} replaces a drawing that does not exist", variant.id) };
            out.replace.push(Replace { view: vi as u16, from: drawing_of[from.id.as_str()], to: drawing_of[to.id.as_str()] });
            if fix_overlays {
                if let (Some(a), Some(b)) = (from.overlays.first(), to.overlays.first()) {
                    let named = variant.replacements.iter().any(|x| x.view == r.view && &x.image == a);
                    if a != b && !named {
                        out.replace.push(Replace { view: vi as u16, from: drawing_of[a.as_str()], to: drawing_of[b.as_str()] });
                    }
                }
            }
        }
        s.variants.push(out);
    }

    // Whatever else the archive carried rides along.
    for (path, bytes) in &package.entries {
        if !referenced.contains(path.as_str()) {
            s.extras.push(Extra { path: path.clone(), bytes: bytes.clone() });
        }
    }
    if !doc.extra.is_empty() {
        s.extras.push(Extra { path: "legacy/fields.json".into(), bytes: serde_json::to_vec(&doc.extra)? });
    }
    s.validate()?;
    Ok(s)
}

/// Draw every tick of every animation, view and variant both ways and count
/// the pictures that differ. The older file's own picture is the yardstick.
pub fn verify(package: &Package) -> Result<(usize, usize)> {
    let new = from_legacy(package, None, false)?;
    let old = Reference::new(package)?;
    let (mut frames, mut wrong) = (0, 0);
    for (vi, view) in package.document.views.iter().enumerate() {
        for animation in &view.animations {
            let ai = new.animation(&animation.id).unwrap();
            let mut looks: Vec<Vec<&str>> = vec![vec![]];
            for variant in &package.document.variants {
                looks.push(vec![variant.id.as_str()]);
            }
            for look in &looks {
                let active: Vec<usize> = look.iter().map(|id| new.variant(id).unwrap()).collect();
                for tick in 0..animation.length as u32 {
                    let want = old.render(&view.id, &animation.id, f64::from(tick), look)?;
                    frames += 1;
                    if picture(&new, vi, ai, f64::from(tick), &active) != want {
                        wrong += 1;
                    }
                }
            }
        }
    }
    Ok((frames, wrong))
}
