//! For every version 0 `.spu` under the given folders: upgrade it, save it
//! as version 1, reopen it, and compare every tick of every animation in
//! every view and variant, pixel for pixel, with Smeargle's renderer.
//!
//! `corpus-check <folder>... [--write <folder>]` (`--write` keeps the
//! upgraded files, mirroring the first folder's layout).
use anyhow::{Context, Result, ensure};
use smeargle_core::sprite_lab::{RenderSettings, read_spu};
use sprite_lab::{format::Package, reference::Reference};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, atomic::{AtomicUsize, Ordering}},
};

fn find(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find(&path, out);
        } else if path.extension().is_some_and(|e| e == "spu") {
            out.push(path);
        }
    }
}

/// Version 0 could leave out the tracks of trailing parts that never move;
/// give every animation one track per part, as version 1 files open.
fn padded(mut doc: smeargle_core::sprite_lab::Document) -> smeargle_core::sprite_lab::Document {
    for view in &mut doc.views {
        let parts = view.parts.len();
        for animation in &mut view.animations {
            if animation.tracks.len() < parts {
                std::sync::Arc::make_mut(animation).tracks.resize_with(parts, Default::default);
            }
        }
    }
    doc
}

/// Frames compared, or the first difference.
fn check(path: &Path, write: Option<&Path>) -> Result<usize> {
    let bytes = std::fs::read(path)?;
    let old = padded(read_spu(&bytes).context("Smeargle cannot read it")?);
    let upgraded = Package::read(&bytes).context("upgrade")?;
    ensure!(upgraded.migrated_from.as_deref() == Some("spu 0"), "not a version 0 file");
    let saved = upgraded.write()?;
    let package = Package::read(&saved).context("reopen")?;
    ensure!(package.migrated_from.is_none(), "saved file is not version 1");
    for name in upgraded.entries.keys() {
        ensure!(
            name == "sprite.json" || package.entries.get(name) == upgraded.entries.get(name),
            "member {name} changed"
        );
    }
    // Smeargle opens the version 1 file as the same sprite, and what
    // Smeargle saves is valid version 1 that reopens as the same sprite.
    ensure!(padded(read_spu(&saved).context("Smeargle cannot read version 1")?) == old, "Smeargle reads version 1 differently");
    let resaved = smeargle_core::sprite_lab::write_spu(&old)?;
    ensure!(Package::read(&resaved).context("Smeargle's save is invalid")?.migrated_from.is_none(), "Smeargle saved an old version");
    ensure!(padded(read_spu(&resaved)?) == old, "Smeargle's save reopens differently");
    let package = Package::read(&resaved)?;
    let reference = Reference::new(&package)?;
    let doc = &package.document;
    ensure!(doc.variants.len() == old.variants.len(), "variant count");
    let mut frames = 0;
    for (v, view) in doc.views.iter().enumerate() {
        let bounds = old.views[v].canvas.bounds();
        for (a, animation) in view.animations.iter().enumerate() {
            let length = old.views[v].animations[a].length;
            // Each variant alone, none, and all together.
            let mut sets: Vec<Vec<usize>> = vec![vec![]];
            sets.extend((0..doc.variants.len()).map(|i| vec![i]));
            if doc.variants.len() > 1 {
                sets.push((0..doc.variants.len()).collect());
            }
            for set in sets {
                let ids: Vec<&str> = set.iter().map(|&i| doc.variants[i].id.as_str()).collect();
                // Past the end too: looping wraps, the rest hold.
                for tick in (0..length).chain([length, length + 7]) {
                    let want = old.render(v, a, tick, bounds, RenderSettings { variants: &set, ..Default::default() })?;
                    let got = reference.render(&view.id, &animation.id, f64::from(tick), &ids)?;
                    ensure!(
                        (got.width, got.height) == (want.width, want.height) && got.pixels == want.pixels,
                        "view {} animation {} tick {tick} variants {ids:?} differs",
                        view.id, animation.id
                    );
                    frames += 1;
                }
            }
        }
    }
    if let Some(out) = write {
        std::fs::create_dir_all(out.parent().unwrap())?;
        std::fs::write(out, saved)?;
    }
    Ok(frames)
}

fn main() -> Result<()> {
    let mut roots = Vec::new();
    let mut write = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--write" {
            write = Some(PathBuf::from(args.next().context("--write needs a folder")?));
        } else {
            roots.push(PathBuf::from(arg));
        }
    }
    ensure!(!roots.is_empty(), "corpus-check <folder>... [--write <folder>]");
    let mut files = Vec::new();
    for root in &roots {
        let mut found = Vec::new();
        find(root, &mut found);
        found.sort();
        files.extend(found.into_iter().map(|f| (root.clone(), f)));
    }
    let (next, frames) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..std::thread::available_parallelism().map_or(4, |n| n.get()) {
            scope.spawn(|| loop {
                let Some((root, file)) = files.get(next.fetch_add(1, Ordering::Relaxed)) else { break };
                let out = write.as_ref().map(|w| {
                    w.join(root.file_name().unwrap()).join(file.strip_prefix(root).unwrap())
                });
                match check(file, out.as_deref()) {
                    Ok(n) => {
                        frames.fetch_add(n, Ordering::Relaxed);
                    }
                    Err(e) => failures.lock().unwrap().push(format!("{}: {e:#}", file.display())),
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    for f in &failures {
        println!("FAIL {f}");
    }
    println!(
        "{} files, {} identical, {} frames compared, {} failed",
        files.len(),
        files.len() - failures.len(),
        frames.into_inner(),
        failures.len()
    );
    ensure!(failures.is_empty(), "differences found");
    Ok(())
}
