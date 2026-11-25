//! The reference picture: exactly what a sprite looks like at a tick, with
//! no lighting. Every renderer can be checked against it pixel for pixel.
//! Materials, facing and glow never change this picture.
use crate::sprite::*;
use anyhow::{Context, Result};
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

/// The value of a number at `tick`: the key at or before it, eased toward
/// the next. Before the first key, the first key's value holds.
pub fn number(keys: &[Key], tick: f64, default: f64) -> f64 {
    let i = keys.partition_point(|k| k.tick as f64 <= tick);
    if i == 0 {
        return keys.first().map_or(default, |k| k.value);
    }
    let a = &keys[i - 1];
    match keys.get(i) {
        Some(b) if a.ease != Ease::Step => a.value + (b.value - a.value) * a.ease.apply((tick - a.tick as f64) / (b.tick as f64 - a.tick as f64)),
        _ => a.value,
    }
}

/// The drawing a part shows at `tick`: the key at or before it. Before the
/// first key, the first key's value holds. With no keys, none.
pub fn image(keys: &[(u32, Option<u16>)], tick: f64) -> Option<u16> {
    let i = keys.partition_point(|k| k.0 as f64 <= tick);
    keys.get(i.saturating_sub(1)).and_then(|k| k.1)
}

/// The tick an animation shows at playback tick `tick`.
pub fn wrap(motion: &Motion, tick: f64) -> f64 {
    let length = motion.length as f64;
    if motion.looping { tick.rem_euclid(length) } else { tick.min(length - 1.0).max(0.0) }
}

/// One drawing placed on the canvas.
pub struct Placed {
    /// Position in the view's list of parts.
    pub part: usize,
    pub drawing: u16,
    pub place: Place,
    pub opacity: f32,
    pub emission: f32,
    /// `[a, b, c, d, tx, ty]`: `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`.
    pub matrix: [f64; 6],
}

/// Everything a view draws at a tick, back to front. `variants` are the
/// active variants, applied in order.
pub fn placed(s: &Sprite, view: usize, animation: usize, tick: f64, variants: &[usize]) -> Vec<Placed> {
    let v = &s.views[view];
    let Some(motion) = &s.animations[animation].views[view] else { return Vec::new() };
    let tick = wrap(motion, tick);
    let active: Vec<&Variant> = variants.iter().map(|&i| &s.variants[i]).collect();

    let mut tracks: Vec<Option<&Track>> = vec![None; v.parts.len()];
    for t in &motion.tracks {
        if let Some(i) = v.parts.iter().position(|p| p.part == t.part) {
            tracks[i] = Some(t);
        }
    }
    let local = |i: usize| -> Trs {
        let get = |property: usize| tracks[i].map_or(PROPERTIES[property].1, |t| number(&t.numbers[property], tick, PROPERTIES[property].1));
        Trs { x: get(X), y: get(Y), rotation: get(ROTATION), scale_x: get(SCALE_X), scale_y: get(SCALE_Y) }
    };

    let mut list = Vec::new();
    for (i, part) in v.parts.iter().enumerate() {
        let Some(track) = tracks[i] else { continue };
        let Some(mut drawing) = image(&track.image, tick) else { continue };
        // A part marked extra shows only if a variant shows it; the last
        // variant to mention a part decides.
        let mut shown = !s.parts[part.part as usize].extra;
        for variant in &active {
            if variant.show.contains(&part.part) {
                shown = true;
            }
            if variant.hide.contains(&part.part) {
                shown = false;
            }
        }
        if !shown {
            continue;
        }
        // The last active variant that replaces this drawing wins.
        // Replacements do not chain.
        let original = drawing;
        for variant in &active {
            for r in &variant.replace {
                if r.view as usize == view && r.from == original {
                    drawing = r.to;
                }
            }
        }
        let Some(place) = s.drawings[drawing as usize].places[view] else { continue };
        // Join the part to its parents, outermost first.
        let mut chain = vec![i];
        while let Some(parent) = v.parts[*chain.last().unwrap()].parent {
            chain.push(parent as usize);
        }
        let mut world = local(*chain.last().unwrap());
        for &c in chain.iter().rev().skip(1) {
            world = world.then(&local(c));
        }
        list.push((
            number(&track.numbers[Z], tick, 0.0),
            Placed {
                part: i,
                drawing,
                place,
                opacity: number(&track.numbers[OPACITY], tick, 1.0).clamp(0.0, 1.0) as f32,
                emission: number(&track.numbers[EMISSION], tick, 1.0) as f32,
                matrix: world.matrix(),
            },
        ));
    }
    list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.part.cmp(&b.1.part)));
    list.into_iter().map(|(_, p)| p).collect()
}

/// The palette in use: the one named by the last active variant that names
/// one, else the first.
pub fn palette(s: &Sprite, variants: &[usize]) -> usize {
    variants.iter().rev().find_map(|&i| s.variants[i].palette).unwrap_or(0) as usize
}

