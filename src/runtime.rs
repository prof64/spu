use crate::format::*;
use anyhow::{Context, Result, ensure};
use bytemuck::{Pod, Zeroable};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Cursor};

/// The value at `tick`: the key at or before it, eased toward the next.
/// Before the first key, the first key's value holds.
pub fn sample64(keys: &[Key<f64>], tick: f64, default: f64) -> f64 {
    let i = keys.partition_point(|k| k.tick <= tick);
    if i == 0 {
        return keys.first().map_or(default, |k| k.value);
    }
    let a = &keys[i - 1];
    match keys.get(i) {
        Some(b) if !matches!(a.ease, Ease::Step) => {
            a.value + (b.value - a.value) * a.ease.apply((tick - a.tick) / (b.tick - a.tick))
        }
        _ => a.value,
    }
}
pub fn sample(keys: &[Key<f64>], tick: f32, default: f32) -> f32 {
    sample64(keys, tick.into(), default.into()) as f32
}
pub fn hold<T: Clone>(keys: &[Key<T>], tick: f64, default: T) -> T {
    let i = keys.partition_point(|k| k.tick <= tick);
    keys.get(i.saturating_sub(1))
        .map_or(default, |k| k.value.clone())
}

/// A part's placement. Joined parts combine these, never whole matrices: a
/// child's position goes through its parent's full placement, its rotation
/// adds to the parent's (reversed under a mirroring parent) and its scale
/// multiplies per axis. A part therefore never skews.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trs {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
}
impl Default for Trs {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}
impl Trs {
    pub fn of(track: &Track, tick: f64) -> Self {
        Self {
            x: sample64(&track.x, tick, 0.0),
            y: sample64(&track.y, tick, 0.0),
            rotation: sample64(&track.rotation, tick, 0.0),
            scale_x: sample64(&track.scale_x, tick, 1.0),
            scale_y: sample64(&track.scale_y, tick, 1.0),
        }
    }
    /// `[a, b, c, d, tx, ty]`: `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty`.
    pub fn matrix(&self) -> [f64; 6] {
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
    pub fn then(&self, local: &Trs) -> Trs {
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

/// Affine matrices keep genuine parent shear and mirroring, including forward parent references.
#[derive(Clone, Copy, Debug)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub x: f32,
    pub y: f32,
}
impl Default for Affine {
    fn default() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            x: 0.0,
            y: 0.0,
        }
    }
}
impl Affine {
    pub fn trs(x: f32, y: f32, r: f32, sx: f32, sy: f32) -> Self {
        let (s, c) = r.to_radians().sin_cos();
        Self {
            a: c * sx,
            b: s * sx,
            c: -s * sy,
            d: c * sy,
            x,
            y,
        }
    }
    pub fn compose(self, o: Self) -> Self {
        Self {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            x: self.a * o.x + self.c * o.y + self.x,
            y: self.b * o.x + self.d * o.y + self.y,
        }
    }
    pub fn point(self, x: f32, y: f32) -> [f32; 2] {
        [
            self.a * x + self.c * y + self.x,
            self.b * x + self.d * y + self.y,
        ]
    }
}
#[derive(Clone, Copy, Default)]
struct Pose {
    world: Affine,
    image: Option<usize>,
    visible: bool,
    z: f32,
    opacity: f32,
    emission: f32,
}

pub struct CompiledDrawing {
    pub rect: [f32; 4],
    pub size: [f32; 2],
    pub origin: [f32; 2],
    pub material: Material,
    // Emission-map centroid, mean linear radiance, and luminous pixel area.
    pub emitter: [f32; 6],
}
pub struct CompiledClip {
    pub tracks: Vec<Track>,
    pub image_keys: Vec<Vec<Key<Option<usize>>>>,
    pub length: f32,
    pub looping: bool,
}
pub struct CompiledView {
    pub canvas: Canvas,
    pub images: Vec<CompiledDrawing>,
    pub clips: Vec<CompiledClip>,
    pub parents: Vec<Option<usize>>,
    pub order: Vec<usize>,
}
pub struct Asset {
    pub package: Package,
    pub views: Vec<CompiledView>,
    pub light_power: f32,
    pub light_radius: f32,
}
pub struct Scene {
    pub assets: Vec<Asset>,
    pub atlas: [RgbaImage; 4],
    pub emitters: Vec<Emitter>,
    pub reflection_instances: Vec<Instance>,
    capture_scratch: Vec<Instance>,
    scratch: Vec<Pose>,
    placed: Vec<Trs>,
    sort: Vec<usize>,
    world_entities: Vec<crate::world::Entity>,
    pub terrain_tiles: usize,
}

