//! Command-line tool: inspect, check, upgrade and draw `.spu` files.
use anyhow::{Context, Result, bail};
use sprite_lab::{format::Package, reference::Reference};

const USAGE: &str = "\
spu info <file>
spu validate <file>...
spu convert <in> <out.spu>          upgrade an older file to the current version
spu render <file> <out.png> [--view ID] [--animation ID] [--tick N] [--variant ID]...
spu animate <file> <out.png> [--view ID] [--animation ID] [--variant ID]... [--step N]   a looping animated PNG, drawn every N ticks";

fn open(path: &str) -> Result<Package> {
    Package::read(&std::fs::read(path).with_context(|| format!("Cannot read {path}"))?)
        .with_context(|| path.to_owned())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    match (args.first().map(String::as_str), rest.as_slice()) {
        (Some("info"), [path]) => {
            let p = open(path)?;
            let d = &p.document;
            println!("{} (SPU {}{})", d.name, d.version, p.migrated_from.as_ref().map_or(String::new(), |m| format!(", upgraded from {m}")));
            println!("{} ticks per second, {} palettes, {} materials", d.ticks_per_second, d.palettes.len(), d.materials.len());
            for v in &d.variants {
                println!("variant {} ({})", v.id, v.name);
            }
            for v in &d.views {
                println!("view {} ({}): {} parts, {} drawings", v.id, v.name, v.parts.len(), v.images.len());
                for a in &v.animations {
                    println!("  animation {} ({}): {} ticks{}", a.id, a.name, a.length, if a.looping { ", loops" } else { "" });
                }
            }
        }
        (Some("validate"), paths) if !paths.is_empty() => {
            let mut failed = 0;
            for path in paths {
                match open(path).and_then(|p| Reference::new(&p).map(|_| ())) {
                    Ok(()) => println!("ok       {path}"),
                    Err(e) => {
                        failed += 1;
                        println!("INVALID  {path}: {e:#}");
                    }
                }
            }
            if failed > 0 {
                bail!("{failed} invalid");
            }
        }
        (Some("convert"), [input, output]) => {
            let p = open(input)?;
            Reference::new(&p)?;
            std::fs::write(output, p.write()?)?;
        }
        (Some(mode @ ("render" | "animate")), [path, output, options @ ..]) => {
            let animate = mode == "animate";
            let p = open(path)?;
            let mut view = p.document.views[0].id.clone();
            let (mut animation, mut tick, mut variants) = (None, 0.0, Vec::new());
            let mut step = 1u16;
            for pair in options.chunks(2) {
                match pair {
                    ["--view", v] => view = (*v).to_owned(),
                    ["--animation", v] => animation = Some((*v).to_owned()),
                    ["--tick", v] => tick = v.parse()?,
                    ["--step", v] => step = v.parse::<u16>()?.max(1),
                    ["--variant", v] => variants.push(*v),
                    _ => bail!("{USAGE}"),
                }
            }
            let animation = match animation {
                Some(a) => a,
                None => {
                    let v = p.document.views.iter().find(|v| v.id == view).context("No such view")?;
                    v.animations[0].id.clone()
                }
            };
            let reference = Reference::new(&p)?;
            if !animate {
                let frame = reference.render(&view, &animation, tick, &variants)?;
                std::fs::write(output, frame.png()?)?;
                return Ok(());
            }
            let v = p.document.views.iter().find(|v| v.id == view).context("No such view")?;
            let length = v.animations.iter().find(|a| a.id == animation).context("No such animation")?.length;
            // One frame per run of identical ticks.
            let mut frames: Vec<(sprite_lab::reference::Frame, u16)> = Vec::new();
            let length = (length.ceil() as u32).clamp(1, 6000);
            for tick in (0..length).step_by(step.into()) {
                let shown = (length - tick).min(step.into()) as u16;
                let frame = reference.render(&view, &animation, f64::from(tick), &variants)?;
                match frames.last_mut() {
                    Some((last, ticks)) if *last == frame && *ticks < u16::MAX - shown => *ticks += shown,
                    _ => frames.push((frame, shown)),
                }
            }
            let rate = p.document.ticks_per_second.round().clamp(1.0, 1000.0) as u16;
            std::fs::write(output, sprite_lab::reference::Frame::apng(&frames, rate)?)?;
        }
        _ => bail!("{USAGE}"),
    }
    Ok(())
}
