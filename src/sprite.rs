//! What a sprite holds. This mirrors the specification section by section.
use anyhow::{Result, bail, ensure};
use std::collections::HashSet;

/// Limits every file must keep.
pub const MAX_FILE: usize = 64 * 1024 * 1024;
pub const MAX_VIEWS: usize = 256;
pub const MAX_PARTS: usize = 4096;
pub const MAX_DRAWINGS: usize = 4096;
pub const MAX_COLORS: usize = 65536;
pub const MAX_MATERIALS: usize = 256;
pub const MAX_SHEET: u32 = 16384;
pub const MAX_CANVAS: u32 = 4096;
/// Largest size of any animated number.
pub const MAX_NUMBER: f64 = 1_000_000.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sprite {
    pub name: String,
    pub author: String,
    /// The sprite's own version, from 1. Not the format version.
    pub version: u32,
    pub ticks_per_second: f32,
    pub views: Vec<View>,
    pub parts: Vec<Part>,
    pub drawings: Vec<Drawing>,
    pub palettes: Vec<Palette>,
    pub materials: Vec<Material>,
    pub variants: Vec<Variant>,
    pub animations: Vec<Animation>,
    pub extras: Vec<Extra>,
    /// Chunks this reader does not know but may ignore, kept to write back.
    pub unknown: Vec<([u8; 4], Vec<u8>)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    pub id: String,
    pub name: String,
    /// The canvas: the sprite-space point of its top-left pixel, and its size.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// The parts this view shows. Position in this list breaks draw-order ties.
    pub parts: Vec<ViewPart>,
    pub color: ColorSheet,
    pub light: Option<LightSheet>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewPart {
    /// Which of the sprite's parts.
    pub part: u16,
    /// The position in this view's list of the part it is joined to.
    pub parent: Option<u16>,
}

/// Per pixel: a color number into the active palette, and an opacity.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ColorSheet {
    pub width: u32,
    pub height: u32,
    pub number: Vec<u16>,
    pub opacity: Vec<u8>,
}

/// Per pixel, four bytes: facing left-to-right, facing bottom-to-top (each
/// `128 + 127·n` for n from −1 to 1), material number, glow.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightSheet {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Part {
    pub id: String,
    pub name: String,
    /// Hidden unless an active variant shows it.
    pub extra: bool,
    /// How far in front of (positive) or behind the rest of the sprite, in pixels.
    pub depth: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    pub id: String,
    pub name: String,
    /// One entry per view: where its picture is, or none.
    pub places: Vec<Option<Place>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// The rectangle on the view's sheets.
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    /// The picture's top-left corner, from the origin of the part showing it.
    pub offset_x: f32,
    pub offset_y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Palette {
    pub id: String,
    pub name: String,
    /// Red, green, blue, and an opacity that multiplies the pixel's own.
    pub colors: Vec<[u8; 4]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Material {
    pub name: String,
    /// −1 fuzzy, 0 matte, 1 polished.
    pub finish: f32,
    /// −1 metal, 0 solid, 0.5 translucent, 1 clear.
    pub body: f32,
    /// Which of `shades` a surface facing the viewer head-on shows.
    pub base: u16,
    /// Color numbers that are this surface, darkest first.
    pub shades: Vec<u16>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Variant {
    pub id: String,
    pub name: String,
    pub palette: Option<u16>,
    pub replace: Vec<Replace>,
    pub show: Vec<u16>,
    pub hide: Vec<u16>,
}

/// In `view`, wherever drawing `from` would show, show `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Replace {
    pub view: u16,
    pub from: u16,
    pub to: u16,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Animation {
    pub id: String,
    pub name: String,
    /// One entry per view: how the view's parts move, or none.
    pub views: Vec<Option<Motion>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Motion {
    /// In ticks, more than 0.
    pub length: u32,
    pub looping: bool,
    pub tracks: Vec<Track>,
}

/// The eight numbers a part can animate, in file order, and the value each
/// has when it has no keys.
pub const PROPERTIES: [(&str, f64); 8] =
    [("x", 0.0), ("y", 0.0), ("rotation", 0.0), ("scaleX", 1.0), ("scaleY", 1.0), ("z", 0.0), ("opacity", 1.0), ("emission", 1.0)];
pub const X: usize = 0;
pub const Y: usize = 1;
pub const ROTATION: usize = 2;
pub const SCALE_X: usize = 3;
pub const SCALE_Y: usize = 4;
pub const Z: usize = 5;
pub const OPACITY: usize = 6;
pub const EMISSION: usize = 7;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub part: u16,
    /// Which drawing the part shows from each tick on; `None` is no drawing.
    pub image: Vec<(u32, Option<u16>)>,
    /// Keys for each of [`PROPERTIES`].
    pub numbers: [Vec<Key>; 8],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub tick: u32,
    pub value: f64,
    pub ease: Ease,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ease {
    #[default]
    Step,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}
impl Ease {
    pub const ALL: [Ease; 5] = [Ease::Step, Ease::Linear, Ease::EaseIn, Ease::EaseOut, Ease::EaseInOut];
    /// How far toward the next key a value has moved, `t` of the way there.
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Self::Step => 0.0,
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => t * t * (3.0 - 2.0 * t),
        }
    }
}

/// A file carried along that is not part of the sprite: history, original
/// game data. Games skip these; editors keep them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extra {
    pub path: String,
    pub bytes: Vec<u8>,
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>, what: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for id in ids {
        ensure!(!id.is_empty(), "A {what} has an empty id");
        ensure!(seen.insert(id), "Two {what}s have the id {id}");
    }
    Ok(())
}