/// A bounded approximation for light falling on the procedural cave, not a shadow map.
#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct Emitter {
    pub position: [f32; 4],
    pub color: [f32; 4],
}

/// Battle camera uses the same world positions for rendering and reflection rays.
/// Entries are opponent/front, then player/back; XYZ + square capture half-size.
pub fn battle_camera(width: u32, height: u32) -> [f32; 2] {
    [width as f32 / height.max(1) as f32 * 0.45 / 2.2, 0.31 / 2.2]
}
pub fn battle_placements(width: u32, height: u32) -> [[f32; 4]; 2] {
    let aspect = width as f32 / height.max(1) as f32;
    let camera = battle_camera(width, height);
    let fit = (aspect / 1.75).clamp(0.62, 1.0);
    [[0.72, 0.35, -0.55, 0.48], [0.27, 0.66, 0.55, 0.58]].map(|p| {
        [
            (p[0] - 0.5) * aspect + camera[0] * p[2],
            0.5 - p[1] + camera[1] * p[2],
            p[2],
            p[3] * fit,
        ]
    })
}

fn emission_summary(color: &RgbaImage, emission: &RgbaImage) -> [f32; 6] {
    let (mut rgb, mut center, mut weight, mut coverage) = ([0.0; 3], [0.0; 2], 0.0, 0.0);
    for (x, y, pixel) in emission.enumerate_pixels() {
        let alpha = color.get_pixel(x, y).0[3] as f32 / 255.0;
        let c = std::array::from_fn::<_, 3, _>(|i| (pixel.0[i] as f32 / 255.0).powf(2.2) * alpha);
        let luminance = c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
        for i in 0..3 {
            rgb[i] += c[i];
        }
        center[0] += (x as f32 + 0.5) * luminance;
        center[1] += (y as f32 + 0.5) * luminance;
        weight += luminance;
        coverage += alpha;
    }
    if weight < 1e-6 {
        return [0.0; 6];
    }
    [
        center[0] / weight,
        center[1] / weight,
        rgb[0] / coverage,
        rgb[1] / coverage,
        rgb[2] / coverage,
        weight,
    ]
}

