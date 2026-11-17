//! The reference picture: what a file looks like with no lighting.
//!
//! This software renderer defines the exact pixels of a sprite at a tick.
//! Every other renderer (the GPU viewer, an editor, a game) is checked
//! against it, and it draws version 0 files exactly as Smeargle drew them.
use super::{
    format::{Document, Package, View, parse_color},
    timing::{Trs, hold, sample64},
};
use anyhow::{Context, Result, ensure};
use std::{collections::HashMap, io::Cursor};

/// Straight (not premultiplied) 8-bit RGBA, row by row from the top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
impl Frame {
    pub fn png(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        image::write_buffer_with_format(
            &mut Cursor::new(&mut out),
            &self.pixels,
            self.width,
            self.height,
            image::ColorType::Rgba8,
            image::ImageOutputFormat::Png,
        )?;
        Ok(out)
    }
    /// An animated PNG that loops forever. Each frame is shown for
    /// `ticks / ticks_per_second` seconds.
    pub fn apng(frames: &[(Frame, u16)], ticks_per_second: u16) -> Result<Vec<u8>> {
        let first = &frames.first().context("No frames")?.0;
        let mut out = Vec::new();
        // Pixel art rarely needs more than 256 colors: store indices then.
        let mut colors: Vec<[u8; 4]> = Vec::new();
        let mut lookup = HashMap::new();
        let mut indexed = Some(Vec::new());
        'frames: for (frame, _) in frames {
            let mut data = Vec::with_capacity(frame.pixels.len() / 4);
            for p in frame.pixels.chunks_exact(4) {
                let color = if p[3] == 0 { [0; 4] } else { [p[0], p[1], p[2], p[3]] };
                let next = colors.len();
                let i = *lookup.entry(color).or_insert(next);
                if i == next {
                    if next == 256 {
                        indexed = None;
                        break 'frames;
                    }
                    colors.push(color);
                }
                data.push(i as u8);
            }
            indexed.as_mut().unwrap().push(data);
        }
        let mut encoder = png::Encoder::new(&mut out, first.width, first.height);
        encoder.set_depth(png::BitDepth::Eight);
        if indexed.is_some() {
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_palette(colors.iter().flat_map(|c| [c[0], c[1], c[2]]).collect::<Vec<_>>());
            encoder.set_trns(colors.iter().map(|c| c[3]).collect::<Vec<_>>());
        } else {
            encoder.set_color(png::ColorType::Rgba);
        }
        encoder.set_compression(png::Compression::Best);
        encoder.set_animated(frames.len() as u32, 0)?;
        let mut writer = encoder.write_header()?;
        for (i, (frame, ticks)) in frames.iter().enumerate() {
            writer.set_frame_delay(*ticks, ticks_per_second)?;
            match &indexed {
                Some(data) => writer.write_image_data(&data[i])?,
                None => writer.write_image_data(&frame.pixels)?,
            }
        }
        writer.finish()?;
        Ok(out)
    }
    pub fn from_png(bytes: &[u8]) -> Result<Self> {
        let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
        Ok(Self {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }
}

pub(crate) enum Pixels {
    /// Palette indices; 0 is transparent.
    Indexed(Vec<u8>),
    Rgba(Vec<u8>),
}
pub(crate) struct Art {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Pixels,
}

/// Largest drawing side, in pixels.
pub const MAX_DRAWING: u32 = 4096;

pub(crate) fn decode(bytes: &[u8], indexed: bool) -> Result<Art> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let (width, height, depth) = (info.width, info.height, info.bit_depth as u8);
    ensure!(
        width > 0 && height > 0 && width <= MAX_DRAWING && height <= MAX_DRAWING,
        "A drawing is 1–{MAX_DRAWING} pixels on each side"
    );
    if indexed && info.color_type == png::ColorType::Indexed {
        let mut packed = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut packed)?;
        let stride = frame.line_size;
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for row in packed.chunks(stride).take(height as usize) {
            for x in 0..width as usize {
                let bit = x * depth as usize;
                let shift = 8 - depth as usize - bit % 8;
                pixels.push((row[bit / 8] >> shift) & ((1u16 << depth) - 1) as u8);
            }
        }
        return Ok(Art {
            width,
            height,
            pixels: Pixels::Indexed(pixels),
        });
    }
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.to_rgba8();
    Ok(Art {
        width,
        height,
        pixels: Pixels::Rgba(image.into_raw()),
    })
}

