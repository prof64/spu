//! Upgrading older files to the current version when they are opened.
//!
//! Two older shapes exist: Smeargle's `.spu` version 0 (indexed drawings,
//! entities referenced by position) and this repository's `.sprite`
//! prototype 0. Both become version 1 in memory; saving writes version 1.
use super::format::{FORMAT, VERSION};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashSet};

/// The manifest as version 1, and the name of what it was upgraded from.
pub fn upgrade(
    manifest: Value,
    entries: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(Value, Option<String>)> {
    let format = manifest["format"].as_str().unwrap_or_default().to_owned();
    let version = manifest["version"].as_u64();
    match (format.as_str(), version) {
        (FORMAT, Some(v)) if v == u64::from(VERSION) => Ok((manifest, None)),
        ("spu", Some(0)) => Ok((from_spu0(manifest, entries)?, Some("spu 0".into()))),
        ("sprite-lab", Some(0)) => Ok((from_prototype0(manifest)?, Some("sprite-lab 0".into()))),
        _ => bail!(
            "Unsupported format/version ({format:?} {version:?}); this reads SPU version {VERSION}"
        ),
    }
}

fn from_prototype0(mut manifest: Value) -> Result<Value> {
    let doc = manifest.as_object_mut().context("Manifest must be an object")?;
    doc.insert("format".into(), FORMAT.into());
    doc.insert("version".into(), VERSION.into());
    if let Some(required) = doc.get_mut("required").and_then(Value::as_array_mut) {
        required.retain(|c| c != "core-0");
    }
    Ok(manifest)
}

/// Readable ids from names: `Leg FR` becomes `leg-fr`, repeats get `-2`.
struct Ids(HashSet<String>);
impl Ids {
    fn new() -> Self {
        Self(HashSet::new())
    }
    fn make(&mut self, name: &str, fallback: &str) -> String {
        let mut slug = String::new();
        for c in name.chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
            } else if !slug.ends_with('-') && !slug.is_empty() {
                slug.push('-');
            }
        }
        let slug = slug.trim_end_matches('-');
        let base = if slug.is_empty() { fallback } else { slug };
        let mut id = base.to_owned();
        let mut n = 2;
        while !self.0.insert(id.clone()) {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map_or(&[], Vec::as_slice)
}
fn name_of(value: &Value) -> &str {
    value["name"].as_str().unwrap_or_default()
}
fn index(value: &Value, ids: &[String], what: &str) -> Result<String> {
    let i = value.as_u64().with_context(|| format!("{what} must be an index"))? as usize;
    ids.get(i)
        .cloned()
        .with_context(|| format!("{what} {i} does not exist"))
}

fn from_spu0(manifest: Value, entries: &BTreeMap<String, Vec<u8>>) -> Result<Value> {
    let Value::Object(mut doc) = manifest.clone() else {
        bail!("Manifest must be an object")
    };
    let mut ids = Ids::new();
    let palette_ids: Vec<String> = list(&manifest, "palettes")
        .iter()
        .map(|p| ids.make(name_of(p), "palette"))
        .collect();
    let palettes: Vec<Value> = list(&manifest, "palettes")
        .iter()
        .zip(&palette_ids)
        .map(|(p, id)| json!({"id": id, "name": name_of(p), "colors": p["colors"]}))
        .collect();

    let mut ids = Ids::new();
    let mut views = Vec::new();
    let mut view_ids = Vec::new();
    let mut image_ids: Vec<Vec<String>> = Vec::new();
    // (view, open drawing, closed drawing) for every pair of eye overlays.
    let mut blinks = Vec::new();
    for (v, view) in list(&manifest, "views").iter().enumerate() {
        let view_id = ids.make(name_of(view), "view");
        let mut local = Ids::new();
        let drawing_ids: Vec<String> = list(view, "images")
            .iter()
            .map(|i| local.make(name_of(i), "drawing"))
            .collect();
        let mut images = Vec::new();
        let mut overlays = Vec::new();
        for (n, (image, id)) in list(view, "images").iter().zip(&drawing_ids).enumerate() {
            let path = format!("images/{v}/{n}.png");
            ensure!(entries.contains_key(&path), "Missing {path}");
            let mut drawing = json!({
                "id": id, "name": name_of(image),
                "x": image["x"], "y": image["y"], "color": path,
            });
            if let Some(eyes) = image.get("eyes").filter(|e| e.is_object()) {
                let mut pair = Vec::new();
                for state in ["open", "closed"] {
                    let path = format!("images/{v}/{n}.eyes-{state}.png");
                    ensure!(entries.contains_key(&path), "Missing {path}");
                    let eye_id = local.make(&format!("{id}-eyes-{state}"), "eyes");
                    overlays.push(json!({
                        "id": eye_id, "name": format!("{} (eyes {state})", name_of(image)),
                        "x": eyes["x"], "y": eyes["y"], "color": path,
                    }));
                    pair.push(eye_id);
                }
                drawing["overlays"] = json!([pair[0]]);
                blinks.push(json!({"view": view_id, "image": pair[0], "with": pair[1]}));
            }
            images.push(drawing);
        }
        images.extend(overlays);

        let mut local = Ids::new();
        let part_ids: Vec<String> = list(view, "parts")
            .iter()
            .map(|p| local.make(name_of(p), "part"))
            .collect();
        let mut parts = Vec::new();
        for (part, id) in list(view, "parts").iter().zip(&part_ids) {
            let mut out = json!({"id": id, "name": name_of(part)});
            if !part["parent"].is_null() {
                out["parent"] = index(&part["parent"], &part_ids, "Parent part")?.into();
            }
            parts.push(out);
        }

        let mut local = Ids::new();
        let mut animations = Vec::new();
        for animation in list(view, "animations") {
            let mut tracks = Vec::new();
            for (track, part) in list(animation, "tracks").iter().zip(&part_ids) {
                let Some(channels) = track.as_object() else {
                    continue;
                };
                let mut out = Map::new();
                for (channel, keys) in channels {
                    let mut keys = keys.clone();
                    if channel == "image" {
                        for key in keys.as_array_mut().into_iter().flatten() {
                            if !key["value"].is_null() {
                                key["value"] = index(&key["value"], &drawing_ids, "Drawing")?.into();
                            }
                        }
                    }
                    if keys.as_array().is_some_and(|k| !k.is_empty()) {
                        out.insert(channel.clone(), keys);
                    }
                }
                if !out.is_empty() {
                    out.insert("part".into(), part.as_str().into());
                    tracks.push(Value::Object(out));
                }
            }
            animations.push(json!({
                "id": local.make(name_of(animation), "animation"),
                "name": name_of(animation),
                "length": animation["length"].as_u64().unwrap_or(0).max(1),
                "looping": animation["looping"].as_bool().unwrap_or(true),
                "tracks": tracks,
            }));
        }
        if animations.is_empty() {
            animations.push(json!({"id": "still", "name": "Still", "length": 1, "looping": true, "tracks": []}));
        }
        views.push(json!({
            "id": view_id, "name": name_of(view), "canvas": view["canvas"],
            "images": images, "parts": parts, "animations": animations,
        }));
        view_ids.push(view_id);
        image_ids.push(drawing_ids);
    }

    let mut ids = Ids::new();
    let mut variants = Vec::new();
    for variant in list(&manifest, "variants") {
        let mut out = json!({"id": ids.make(name_of(variant), "variant"), "name": name_of(variant)});
        if !variant["palette"].is_null() {
            out["palette"] = index(&variant["palette"], &palette_ids, "Palette")?.into();
        }
        let mut replacements = Vec::new();
        for r in list(variant, "replacements") {
            let v = r["view"].as_u64().context("Replacement view")? as usize;
            let drawings = image_ids.get(v).context("Replacement view does not exist")?;
            replacements.push(json!({
                "view": view_ids[v],
                "image": index(&r["image"], drawings, "Drawing")?,
                "with": index(&r["with"], drawings, "Drawing")?,
            }));
        }
        if !replacements.is_empty() {
            out["replacements"] = replacements.into();
        }
        variants.push(out);
    }
    if !blinks.is_empty() {
        variants.push(json!({
            "id": ids.make("eyes-closed", "variant"), "name": "Eyes closed", "replacements": blinks,
        }));
    }

    doc.insert("format".into(), FORMAT.into());
    doc.insert("version".into(), VERSION.into());
    doc.insert("palettes".into(), palettes.into());
    doc.insert("variants".into(), variants.into());
    doc.insert("views".into(), views.into());
    Ok(Value::Object(doc))
}