fn decode(package: &Package, path: &str) -> Result<RgbaImage> {
    let bytes = package
        .entries
        .get(path)
        .with_context(|| format!("Missing texture: {path}"))?;
    let mut reader = image::io::Reader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::io::Limits::default();
    limits.max_image_width = Some(2048);
    limits.max_image_height = Some(2048);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader
        .decode()
        .with_context(|| format!("Invalid PNG: {path}"))?
        .to_rgba8())
}
impl Scene {
    pub fn load(packages: Vec<Package>) -> Result<Self> {
        ensure!(
            !packages.is_empty() && packages.len() <= 32,
            "Load 1–32 assets"
        );
        let width = 1024u32;
        let (mut x, mut y, mut row_h) = (1u32, 1u32, 0u32);
        let mut placements = Vec::new();
        for (a, p) in packages.iter().enumerate() {
            p.document.validate()?;
            for (v, view) in p.document.views.iter().enumerate() {
                for (i, drawing) in view.images.iter().enumerate() {
                    let color = decode(p, &drawing.color)?;
                    let (w, h) = color.dimensions();
                    ensure!(
                        w + 2 <= width && h > 0 && w > 0,
                        "Texture exceeds atlas width or is empty"
                    );
                    if x + w + 1 > width {
                        x = 1;
                        y += row_h + 2;
                        row_h = 0;
                    }
                    ensure!(
                        y + h + 1 <= 4096,
                        "Combined texture atlas exceeds prototype 1024×4096 limit"
                    );
                    let mut maps = vec![color];
                    for (path, default) in [
                        (&drawing.normal, [128, 128, 255, 255]),
                        (&drawing.surface, [255, 255, 255, 255]),
                        (&drawing.emission, [0, 0, 0, 255]),
                    ] {
                        let map = if let Some(path) = path {
                            decode(p, path)?
                        } else {
                            RgbaImage::from_pixel(w, h, image::Rgba(default))
                        };
                        ensure!(
                            map.dimensions() == (w, h),
                            "Material map dimensions must match drawing"
                        );
                        maps.push(map);
                    }
                    placements.push((a, v, i, x, y, maps));
                    x += w + 2;
                    row_h = row_h.max(h);
                }
            }
        }
        let height = (y + row_h + 1).next_power_of_two().max(4);
        let mut atlas = std::array::from_fn(|_| RgbaImage::new(width, height));
        let mut rects = HashMap::new();
        for (a, v, i, x, y, maps) in placements {
            for (channel, map) in maps.iter().enumerate() {
                image::imageops::replace(&mut atlas[channel], map, x.into(), y.into());
                // Extrude edge texels into a one-pixel gutter; nearest sampling still clamps safely.
                for yy in 0..map.height() {
                    atlas[channel].put_pixel(x - 1, y + yy, *map.get_pixel(0, yy));
                    atlas[channel].put_pixel(
                        x + map.width(),
                        y + yy,
                        *map.get_pixel(map.width() - 1, yy),
                    );
                }
                for xx in 0..map.width() {
                    atlas[channel].put_pixel(x + xx, y - 1, *map.get_pixel(xx, 0));
                    atlas[channel].put_pixel(
                        x + xx,
                        y + map.height(),
                        *map.get_pixel(xx, map.height() - 1),
                    );
                }
            }
            rects.insert(
                (a, v, i),
                (
                    [
                        x as f32 / width as f32,
                        y as f32 / height as f32,
                        maps[0].width() as f32 / width as f32,
                        maps[0].height() as f32 / height as f32,
                    ],
                    [maps[0].width() as f32, maps[0].height() as f32],
                    emission_summary(&maps[0], &maps[3]),
                ),
            );
        }
        let mut assets = Vec::new();
        for (a, package) in packages.into_iter().enumerate() {
            let mut views = Vec::new();
            for (v, view) in package.document.views.iter().enumerate() {
                let ids: HashMap<_, _> = view
                    .images
                    .iter()
                    .enumerate()
                    .map(|(i, d)| (&d.id, i))
                    .collect();
                let part_ids: HashMap<_, _> = view
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (&p.id, i))
                    .collect();
                let parents: Vec<_> = view
                    .parts
                    .iter()
                    .map(|p| p.parent.as_ref().map(|id| part_ids[id]))
                    .collect();
                let mut order = Vec::new();
                while order.len() < parents.len() {
                    let before = order.len();
                    for (i, p) in parents.iter().enumerate() {
                        if !order.contains(&i) && p.is_none_or(|p| order.contains(&p)) {
                            order.push(i);
                        }
                    }
                    ensure!(order.len() > before, "Cyclic hierarchy");
                }
                let images = view
                    .images
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let (rect, size, emitter) = rects[&(a, v, i)];
                        CompiledDrawing {
                            rect,
                            size,
                            origin: [d.x, d.y],
                            emitter,
                            material: package.document.material(&d.material),
                        }
                    })
                    .collect();
                let clips = view
                    .animations
                    .iter()
                    .map(|clip| {
                        let mut tracks = vec![Track::default(); parents.len()];
                        let mut image_keys = vec![Vec::new(); parents.len()];
                        for t in &clip.tracks {
                            let p = part_ids[&t.part];
                            tracks[p] = t.clone();
                            image_keys[p] = t
                                .image
                                .iter()
                                .map(|k| Key {
                                    tick: k.tick,
                                    value: k.value.as_ref().map(|id| ids[id]),
                                    ease: k.ease,
                                })
                                .collect();
                        }
                        CompiledClip {
                            tracks,
                            image_keys,
                            length: clip.length as f32,
                            looping: clip.looping,
                        }
                    })
                    .collect();
                views.push(CompiledView {
                    canvas: view.canvas,
                    images,
                    clips,
                    parents,
                    order,
                });
            }
            let lighting = package.document.extra.get("lighting");
            let light_power = lighting
                .and_then(|l| l.get("power"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .clamp(0.0, 16.0) as f32;
            let light_radius = lighting
                .and_then(|l| l.get("radius"))
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0)
                .clamp(0.1, 4.0) as f32;
            assets.push(Asset {
                package,
                views,
                light_power,
                light_radius,
            });
        }
        Ok(Self {
            assets,
            atlas,
            emitters: Vec::with_capacity(16),
            reflection_instances: Vec::with_capacity(1024),
            capture_scratch: Vec::with_capacity(256),
            scratch: vec![Pose::default(); 256],
            placed: vec![Trs::default(); 256],
            sort: Vec::with_capacity(256),
            world_entities: Vec::new(),
            terrain_tiles: 0,
        })
    }
    pub fn texture_bytes(&self) -> usize {
        self.atlas.iter().map(|a| a.as_raw().len()).sum()
    }
    pub fn metadata(&self) -> serde_json::Value {
        serde_json::json!({"atlas":[self.atlas[0].width(),self.atlas[0].height()],"textureBytes":self.texture_bytes(),
            "assets":self.assets.iter().map(|a|serde_json::json!({"name":a.package.document.name,"bytes":a.package.compressed_bytes,
                "views":a.package.document.views.iter().map(|v|serde_json::json!({"name":v.name,"parts":v.parts.len(),"drawings":v.images.len(),
                    "clips":v.animations.iter().map(|c|serde_json::json!({"name":c.name,"duration":c.length/f64::from(a.package.document.ticks_per_second),"looping":c.looping})).collect::<Vec<_>>()
                })).collect::<Vec<_>>(),"extras":a.package.document.extra})).collect::<Vec<_>>()})
    }
    /// One pass over independent animation instances; shared textures, no per-frame decoding.
    pub fn prepare(
        &mut self,
        settings: &Settings,
        time: f32,
        width: u32,
        height: u32,
        output: &mut Vec<Instance>,
    ) {
        self.reflection_instances.clear();
        if !settings.reference
            && !settings.world
            && (settings.battle || matches!(settings.background, 6 | 7))
        {
            let mut scratch = std::mem::take(&mut self.capture_scratch);
            for tile in 0..4 {
                let asset = if tile < 2 {
                    settings.asset
                } else {
                    settings.partner
                }
                .min(self.assets.len() - 1);
                let desired = if tile % 2 == 0 { "front" } else { "back" };
                let view = self.assets[asset]
                    .package
                    .document
                    .views
                    .iter()
                    .position(|v| v.name.eq_ignore_ascii_case(desired))
                    .unwrap_or(0);
                let mut capture = if settings.battle || tile < 2 {
                    settings.clone()
                } else {
                    Settings::default()
                };
                capture.asset = asset;
                capture.view = view;
                capture.count = 1;
                capture.mixed = false;
                capture.background = 0;
                capture.battle = false;
                capture.phased = false;
                capture.zoom = if settings.battle || tile < 2 {
                    settings.zoom
                } else {
                    1.4
                };
                capture.animation = settings.animation;
                capture.animation_name = settings.animation_name.clone();
                capture.pixel_timing = settings.pixel_timing;
                self.prepare_batch(&capture, time, 256, 256, &mut scratch);
                for instance in &mut scratch {
                    instance.placement[0] += tile as f32 * 256.0;
                }
                self.reflection_instances.extend_from_slice(&scratch);
            }
            self.capture_scratch = scratch;
        }
        self.prepare_batch(settings, time, width, height, output);
    }
    fn prepare_batch(
        &mut self,
        settings: &Settings,
        time: f32,
        width: u32,
        height: u32,
        output: &mut Vec<Instance>,
    ) {
        output.clear();
        self.emitters.clear();
        let mut entities = std::mem::take(&mut self.world_entities);
        self.terrain_tiles = if settings.world {
            crate::world::populate(&self.assets, settings, time, width, height, &mut entities)
        } else {
            0
        };
        let count = if settings.world {
            entities.len()
        } else if settings.battle {
            2
        } else {
            settings.count.clamp(1, 10000) as usize
        };
        let battle = battle_placements(width, height);
        let cols = ((count as f32 * width as f32 / height as f32).sqrt().ceil() as usize).max(1);
        let rows = count.div_ceil(cols);
        for n in 0..count {
            let entity = if settings.world {
                entities[n]
            } else {
                crate::world::Entity::default()
            };
            let ai = if settings.world {
                entity.asset
            } else if settings.battle {
                if n == 0 {
                    settings.asset
                } else {
                    settings.partner
                }
                .min(self.assets.len() - 1)
            } else if settings.mixed {
                n % self.assets.len()
            } else {
                settings.asset.min(self.assets.len() - 1)
            };
            let asset = &self.assets[ai];
            let vi = if settings.world {
                entity.view
            } else if settings.battle {
                let name = if n == 0 { "front" } else { "back" };
                asset
                    .package
                    .document
                    .views
                    .iter()
                    .position(|v| v.name.eq_ignore_ascii_case(name))
                    .unwrap_or(0)
            } else {
                settings.view.min(asset.views.len() - 1)
            };
            let view = &asset.views[vi];
            let clip_index = if settings.world && settings.showcase > 0 && settings.showcase != 4 {
                settings.animation.min(view.clips.len() - 1)
            } else if settings.world {
                asset.package.document.views[vi]
                    .animations
                    .iter()
                    .position(|c| c.name.eq_ignore_ascii_case("Walk"))
                    .unwrap_or(0)
            } else if let Some(name) = settings.animation_name.as_deref() {
                let animations = &asset.package.document.views[vi].animations;
                animations
                    .iter()
                    .position(|clip| clip.name.eq_ignore_ascii_case(name))
                    .or_else(|| {
                        animations
                            .iter()
                            .position(|clip| clip.name.eq_ignore_ascii_case("Idle"))
                    })
                    .unwrap_or(0)
            } else {
                settings.animation.min(view.clips.len() - 1)
            };
            let clip = &view.clips[clip_index];
            let tick = time * asset.package.document.ticks_per_second
                + if settings.phased && count > 1 && !settings.battle && settings.showcase == 0 {
                    ((if settings.world {
                        entity.id as f32
                    } else {
                        n as f32
                    }) * 137.50777)
                        % clip.length
                } else {
                    0.0
                };
            let tick = if clip.looping {
                tick.rem_euclid(clip.length)
            } else {
                tick.clamp(0.0, (clip.length - 1.0).max(0.0))
            };
            let tick = if settings.pixel_timing {
                tick.floor()
            } else {
                tick
            };
            let (cw, ch) = (view.canvas.width, view.canvas.height);
            let (cx, cy) = (view.canvas.x + cw * 0.5, view.canvas.y + ch * 0.5);
            let (scale, ox, oy) = if settings.world {
                let scale = entity.size / ch;
                let anchor = asset
                    .package
                    .document
                    .extra
                    .get("anchor")
                    .and_then(|a| a.as_array());
                let ax = anchor
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(cx);
                let ay = anchor
                    .and_then(|a| a.get(1))
                    .and_then(|v| v.as_f64())
                    .map(|v| v as f32)
                    .unwrap_or(view.canvas.y + ch);
                (
                    scale,
                    entity.x + (cx - ax) * scale,
                    entity.y + (cy - ay) * scale,
                )
            } else if settings.battle {
                let p = battle[n];
                let camera = battle_camera(width, height);
                (
                    height as f32 * p[3] * 2.0 / cw.max(ch) * 0.68 * settings.zoom,
                    width as f32 * 0.5 + (p[0] - camera[0] * p[2]) * height as f32,
                    height as f32 * (0.5 - p[1] + camera[1] * p[2] + 0.02 * p[3]),
                )
            } else if count == 1 {
                (
                    (width as f32 / cw).min(height as f32 / ch) * 0.68 * settings.zoom,
                    width as f32 * 0.5,
                    height as f32 * 0.51,
                )
            } else if settings.overlap {
                (
                    64.0 / ch,
                    32.0 + (n as f32 * 97.131).rem_euclid((width as f32 - 64.0).max(1.0)),
                    32.0 + (n as f32 * 53.719).rem_euclid((height as f32 - 64.0).max(1.0)),
                )
            } else {
                let cellw = width as f32 / cols as f32;
                let cellh = height as f32 / rows as f32;
                (
                    (cellw / cw).min(cellh / ch) * 0.88,
                    (n % cols) as f32 * cellw + cellw * 0.5,
                    (n / cols) as f32 * cellh + cellh * 0.5,
                )
            };
            self.sort.clear();
            for &p in &view.order {
                let t = &clip.tracks[p];
                let mut placed = Trs::of(t, tick.into());
                if let Some(parent) = view.parents[p] {
                    placed = self.placed[parent].then(&placed);
                }
                self.placed[p] = placed;
                let m = placed.matrix();
                let pose = Pose {
                    world: Affine {
                        a: m[0] as f32,
                        b: m[1] as f32,
                        c: m[2] as f32,
                        d: m[3] as f32,
                        x: m[4] as f32,
                        y: m[5] as f32,
                    },
                    image: hold(&clip.image_keys[p], tick.into(), None),
                    visible: hold(&t.visible, tick.into(), true),
                    z: sample(&t.z, tick, 0.0),
                    opacity: sample(&t.opacity, tick, 1.0),
                    emission: sample(&t.emission, tick, 1.0),
                };
                self.scratch[p] = pose;
                if pose.visible && pose.image.is_some() {
                    self.sort.push(p);
                }
            }
            self.sort.sort_by(|&a, &b| {
                self.scratch[a]
                    .z
                    .total_cmp(&self.scratch[b].z)
                    .then(a.cmp(&b))
            });
            let mut world_light: Option<Emitter> = None;
            let mut world_light_power = 0.0;
            for &p in &self.sort {
                let pose = self.scratch[p];
                let d = &view.images[pose.image.unwrap()];
                let m = &d.material;
                let t = pose.world;
                let origin = t.point(d.origin[0], d.origin[1]);
                let det = t.a * t.d - t.b * t.c;
                if det.abs() < 1e-7 {
                    continue;
                }
                // Cap cave lighting at the first eight sprites / sixteen bright drawings.
                // The full crowd still animates and renders; no extra per-pixel crowd light loop.
                if ((settings.world && settings.lights > 0)
                    || (!settings.world && settings.background == 4 && n < 8))
                    && settings.mode == 0
                    && !settings.reference
                    && settings.spill > 0.0
                    && d.emitter[5] > 0.0
                {
                    let strength = asset.light_power
                        * if settings.world { settings.spill } else { 1.0 }
                        * m.emission
                        * settings.emission
                        * pose.emission
                        * (m.opacity * settings.opacity * pose.opacity).clamp(0.0, 1.0);
                    if strength > 0.0 {
                        let pos = t.point(d.origin[0] + d.emitter[0], d.origin[1] + d.emitter[1]);
                        let radius =
                            (d.emitter[5].sqrt() * scale * det.abs().sqrt() / height as f32 * 3.0)
                                .clamp(0.025, 0.45);
                        let emitter = Emitter {
                            position: [
                                ((pos[0] - cx) * scale + ox) / width as f32,
                                ((pos[1] - cy) * scale + oy) / height as f32,
                                if settings.world {
                                    (entity.y / height as f32 - 0.5) * 2.0 + 0.06
                                } else {
                                    0.08
                                },
                                if settings.world {
                                    radius.max(90.0 / height as f32) * asset.light_radius
                                } else {
                                    radius
                                },
                            ],
                            color: [
                                d.emitter[2] * strength,
                                d.emitter[3] * strength,
                                d.emitter[4] * strength,
                                0.0,
                            ],
                        };
                        if settings.world {
                            // Rank actual luminous area, before the minimum display light radius.
                            // Otherwise one tiny hot particle can replace a broad emissive body.
                            let power = d.emitter[5] * strength * det.abs();
                            if world_light.is_none() || power > world_light_power {
                                world_light = Some(emitter);
                                world_light_power = power;
                            }
                        } else if self.emitters.len() < 16 {
                            self.emitters.push(emitter);
                        } else {
                            let power = |e: &Emitter| {
                                (e.color[0] + e.color[1] + e.color[2])
                                    * e.position[3]
                                    * e.position[3]
                            };
                            let weakest = self
                                .emitters
                                .iter()
                                .enumerate()
                                .min_by(|(_, a), (_, b)| power(a).total_cmp(&power(b)))
                                .unwrap()
                                .0;
                            if power(&emitter) > power(&self.emitters[weakest]) {
                                self.emitters[weakest] = emitter;
                            }
                        }
                    }
                }
                output.push(Instance {
                    basis: [
                        t.a * d.size[0] * scale,
                        t.b * d.size[0] * scale,
                        t.c * d.size[1] * scale,
                        t.d * d.size[1] * scale,
                    ],
                    placement: [
                        (origin[0] - cx) * scale + ox,
                        (origin[1] - cy) * scale + oy,
                        m.reflectivity,
                        if settings.battle { n as f32 + 1.0 } else { 0.0 },
                    ],
                    uv: d.rect,
                    material: [
                        (m.roughness * settings.roughness).clamp(0.04, 1.0),
                        m.metalness,
                        (m.transmission * settings.transmission).clamp(0.0, 1.0),
                        m.emission * settings.emission * pose.emission,
                    ],
                    optical: [
                        if settings.reference {
                            pose.opacity.clamp(0.0, 1.0)
                        } else {
                            (m.opacity * settings.opacity * pose.opacity).clamp(0.0, 1.0)
                        },
                        m.thickness,
                        m.ior,
                        m.normal_strength * settings.normals,
                    ],
                    normal_matrix: [t.d / det, -t.c / det, -t.b / det, t.a / det],
                    world: [
                        entity.x,
                        entity.y,
                        entity.id as f32,
                        u32::from(entity.terrain) as f32,
                    ],
                });
            }
            if let Some(light) = world_light {
                self.emitters.push(light);
            }
        }
        if settings.world {
            let wanted = (settings.lights as usize)
                .min(crate::world::MAX_LIGHTS)
                .min(self.emitters.len());
            if wanted > 0 {
                let available = self.emitters.len();
                for i in 0..wanted {
                    self.emitters[i] = self.emitters[i * available / wanted];
                }
            }
            self.emitters.truncate(wanted);
        }
        self.world_entities = entities;
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub basis: [f32; 4],
    pub placement: [f32; 4],
    pub uv: [f32; 4],
    pub material: [f32; 4],
    pub optical: [f32; 4],
    pub normal_matrix: [f32; 4],
    pub world: [f32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Original artwork at the same pose: neutral backdrop and no material/display effects.
    pub reference: bool,
    pub battle: bool,
    pub world: bool,
    pub showcase: u32,
    pub showcase_motion: f32,
    pub lights: u32,
    pub reflection_steps: u32,
    pub world_size: f32,
    pub asset: usize,
    pub view: usize,
    pub animation: usize,
    /// Resolve each non-world view independently; missing names fall back to Idle, then 0.
    pub animation_name: Option<String>,
    pub count: u32,
    pub mixed: bool,
    pub phased: bool,
    pub overlap: bool,
    pub zoom: f32,
    pub roughness: f32,
    pub transmission: f32,
    pub emission: f32,
    pub opacity: f32,
    pub normals: f32,
    pub reflectivity: f32,
    pub ambient: f32,
    pub light_power: f32,
    pub light: [f32; 3],
    pub exposure: f32,
    pub mode: u32,
    pub background: u32,
    pub probe: [f32; 2],
    pub probe_front: bool,
    pub probe_enabled: bool,
    pub pixel_timing: bool,
    pub bloom: f32,
    pub spill: f32,
    pub partner: usize,
    pub source_depth: f32,
    pub source_offset: f32,
    pub source_trainer: bool,
    pub trainers: bool,
    pub reflection_focus: bool,
    pub rain: f32,
    pub wind: f32,
    pub wetness: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            reference: false,
            battle: false,
            world: false,
            showcase: 0,
            showcase_motion: 0.5,
            lights: 32,
            reflection_steps: 16,
            world_size: 48.0,
            asset: 0,
            view: 0,
            animation: 0,
            animation_name: None,
            count: 1,
            mixed: false,
            phased: true,
            overlap: false,
            zoom: 1.0,
            roughness: 1.0,
            transmission: 1.0,
            emission: 1.0,
            opacity: 1.0,
            normals: 1.0,
            reflectivity: 1.0,
            ambient: 0.65,
            light_power: 1.1,
            light: [0.3, 0.2, 0.6],
            exposure: 1.0,
            mode: 0,
            background: 0,
            probe: [0.58, 0.5],
            probe_front: false,
            probe_enabled: true,
            pixel_timing: true,
            bloom: 0.35,
            spill: 1.0,
            partner: 0,
            source_depth: 0.9,
            source_offset: 0.0,
            source_trainer: false,
            trainers: true,
            reflection_focus: true,
            rain: 0.7,
            wind: 0.25,
            wetness: 0.8,
        }
    }
}