/// A file prepared for drawing.
pub struct Reference<'a> {
    document: &'a Document,
    /// Decoded color image of each drawing, per view, in file order.
    art: Vec<Vec<Art>>,
    palettes: Vec<Vec<[u8; 4]>>,
}

impl<'a> Reference<'a> {
    pub fn new(package: &'a Package) -> Result<Self> {
        let document = &package.document;
        let indexed = !document.palettes.is_empty();
        let mut art = Vec::new();
        for view in &document.views {
            let mut drawings = Vec::new();
            for drawing in &view.images {
                let bytes = package
                    .entries
                    .get(&drawing.color)
                    .with_context(|| format!("Missing image {}", drawing.color))?;
                let art = decode(bytes, indexed)
                    .with_context(|| format!("Invalid PNG {}", drawing.color))?;
                // Material maps never change this picture, but a file whose
                // maps are missing or a different size is still broken.
                for map in [&drawing.normal, &drawing.surface, &drawing.emission]
                    .into_iter()
                    .flatten()
                {
                    let bytes = package
                        .entries
                        .get(map)
                        .with_context(|| format!("Missing image {map}"))?;
                    let size = decode(bytes, false).with_context(|| format!("Invalid PNG {map}"))?;
                    ensure!(
                        (size.width, size.height) == (art.width, art.height),
                        "Material map {map} must be the size of its drawing"
                    );
                }
                drawings.push(art);
            }
            art.push(drawings);
        }
        let palettes = document
            .palettes
            .iter()
            .map(|p| {
                let mut colors: Vec<[u8; 4]> =
                    p.colors.iter().map(|c| parse_color(c).unwrap()).collect();
                colors[0][3] = 0;
                colors
            })
            .collect();
        Ok(Self {
            document,
            art,
            palettes,
        })
    }

    /// One tick of an animation over the view's canvas. `variants` are the
    /// active variant ids, applied in order.
    pub fn render(&self, view: &str, animation: &str, tick: f64, variants: &[&str]) -> Result<Frame> {
        let doc = self.document;
        let v = doc
            .views
            .iter()
            .position(|x| x.id == view)
            .with_context(|| format!("No view {view}"))?;
        let view = &doc.views[v];
        let clip = view
            .animations
            .iter()
            .find(|a| a.id == animation)
            .with_context(|| format!("No animation {animation}"))?;
        let mut active = Vec::new();
        for id in variants {
            active.push(
                doc.variants
                    .iter()
                    .find(|x| x.id == *id)
                    .with_context(|| format!("No variant {id}"))?,
            );
        }
        // The last active variant that names a palette decides the colors.
        let palette = active
            .iter()
            .rev()
            .find_map(|x| x.palette.as_ref())
            .and_then(|id| doc.palettes.iter().position(|p| &p.id == id))
            .unwrap_or(0);
        let colors: &[[u8; 4]] = self.palettes.get(palette).map_or(&[], Vec::as_slice);
        let resolve = |image: &str| -> usize {
            let mut drawn = image;
            for variant in &active {
                for r in &variant.replacements {
                    if r.view == view.id && r.image == image {
                        drawn = &r.with;
                    }
                }
            }
            drawing_index(view, drawn)
        };

        let tick = if clip.looping {
            tick.rem_euclid(clip.length)
        } else {
            tick.min(clip.length - 1.0).max(0.0)
        };
        let part_index: HashMap<&str, usize> = view
            .parts
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.as_str(), i))
            .collect();
        let mut tracks = vec![None; view.parts.len()];
        for track in &clip.tracks {
            tracks[part_index[track.part.as_str()]] = Some(track);
        }
        let locals: Vec<Trs> = tracks
            .iter()
            .map(|t| t.map_or_else(Trs::default, |t| Trs::of(t, tick)))
            .collect();
        // (z, part, drawing, opacity, placement), back to front.
        let mut list = Vec::new();
        for (p, track) in tracks.iter().enumerate() {
            let Some(track) = track else { continue };
            if !hold(&track.visible, tick, true) {
                continue;
            }
            let Some(image) = hold(&track.image, tick, None) else {
                continue;
            };
            // Top-down chain of parents; validation guarantees no cycles.
            let mut chain = vec![p];
            while let Some(parent) = &view.parts[*chain.last().unwrap()].parent {
                chain.push(part_index[parent.as_str()]);
            }
            let mut world = locals[*chain.last().unwrap()];
            for &c in chain.iter().rev().skip(1) {
                world = world.then(&locals[c]);
            }
            list.push((
                sample64(&track.z, tick, 0.0),
                p,
                image,
                sample64(&track.opacity, tick, 1.0).clamp(0.0, 1.0) as f32,
                world,
            ));
        }
        list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));

        let canvas = view.canvas;
        let (min_x, min_y) = (canvas.x.floor() as i64, canvas.y.floor() as i64);
        let mut frame = Frame {
            width: canvas.width.ceil() as u32,
            height: canvas.height.ceil() as u32,
            pixels: Vec::new(),
        };
        frame.pixels = vec![0; (frame.width * frame.height * 4) as usize];
        for (_, _, image, opacity, world) in list {
            let base = drawing_index(view, &image);
            let over = view.images[base].overlays.iter().map(|o| resolve(o));
            for d in std::iter::once(resolve(&image)).chain(over) {
                let drawing = &view.images[d];
                paint(
                    &mut frame,
                    (min_x, min_y),
                    (f64::from(drawing.x), f64::from(drawing.y)),
                    &self.art[v][d],
                    &world.matrix(),
                    colors,
                    opacity,
                );
            }
        }
        Ok(frame)
    }
}

