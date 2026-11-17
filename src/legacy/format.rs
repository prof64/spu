//! Working document model, explicitly experimental; not a ratified specification.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read, Write},
};

pub type Extras = BTreeMap<String, serde_json::Value>;
fn one() -> f32 {
    1.0
}
fn rough() -> f32 {
    0.65
}
fn ior() -> f32 {
    1.45
}
fn default_material() -> String {
    DEFAULT_MATERIAL.into()
}

/// Format identity written to `sprite.json`.
pub const FORMAT: &str = "spu";
pub const VERSION: u32 = 1;
/// Drawings that name no material use this one; a file need not declare it.
pub const DEFAULT_MATERIAL: &str = "default";
/// Optional capabilities this implementation understands beyond the core.
pub const CAPABILITIES: &[&str] = &[];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub ticks_per_second: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palettes: Vec<Palette>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub materials: Vec<Material>,
    pub views: Vec<View>,
    #[serde(flatten)]
    pub extra: Extras,
}

/// Shared colors for indexed drawings. Index 0 always draws transparent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Palette {
    pub id: String,
    pub name: String,
    /// `#rrggbb` or `#rrggbbaa`, at most 256.
    pub colors: Vec<String>,
    #[serde(flatten)]
    pub extra: Extras,
}

/// An alternate look: another palette, replacement drawings, or both.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replacements: Vec<Replacement>,
    #[serde(flatten)]
    pub extra: Extras,
}

/// While the variant is active, `image` in `view` draws as `with`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Replacement {
    pub view: String,
    pub image: String,
    pub with: String,
}

