//! Command-line tool: inspect, check and draw `.spu` files.
use anyhow::{Context, Result, bail};
use spu::{
    binary,
    picture::{Frame, picture},
    sprite::Sprite,
};

const USAGE: &str = "\
spu info <file>
spu validate <file>...
spu render <file> <out.png> [--view ID] [--animation ID] [--tick N] [--variant ID]...
spu animate <file> <out.png> [--view ID] [--animation ID] [--variant ID]... [--step N]
        a looping animated PNG, drawn every N ticks";

fn open(path: &str) -> Result<Sprite> {
    binary::read(&std::fs::read(path).with_context(|| format!("Cannot read {path}"))?).with_context(|| path.to_owned())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    match (args.first().map(String::as_str), rest.as_slice()) {
        (Some("info"), [path]) => {
            let s = open(path)?;
            println!("{}", s.name);
            if !s.author.is_empty() {
                println!("by {}", s.author);
            }
            println!("version {}, {} ticks per second", s.version, s.ticks_per_second);
            println!("{} parts, {} drawings, {} palettes of {} colors, {} materials", s.parts.len(), s.drawings.len(), s.palettes.len(), s.palettes[0].colors.len(), s.materials.len());
            for v in &s.variants {
                println!("variant {} ({})", v.id, v.name);
            }
            for (i, v) in s.views.iter().enumerate() {
                let drawings = s.drawings.iter().filter(|d| d.places[i].is_some()).count();
                println!("view {} ({}): {} parts, {} drawings, sheet {}x{}{}", v.id, v.name, v.parts.len(), drawings, v.color.width, v.color.height, if v.light.is_some() { ", lit" } else { "" });
                for a in &s.animations {
                    if let Some(m) = &a.views[i] {
                        println!("  animation {} ({}): {} ticks{}", a.id, a.name, m.length, if m.looping { ", loops" } else { "" });
                    }
                }
            }
            if !s.extras.is_empty() {
                println!("{} extras, {} bytes", s.extras.len(), s.extras.iter().map(|e| e.bytes.len()).sum::<usize>());
            }
        }
        (Some("validate"), paths) if !paths.is_empty() => {
            let mut failed = 0;
            for path in paths {
                match open(path) {
                    Ok(_) => println!("ok       {path}"),
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
        (Some(mode @ ("render" | "animate")), [path, output, options @ ..]) => {
            let s = open(path)?;
            let (mut view, mut animation, mut tick, mut variants) = (0, None, 0.0, Vec::new());
            let mut step = 1u16;
            for pair in options.chunks(2) {
                match pair {
                    ["--view", v] => view = s.view(v).with_context(|| format!("No view {v}"))?,
                    ["--animation", v] => animation = Some(s.animation(v).with_context(|| format!("No animation {v}"))?),
                    ["--tick", v] => tick = v.parse()?,
                    ["--step", v] => step = v.parse::<u16>()?.max(1),
                    ["--variant", v] => variants.push(s.variant(v).with_context(|| format!("No variant {v}"))?),
                    _ => bail!("{USAGE}"),
                }
            }
            // By default, the first animation this view has.
            let animation = match animation {
                Some(a) => a,
                None => s.animations.iter().position(|a| a.views[view].is_some()).context("This view has no animation")?,
            };
            if mode == "render" {
                std::fs::write(output, picture(&s, view, animation, tick, &variants).png()?)?;
                return Ok(());
            }
            let motion = s.animations[animation].views[view].as_ref().context("This view does not have that animation")?;
            // One frame per run of identical ticks.
            let mut frames: Vec<(Frame, u16)> = Vec::new();
            let length = motion.length.clamp(1, 6000);
            for tick in (0..length).step_by(step.into()) {
                let shown = (length - tick).min(step.into()) as u16;
                let frame = picture(&s, view, animation, f64::from(tick), &variants);
                match frames.last_mut() {
                    Some((last, ticks)) if *last == frame && *ticks < u16::MAX - shown => *ticks += shown,
                    _ => frames.push((frame, shown)),
                }
            }
            let rate = s.ticks_per_second.round().clamp(1.0, 1000.0) as u16;
            std::fs::write(output, Frame::apng(&frames, rate)?)?;
        }
        _ => bail!("{USAGE}"),
    }
    Ok(())
}