/// The picture of `view` at `tick` of `animation`: straight (not
/// premultiplied) 8-bit RGBA the size of the canvas, transparent where
/// nothing draws.
pub fn picture(s: &Sprite, view: usize, animation: usize, tick: f64, variants: &[usize]) -> Frame {
    let v = &s.views[view];
    let colors = &s.palettes[palette(s, variants)].colors;
    let mut frame = Frame { width: v.width, height: v.height, pixels: vec![0; (v.width * v.height * 4) as usize] };
    let sheet = &v.color;
    for p in placed(s, view, animation, tick, variants) {
        let m = p.matrix;
        let det = m[0] * m[3] - m[1] * m[2];
        if p.opacity <= 0.0 || det.abs() < 1e-12 {
            continue;
        }
        // Nearest sample at each canvas pixel's center.
        let (a, b, c, d) = (m[3] / det, -m[1] / det, -m[2] / det, m[0] / det);
        let (tx, ty) = (-(a * m[4] + c * m[5]), -(b * m[4] + d * m[5]));
        let at = (f64::from(p.place.offset_x), f64::from(p.place.offset_y));
        let (w, h) = (f64::from(p.place.width), f64::from(p.place.height));
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for (x, y) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
            let (x, y) = (at.0 + x, at.1 + y);
            let (sx, sy) = (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
            x0 = x0.min(sx.floor() as i64);
            y0 = y0.min(sy.floor() as i64);
            x1 = x1.max(sx.ceil() as i64);
            y1 = y1.max(sy.ceil() as i64);
        }
        let origin = (i64::from(v.x), i64::from(v.y));
        let (max_x, max_y) = (origin.0 + i64::from(v.width), origin.1 + i64::from(v.height));
        for sy in y0.max(origin.1)..y1.min(max_y) {
            for sx in x0.max(origin.0)..x1.min(max_x) {
                let (px, py) = (sx as f64 + 0.5, sy as f64 + 0.5);
                let (lx, ly) = (a * px + c * py + tx, b * px + d * py + ty);
                let (u, t) = (texel(lx - at.0), texel(ly - at.1));
                if !(u >= 0.0 && t >= 0.0 && u < w) || t >= h {
                    continue;
                }
                let i = (p.place.y as usize + t as usize) * sheet.width as usize + p.place.x as usize + u as usize;
                if sheet.opacity[i] == 0 {
                    continue;
                }
                let rgb = colors[sheet.number[i] as usize];
                // A palette color's opacity multiplies the pixel's.
                let alpha = ((u16::from(sheet.opacity[i]) * u16::from(rgb[3]) + 127) / 255) as u8;
                let o = (((sy - origin.1) as u32 * frame.width + (sx - origin.0) as u32) * 4) as usize;
                blend(&mut frame.pixels[o..o + 4], [rgb[0], rgb[1], rgb[2], alpha], p.opacity);
            }
        }
    }
    frame
}

/// [`picture`] by ids.
pub fn picture_named(s: &Sprite, view: &str, animation: &str, tick: f64, variants: &[&str]) -> Result<Frame> {
    let v = s.view(view).with_context(|| format!("No view {view}"))?;
    let a = s.animation(animation).with_context(|| format!("No animation {animation}"))?;
    let active = variants.iter().map(|id| s.variant(id).with_context(|| format!("No variant {id}"))).collect::<Result<Vec<_>>>()?;
    Ok(picture(s, v, a, tick, &active))
}

/// A part's placement. Joined parts combine these, never whole matrices: a
/// child's position goes through its parent's full placement, its rotation
/// adds to the parent's (reversed under a mirroring parent) and its scale
/// multiplies per axis. A part therefore never skews.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Trs {
    x: f64,
    y: f64,
    rotation: f64,
    scale_x: f64,
    scale_y: f64,
}
impl Trs {
    /// `[a, b, c, d, tx, ty]`: `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`.
    fn matrix(&self) -> [f64; 6] {
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        [
            cos * self.scale_x,
            sin * self.scale_x,
            -sin * self.scale_y,
            cos * self.scale_y,
            self.x,
            self.y,
        ]
    }
    /// `local` placed under this placement as its parent. Results within
    /// 1e-9 of a multiple of 2^-20 land exactly on it, so a joined part
    /// draws exactly where the same numbers would put it on its own.
    fn then(&self, local: &Trs) -> Trs {
        let m = self.matrix();
        let x = m[0] * local.x + m[2] * local.y + m[4];
        let y = m[1] * local.x + m[3] * local.y + m[5];
        let mirrors = (self.scale_x < 0.0) != (self.scale_y < 0.0);
        let turn = if mirrors {
            -local.rotation
        } else {
            local.rotation
        };
        let tidy = |v: f64| {
            const STEP: f64 = (1u32 << 20) as f64;
            let snapped = (v * STEP).round() / STEP;
            if (snapped - v).abs() < 1e-9 {
                snapped
            } else {
                v
            }
        };
        Trs {
            x: tidy(x),
            y: tidy(y),
            rotation: tidy(self.rotation + turn),
            scale_x: tidy(self.scale_x * local.scale_x),
            scale_y: tidy(self.scale_y * local.scale_y),
        }
    }
}

/// The texel a local coordinate falls in. A sample within 1e-9 of a texel
/// edge counts as on it, so equivalent placements pick the same texel.
fn texel(local: f64) -> f64 {
    let edge = local.round();
    if (local - edge).abs() < 1e-9 {
        edge
    } else {
        local.floor()
    }
}

/// Paint `color`, faded by `opacity`, over `dst`. Both are straight RGBA.
fn blend(dst: &mut [u8], color: [u8; 4], opacity: f32) {
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