fn drawing_index(view: &View, id: &str) -> usize {
    view.images
        .iter()
        .position(|i| i.id == id)
        .expect("validated drawing reference")
}

/// The texel a local coordinate falls in. A sample within 1e-9 of a texel
/// edge counts as on it, so equivalent placements pick the same texel.
pub(crate) fn texel(local: f64) -> f64 {
    let edge = local.round();
    if (local - edge).abs() < 1e-9 {
        edge
    } else {
        local.floor()
    }
}

/// Nearest-neighbor at pixel centers, then "over" compositing.
fn paint(
    out: &mut Frame,
    origin: (i64, i64),
    at: (f64, f64),
    art: &Art,
    m: &[f64; 6],
    colors: &[[u8; 4]],
    opacity: f32,
) {
    let det = m[0] * m[3] - m[1] * m[2];
    if opacity <= 0.0 || det.abs() < 1e-12 {
        return;
    }
    let (a, b, c, d) = (m[3] / det, -m[1] / det, -m[2] / det, m[0] / det);
    let (tx, ty) = (-(a * m[4] + c * m[5]), -(b * m[4] + d * m[5]));
    let (w, h) = (f64::from(art.width), f64::from(art.height));
    let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for (x, y) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
        let (x, y) = (at.0 + x, at.1 + y);
        let (sx, sy) = (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
        x0 = x0.min(sx.floor() as i64);
        y0 = y0.min(sy.floor() as i64);
        x1 = x1.max(sx.ceil() as i64);
        y1 = y1.max(sy.ceil() as i64);
    }
    let (max_x, max_y) = (origin.0 + i64::from(out.width), origin.1 + i64::from(out.height));
    for sy in y0.max(origin.1)..y1.min(max_y) {
        for sx in x0.max(origin.0)..x1.min(max_x) {
            let (px, py) = (sx as f64 + 0.5, sy as f64 + 0.5);
            let (lx, ly) = (a * px + c * py + tx, b * px + d * py + ty);
            let (u, v) = (texel(lx - at.0), texel(ly - at.1));
            if !(u >= 0.0 && v >= 0.0 && u < w && v < h) {
                continue;
            }
            let i = v as usize * art.width as usize + u as usize;
            let color = match &art.pixels {
                Pixels::Indexed(indices) => match indices[i] {
                    0 => continue,
                    n => match colors.get(n as usize) {
                        Some(&color) => color,
                        None => continue,
                    },
                },
                Pixels::Rgba(rgba) => [rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2], rgba[i * 4 + 3]],
            };
            let o = (((sy - origin.1) as u32 * out.width + (sx - origin.0) as u32) * 4) as usize;
            blend(&mut out.pixels[o..o + 4], color, opacity);
        }
    }
}

pub(crate) fn blend(dst: &mut [u8], color: [u8; 4], opacity: f32) {
    let a = opacity * f32::from(color[3]) / 255.0;
    let da = f32::from(dst[3]) / 255.0;
    let oa = a + da * (1.0 - a);
    if oa <= 0.0 {
        return;
    }
    for c in 0..3 {
        let v = (f32::from(color[c]) * a + f32::from(dst[c]) * da * (1.0 - a)) / oa;
        dst[c] = v.round() as u8;
    }
    dst[3] = (oa * 255.0).round() as u8;
}