impl Sprite {
    pub fn view(&self, id: &str) -> Option<usize> {
        self.views.iter().position(|v| v.id == id)
    }
    pub fn animation(&self, id: &str) -> Option<usize> {
        self.animations.iter().position(|a| a.id == id)
    }
    pub fn variant(&self, id: &str) -> Option<usize> {
        self.variants.iter().position(|v| v.id == id)
    }

    /// Every rule of the specification that can be checked without drawing.
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version >= 1, "The sprite's version starts at 1");
        ensure!(self.ticks_per_second.is_finite() && self.ticks_per_second > 0.0, "Ticks per second must be more than 0");
        ensure!(!self.views.is_empty() && self.views.len() <= MAX_VIEWS, "A sprite has 1 to {MAX_VIEWS} views");
        ensure!(self.parts.len() <= MAX_PARTS, "A sprite has at most {MAX_PARTS} parts");
        ensure!(self.drawings.len() <= MAX_DRAWINGS, "A sprite has at most {MAX_DRAWINGS} drawings");
        ensure!(self.materials.len() <= MAX_MATERIALS, "A sprite has at most {MAX_MATERIALS} materials");
        unique(self.views.iter().map(|v| v.id.as_str()), "view")?;
        unique(self.parts.iter().map(|v| v.id.as_str()), "part")?;
        unique(self.drawings.iter().map(|v| v.id.as_str()), "drawing")?;
        unique(self.palettes.iter().map(|v| v.id.as_str()), "palette")?;
        unique(self.variants.iter().map(|v| v.id.as_str()), "variant")?;
        unique(self.animations.iter().map(|v| v.id.as_str()), "animation")?;

        ensure!(!self.palettes.is_empty(), "A sprite has at least one palette");
        let colors = self.palettes[0].colors.len();
        ensure!((1..=MAX_COLORS).contains(&colors), "A palette has 1 to {MAX_COLORS} colors");
        ensure!(self.palettes.iter().all(|p| p.colors.len() == colors), "Every palette must have the same number of colors");

        for (i, m) in self.materials.iter().enumerate() {
            ensure!((-1.0..=1.0).contains(&m.finish) && (-1.0..=1.0).contains(&m.body), "Material {i}: finish and body run from -1 to 1");
            ensure!(m.shades.iter().all(|&c| (c as usize) < colors), "Material {i} lists a color the palette does not have");
            ensure!(m.shades.is_empty() && m.base == 0 || (m.base as usize) < m.shades.len(), "Material {i}: its base is not one of its shades");
        }

        for v in &self.views {
            let what = &v.id;
            ensure!((1..=MAX_CANVAS).contains(&v.width) && (1..=MAX_CANVAS).contains(&v.height), "View {what}: the canvas is 1 to {MAX_CANVAS} pixels a side");
            let c = &v.color;
            ensure!(c.width <= MAX_SHEET && c.height <= MAX_SHEET, "View {what}: a sheet is at most {MAX_SHEET} pixels a side");
            let n = c.width as usize * c.height as usize;
            ensure!(c.number.len() == n && c.opacity.len() == n, "View {what}: the Color sheet is the wrong size");
            ensure!(c.number.iter().zip(&c.opacity).all(|(&k, &o)| o == 0 || (k as usize) < colors), "View {what}: a pixel uses a color the palette does not have");
            if let Some(l) = &v.light {
                ensure!((l.width, l.height) == (c.width, c.height) && l.data.len() == n * 4, "View {what}: the Light sheet must be the size of the Color sheet");
                let rows = self.materials.len().max(1);
                ensure!(l.data.chunks_exact(4).zip(&c.opacity).all(|(p, &o)| o == 0 || (p[2] as usize) < rows), "View {what}: a pixel names a material the sprite does not have");
            }
            let mut seen = HashSet::new();
            for (i, p) in v.parts.iter().enumerate() {
                ensure!((p.part as usize) < self.parts.len(), "View {what} lists a part that does not exist");
                ensure!(seen.insert(p.part), "View {what} lists a part twice");
                if let Some(parent) = p.parent {
                    ensure!((parent as usize) < v.parts.len() && parent as usize != i, "View {what}: a part is joined to a part the view does not show");
                }
            }
            for start in 0..v.parts.len() {
                let (mut at, mut steps) = (start, 0);
                while let Some(parent) = v.parts[at].parent {
                    at = parent as usize;
                    steps += 1;
                    ensure!(steps <= v.parts.len(), "View {what}: parts are joined in a loop");
                }
            }
        }

        for d in &self.drawings {
            ensure!(d.places.len() == self.views.len(), "Drawing {}: one place per view", d.id);
            for (v, place) in d.places.iter().enumerate() {
                let Some(p) = place else { continue };
                let c = &self.views[v].color;
                ensure!(p.width >= 1 && p.height >= 1, "Drawing {}: an empty picture", d.id);
                ensure!(p.x as u32 + p.width as u32 <= c.width && p.y as u32 + p.height as u32 <= c.height, "Drawing {}: its picture is off the sheet", d.id);
                ensure!(p.offset_x.is_finite() && p.offset_y.is_finite(), "Drawing {}: its offset is not a number", d.id);
            }
        }

        for v in &self.variants {
            ensure!(v.palette.is_none_or(|p| (p as usize) < self.palettes.len()), "Variant {} names a palette that does not exist", v.id);
            for r in &v.replace {
                ensure!((r.view as usize) < self.views.len() && (r.from as usize) < self.drawings.len() && (r.to as usize) < self.drawings.len(), "Variant {} replaces something that does not exist", v.id);
            }
            ensure!(v.show.iter().chain(&v.hide).all(|&p| (p as usize) < self.parts.len()), "Variant {} shows or hides a part that does not exist", v.id);
        }

        for a in &self.animations {
            ensure!(a.views.len() == self.views.len(), "Animation {}: one entry per view", a.id);
            for (v, motion) in a.views.iter().enumerate() {
                let Some(m) = motion else { continue };
                let what = format!("Animation {} in view {}", a.id, self.views[v].id);
                ensure!(m.length > 0, "{what}: its length must be more than 0");
                let shown: HashSet<u16> = self.views[v].parts.iter().map(|p| p.part).collect();
                let mut seen = HashSet::new();
                for t in &m.tracks {
                    ensure!(shown.contains(&t.part), "{what} moves a part the view does not show");
                    ensure!(seen.insert(t.part), "{what} has two tracks for one part");
                    ensure!(t.image.windows(2).all(|w| w[0].0 < w[1].0), "{what}: keys must be in order of tick");
                    for (_, d) in &t.image {
                        ensure!(d.is_none_or(|d| (d as usize) < self.drawings.len()), "{what} shows a drawing that does not exist");
                    }
                    for keys in &t.numbers {
                        ensure!(keys.windows(2).all(|w| w[0].tick < w[1].tick), "{what}: keys must be in order of tick");
                        if keys.iter().any(|k| !k.value.is_finite() || k.value.abs() > MAX_NUMBER) {
                            bail!("{what}: a number is not finite or is larger than {MAX_NUMBER}");
                        }
                    }
                }
            }
        }

        unique(self.extras.iter().map(|e| e.path.as_str()), "extra")?;
        Ok(())
    }
}