pub fn parse_color(text: &str) -> Result<[u8; 4]> {
    let hex = text.strip_prefix('#').context("Color must start with #")?;
    ensure!(
        (hex.len() == 6 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit()),
        "Color must be #rrggbb or #rrggbbaa: {text}"
    );
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
    Ok([
        byte(0),
        byte(2),
        byte(4),
        if hex.len() == 8 { byte(6) } else { 255 },
    ])
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    pub id: String,
    pub name: String,
    #[serde(default = "rough")]
    pub roughness: f32,
    #[serde(default)]
    pub metalness: f32,
    #[serde(default)]
    pub transmission: f32,
    #[serde(default = "ior")]
    pub ior: f32,
    #[serde(default = "one")]
    pub thickness: f32,
    #[serde(default)]
    pub emission: f32,
    #[serde(default = "one")]
    pub normal_strength: f32,
    #[serde(default = "one")]
    pub opacity: f32,
    #[serde(default = "one")]
    pub reflectivity: f32,
    #[serde(flatten)]
    pub extra: Extras,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct View {
    pub id: String,
    pub name: String,
    pub canvas: Canvas,
    pub images: Vec<Drawing>,
    pub parts: Vec<Part>,
    pub animations: Vec<Animation>,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Canvas {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Drawing {
    pub id: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub color: String,
    #[serde(default)]
    pub normal: Option<String>,
    #[serde(default)]
    pub surface: Option<String>,
    #[serde(default)]
    pub emission: Option<String>,
    #[serde(default = "default_material")]
    pub material: String,
    /// Drawings painted directly over this one, in the same part space.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlays: Vec<String>,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    pub id: String,
    pub name: String,
    pub length: f64,
    pub looping: bool,
    pub tracks: Vec<Track>,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub part: String,
    #[serde(default)]
    pub visible: Vec<Key<bool>>,
    #[serde(default)]
    pub image: Vec<Key<Option<String>>>,
    #[serde(default)]
    pub x: Vec<Key<f64>>,
    #[serde(default)]
    pub y: Vec<Key<f64>>,
    #[serde(default)]
    pub rotation: Vec<Key<f64>>,
    #[serde(default)]
    pub scale_x: Vec<Key<f64>>,
    #[serde(default)]
    pub scale_y: Vec<Key<f64>>,
    #[serde(default)]
    pub z: Vec<Key<f64>>,
    #[serde(default)]
    pub opacity: Vec<Key<f64>>,
    #[serde(default)]
    pub emission: Vec<Key<f64>>,
    #[serde(flatten)]
    pub extra: Extras,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ease {
    #[default]
    Step,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}
impl Ease {
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Key<T> {
    pub tick: f64,
    pub value: T,
    #[serde(default)]
    pub ease: Ease,
}

/// Bounded in-memory archive; filenames are never extracted to the filesystem.
#[derive(Clone)]
pub struct Package {
    pub document: Document,
    pub entries: BTreeMap<String, Vec<u8>>,
    pub compressed_bytes: usize,
    /// The older format this was read from and upgraded, if any.
    pub migrated_from: Option<String>,
}
impl Package {
    pub fn read(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= 32 * 1024 * 1024,
            "File exceeds 32 MiB prototype limit"
        );
        let mut zip =
            zip::ZipArchive::new(Cursor::new(bytes)).context("Expected an .spu ZIP package")?;
        ensure!(zip.len() <= 2048, "Too many archive members");
        let mut entries = BTreeMap::new();
        let mut total = 0usize;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            ensure!(!entry.is_dir(), "Directory entries are unnecessary");
            ensure!(entry.enclosed_name().is_some(), "Unsafe archive path");
            let name = entry.name().to_owned();
            ensure!(
                entry.size() <= 16 * 1024 * 1024,
                "Archive member too large: {name}"
            );
            let mut data = Vec::new();
            entry
                .by_ref()
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut data)?;
            ensure!(
                data.len() <= 16 * 1024 * 1024,
                "Archive member exceeds size limit"
            );
            total += data.len();
            ensure!(total <= 64 * 1024 * 1024, "Unpacked archive exceeds 64 MiB");
            ensure!(
                entries.insert(name, data).is_none(),
                "Duplicate archive member"
            );
        }
        let manifest = entries.get("sprite.json").context("Missing sprite.json")?;
        ensure!(manifest.len() <= 4 * 1024 * 1024, "Manifest exceeds 4 MiB");
        let manifest: serde_json::Value =
            serde_json::from_slice(manifest).context("sprite.json is not JSON")?;
        let (manifest, migrated_from) = super::migrate::upgrade(manifest, &mut entries)?;
        let document: Document =
            serde_json::from_value(manifest).context("Invalid sprite.json")?;
        document.validate()?;
        Ok(Self {
            document,
            entries,
            compressed_bytes: bytes.len(),
            migrated_from,
        })
    }
    pub fn write(&self) -> Result<Vec<u8>> {
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        archive.start_file("sprite.json", options)?;
        archive.write_all(&serde_json::to_vec_pretty(&self.document)?)?;
        for (name, bytes) in &self.entries {
            if name == "sprite.json" {
                continue;
            }
            archive.start_file(name, options)?;
            archive.write_all(bytes)?;
        }
        Ok(archive.finish()?.into_inner())
    }
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>, what: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for id in ids {
        ensure!(
            !id.is_empty() && seen.insert(id),
            "Empty or duplicate {what} id: {id}"
        );
    }
    Ok(())
}
fn keys<T>(v: &[Key<T>]) -> Result<()> {
    ensure!(v.len() < 100_000, "Too many keys");
    let mut prev = -1.0;
    for k in v {
        // Keys may sit at or past the clip's end: they still shape the
        // motion toward them.
        ensure!(
            k.tick.is_finite() && k.tick >= 0.0 && k.tick <= 1e9 && k.tick > prev,
            "Keys must be at ordered, distinct, non-negative ticks"
        );
        prev = k.tick;
    }
    Ok(())
}
impl Document {
    /// The material a drawing uses; the built-in default unless declared.
    pub fn material(&self, id: &str) -> Material {
        self.materials
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .unwrap_or_else(|| {
                serde_json::from_value(serde_json::json!({"id": id, "name": "Default"})).unwrap()
            })
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.format == FORMAT && self.version == VERSION,
            "Unsupported format/version; this reads SPU version {VERSION}"
        );
        for cap in &self.required {
            ensure!(
                CAPABILITIES.contains(&cap.as_str()),
                "Unsupported required capability: {cap}"
            );
        }
        ensure!(
            self.ticks_per_second.is_finite()
                && self.ticks_per_second > 0.0
                && self.ticks_per_second <= 10000.0,
            "Invalid clock"
        );
        ensure!(
            !self.views.is_empty() && self.views.len() <= 32,
            "Need 1–32 views"
        );
        ensure!(self.materials.len() <= 256, "At most 256 materials");
        ensure!(
            self.palettes.len() <= 256 && self.variants.len() <= 256,
            "At most 256 palettes and 256 variants"
        );
        unique(self.materials.iter().map(|x| x.id.as_str()), "material")?;
        unique(self.palettes.iter().map(|x| x.id.as_str()), "palette")?;
        unique(self.variants.iter().map(|x| x.id.as_str()), "variant")?;
        for p in &self.palettes {
            ensure!(
                !p.colors.is_empty() && p.colors.len() <= 256,
                "A palette holds 1–256 colors"
            );
            for c in &p.colors {
                parse_color(c)?;
            }
        }
        for variant in &self.variants {
            if let Some(id) = &variant.palette {
                ensure!(
                    self.palettes.iter().any(|p| &p.id == id),
                    "Unknown palette {id}"
                );
            }
            for r in &variant.replacements {
                let view = self
                    .views
                    .iter()
                    .find(|v| v.id == r.view)
                    .with_context(|| format!("Unknown view {}", r.view))?;
                for id in [&r.image, &r.with] {
                    ensure!(
                        view.images.iter().any(|i| &i.id == id),
                        "Unknown drawing {id}"
                    );
                }
            }
        }
        unique(self.views.iter().map(|x| x.id.as_str()), "view")?;
        for m in &self.materials {
            for x in [m.roughness, m.metalness, m.transmission, m.opacity] {
                ensure!(
                    x.is_finite() && (0.0..=1.0).contains(&x),
                    "Invalid material factor"
                );
            }
            ensure!(
                m.ior.is_finite() && (1.0..=3.0).contains(&m.ior),
                "Invalid index of refraction"
            );
            for x in [m.thickness, m.emission, m.normal_strength, m.reflectivity] {
                ensure!(
                    x.is_finite() && (0.0..=100.0).contains(&x),
                    "Invalid material factor"
                );
            }
        }
        for view in &self.views {
            ensure!(
                view.parts.len() <= 256 && view.images.len() <= 1024,
                "View exceeds part/drawing limit"
            );
            unique(view.parts.iter().map(|x| x.id.as_str()), "part")?;
            unique(view.images.iter().map(|x| x.id.as_str()), "drawing")?;
            unique(view.animations.iter().map(|x| x.id.as_str()), "animation")?;
            let c = view.canvas;
            ensure!(
                [c.x, c.y, c.width, c.height].iter().all(|x| x.is_finite())
                    && c.width > 0.0
                    && c.height > 0.0
                    && c.width <= 4096.0
                    && c.height <= 4096.0
                    && [c.x, c.y, c.width, c.height].iter().all(|x| x.fract() == 0.0),
                "Invalid view canvas"
            );
            for image in &view.images {
                ensure!(
                    image.x.is_finite() && image.y.is_finite(),
                    "Invalid drawing origin"
                );
                ensure!(
                    image.material == DEFAULT_MATERIAL
                        || self.materials.iter().any(|m| m.id == image.material),
                    "Unknown material {}",
                    image.material
                );
                for id in &image.overlays {
                    let over = view
                        .images
                        .iter()
                        .find(|i| &i.id == id)
                        .with_context(|| format!("Unknown overlay drawing {id}"))?;
                    ensure!(
                        over.overlays.is_empty() && id != &image.id,
                        "An overlay cannot have overlays of its own"
                    );
                }
            }
            for part in &view.parts {
                let mut cursor = part;
                let mut visited = HashSet::new();
                while let Some(id) = &cursor.parent {
                    ensure!(visited.insert(id), "Cyclic part hierarchy");
                    cursor = view
                        .parts
                        .iter()
                        .find(|p| &p.id == id)
                        .context("Unknown parent")?;
                }
            }
            ensure!(
                !view.animations.is_empty() && view.animations.len() <= 256,
                "Need 1–256 animations per view"
            );
            for a in &view.animations {
                ensure!(
                    a.length.is_finite() && a.length > 0.0 && a.length <= 1e9,
                    "Invalid animation length"
                );
                unique(a.tracks.iter().map(|x| x.part.as_str()), "track target")?;
                for t in &a.tracks {
                    ensure!(
                        view.parts.iter().any(|p| p.id == t.part),
                        "Unknown animation target"
                    );
                    keys(&t.image)?;
                    keys(&t.visible)?;
                    for k in &t.image {
                        if let Some(id) = &k.value {
                            ensure!(
                                view.images.iter().any(|i| &i.id == id),
                                "Unknown drawing {id}"
                            );
                        }
                    }
                    for v in [
                        &t.x,
                        &t.y,
                        &t.rotation,
                        &t.scale_x,
                        &t.scale_y,
                        &t.z,
                        &t.opacity,
                        &t.emission,
                    ] {
                        keys(v)?;
                        if v.iter()
                            .any(|k| !k.value.is_finite() || k.value.abs() > 1e6)
                        {
                            bail!("Invalid key value");
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
