//! Deterministic scene workload and spatial light lists, shared by WASM and native runs.
use crate::runtime::{Asset, Emitter, Settings};

pub const TILE_SIZE: u32 = 64;
pub const LIGHTS_PER_TILE: usize = 8;
pub const MAX_LIGHTS: usize = 512;
#[derive(Clone, Copy, Default)]
pub struct Entity {
    pub asset: usize,
    pub view: usize,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub id: u32,
    pub terrain: bool,
}

fn showcase_only(a: &Asset) -> bool {
    a.package
        .document
        .extra
        .get("showcaseOnly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn role(a: &Asset) -> &str {
    a.package
        .document
        .extra
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

pub fn populate(
    assets: &[Asset],
    s: &Settings,
    time: f32,
    w: u32,
    h: u32,
    out: &mut Vec<Entity>,
) -> usize {
    out.clear();
    if s.showcase > 0 {
        return showcase(assets, s, time, w, h, out);
    }
    let terrain = assets
        .iter()
        .position(|a| role(a) == "terrain" && !showcase_only(a));
    if let Some(asset) = terrain {
        let size = 48.0;
        for y in 0..(h as f32 / size).ceil() as u32 {
            for x in 0..(w as f32 / size).ceil() as u32 {
                out.push(Entity {
                    asset,
                    x: x as f32 * size + size * 0.5,
                    y: y as f32 * size + size,
                    size,
                    terrain: true,
                    ..Default::default()
                });
            }
        }
    }
    let tiles = out.len();
    let mut choices: Vec<_> = assets
        .iter()
        .enumerate()
        .filter(|(_, a)| role(a) != "terrain" && !showcase_only(a))
        .map(|(i, _)| i)
        .collect();
    if choices.is_empty() {
        choices = (0..assets.len()).filter(|i| Some(*i) != terrain).collect();
    }
    if choices.is_empty() {
        return tiles;
    }
    let size = s.world_size.clamp(16.0, 160.0);
    for n in 0..s.count.clamp(1, 10000) {
        let asset = choices[n as usize % choices.len()];
        let a = &assets[asset];
        let moving = role(a) != "prop";
        let phase = n as f32 * 2.399963;
        let t = time * 0.65 + phase;
        let margin = size * 0.6;
        let mut x =
            margin + ((n as f32 * 0.618034 + 0.13).fract()) * (w as f32 - 2.0 * margin).max(1.0);
        let mut y =
            size + ((n as f32 * 0.754877 + 0.21).fract()) * (h as f32 - size - margin).max(1.0);
        if moving {
            x += (t.sin()) * size * 0.23;
            y += (t * 0.73).cos() * size * 0.13;
        }
        let angle = (-0.73 * (t * 0.73).sin()).atan2(t.cos());
        let direction =
            ((angle / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8) as usize;
        let names = [
            "East",
            "SouthEast",
            "South",
            "SouthWest",
            "West",
            "NorthWest",
            "North",
            "NorthEast",
        ];
        let view = a
            .package
            .document
            .views
            .iter()
            .position(|v| v.name.eq_ignore_ascii_case(names[direction]))
            .unwrap_or(0);
        out.push(Entity {
            asset,
            view,
            x,
            y,
            size,
            id: n + 1,
            terrain: false,
        });
    }
    out[tiles..].sort_by(|a, b| a.y.total_cmp(&b.y).then(a.id.cmp(&b.id)));
    tiles
}

/// Carefully arranged demonstrations share the exact same material/animation path as World.
fn showcase(
    assets: &[Asset],
    s: &Settings,
    time: f32,
    w: u32,
    h: u32,
    out: &mut Vec<Entity>,
) -> usize {
    let find = |words: &[&str], fallback: usize| -> usize {
        words
            .iter()
            .find_map(|word| {
                assets
                    .iter()
                    .position(|a| a.package.document.name.to_lowercase().contains(word))
            })
            .unwrap_or(fallback.min(assets.len().saturating_sub(1)))
    };
    let fire = find(&["cindermaw", "salamander", "furnace", "emberwing"], 0);
    let ice = find(
        &[
            "prismcoil",
            "glass serpent",
            "ice guardian",
            "glass guardian",
            "crystal",
            "glass vial",
        ],
        0,
    );
    let metal = find(&["mirror knight", "tin beetle"], 0);
    let scout = find(&["dungeon scout", "moss scout"], 0);
    let lamp = find(&["lantern"], 0);
    let hall = assets
        .iter()
        .position(|a| a.package.document.name.to_lowercase().contains("obsidian"));
    let tiles = if let Some(asset) = hall {
        let v = &assets[asset].views[0];
        let height = (h as f32).max(w as f32 * v.canvas.height / v.canvas.width);
        out.push(Entity {
            asset,
            view: 0,
            x: w as f32 * 0.5,
            y: (h as f32 + height) * 0.5,
            size: height,
            id: 0,
            terrain: true,
        });
        1
    } else {
        0
    };
    let motion = s.showcase_motion.clamp(0.0, 1.0);
    let unit = (h as f32).min(w as f32 * 0.68);
    let mut add = |asset: usize, x: f32, y: f32, size: f32, id: u32, view: usize| {
        out.push(Entity {
            asset,
            view: view.min(assets[asset].views.len() - 1),
            x: x * w as f32,
            y: y * h as f32,
            size: size * unit,
            id,
            terrain: false,
        });
    };
    match s.showcase {
        1 => {
            add(fire, 0.20 + (motion - 0.5) * 0.12, 0.66, 0.60, 1, 0);
            add(lamp, 0.52, 0.85, 0.20, 2, 0);
            add(metal, 0.82, 0.66, 0.45, 3, 0);
        }
        2 => {
            // Near fire / far shield gives reflected rays a genuine depth crossing.
            add(metal, 0.82, 0.65, 0.60, 2, 0);
            add(fire, 0.20 + (motion - 0.5) * 0.20, 0.68, 0.60, 1, 0);
        }
        3 => {
            // Opaque moving subject is captured before the transmitting guardian.
            add(fire, 0.25 + motion * 0.50, 0.71, 0.48, 1, 0);
            add(lamp, 0.77, 0.68, 0.24, 3, 0);
            add(ice, 0.54, 0.88, 0.82, 2, 0);
        }
        _ => {
            let angle = time * 0.42 + motion * std::f32::consts::TAU;
            let x = 0.5 + angle.cos() * 0.19;
            let y = 0.70 + angle.sin() * 0.09;
            let direction = (((-angle.cos()).atan2(-angle.sin()) / std::f32::consts::FRAC_PI_4)
                .round() as i32)
                .rem_euclid(8) as usize;
            let names = [
                "East",
                "SouthEast",
                "South",
                "SouthWest",
                "West",
                "NorthWest",
                "North",
                "NorthEast",
            ];
            let view = assets[scout]
                .package
                .document
                .views
                .iter()
                .position(|v| v.name.eq_ignore_ascii_case(names[direction]))
                .unwrap_or(0);
            add(lamp, 0.23, 0.62, 0.22, 2, 0);
            add(lamp, 0.80, 0.82, 0.20, 3, 0);
            add(scout, x, y, 0.34, 1, view);
            add(ice, 0.77, 0.50, 0.31, 4, 0);
        }
    }
    out[tiles..].sort_by(|a, b| a.y.total_cmp(&b.y).then(a.id.cmp(&b.id)));
    tiles
}

/// Each 64px tile retains its eight strongest nearby lights. Bounded shading cost.
pub fn light_grid(lights: &[Emitter], w: u32, h: u32, out: &mut Vec<u32>) {
    let cols = w.div_ceil(TILE_SIZE);
    let rows = h.div_ceil(TILE_SIZE);
    out.clear();
    out.resize((cols * rows) as usize * (LIGHTS_PER_TILE + 1), 0);
    let mut ranks = vec![[0.0f32; LIGHTS_PER_TILE]; (cols * rows) as usize];
    for (id, e) in lights.iter().take(MAX_LIGHTS).enumerate() {
        let x = e.position[0] * w as f32;
        let y = e.position[1] * h as f32;
        let radius = e.position[3] * h as f32 * 2.0;
        let left = ((x - radius) / TILE_SIZE as f32).floor().max(0.0) as u32;
        let right = ((x + radius) / TILE_SIZE as f32)
            .floor()
            .max(0.0)
            .min(cols.saturating_sub(1) as f32) as u32;
        let top = ((y - radius) / TILE_SIZE as f32).floor().max(0.0) as u32;
        let bottom = ((y + radius) / TILE_SIZE as f32)
            .floor()
            .max(0.0)
            .min(rows.saturating_sub(1) as f32) as u32;
        for ty in top..=bottom {
            for tx in left..=right {
                if tx >= cols || ty >= rows {
                    continue;
                }
                let tile = (ty * cols + tx) as usize;
                let base = tile * (LIGHTS_PER_TILE + 1);
                let dx = (tx as f32 + 0.5) * TILE_SIZE as f32 - x;
                let dy = (ty as f32 + 0.5) * TILE_SIZE as f32 - y;
                let score = (e.color[0] + e.color[1] + e.color[2]) * radius * radius
                    / (dx * dx + dy * dy + radius * radius);
                let count = out[base] as usize;
                let slot = if count < LIGHTS_PER_TILE {
                    out[base] += 1;
                    count
                } else {
                    let weakest = (0..LIGHTS_PER_TILE)
                        .min_by(|&a, &b| ranks[tile][a].total_cmp(&ranks[tile][b]))
                        .unwrap();
                    if ranks[tile][weakest] >= score {
                        continue;
                    }
                    weakest
                };
                ranks[tile][slot] = score;
                out[base + 1 + slot] = id as u32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_sprites_without_a_role_participate_in_the_world() {
        use crate::{format::Package, runtime::Scene};
        let scene = Scene::load(vec![
            Package::read(include_bytes!("../web/examples/moss-stone.sprite")).unwrap(),
            Package::read(include_bytes!("../web/examples/copperbot.sprite")).unwrap(),
        ])
        .unwrap();
        let mut entities = Vec::new();
        let settings = Settings {
            count: 7,
            ..Default::default()
        };
        let tiles = populate(&scene.assets, &settings, 0.0, 640, 480, &mut entities);
        assert_eq!(tiles, 140);
        assert!(entities[..tiles].iter().all(|e| e.terrain && e.asset == 0));
        assert_eq!(entities.len() - tiles, 7);
        assert!(entities[tiles..].iter().all(|e| !e.terrain && e.asset == 1));
        let mut ids: Vec<_> = entities[tiles..].iter().map(|e| e.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, (1..=7).collect::<Vec<_>>());
    }

    #[test]
    fn spatial_lists_are_bounded_and_clear_after_lights_disappear() {
        let mut lights = Vec::new();
        for i in 0..20 {
            lights.push(Emitter {
                position: [0.15, 0.2, 0.0, 0.1],
                color: [i as f32 + 1.0, 0.0, 0.0, 0.0],
            });
        }
        let mut grid = Vec::new();
        light_grid(&lights, 640, 480, &mut grid);
        let first = &grid[..9];
        assert_eq!(first[0], 8);
        assert!(
            first[1..].iter().all(|&i| i >= 12 && i < 20),
            "Keep stronger lights when a tile overflows"
        );
        assert!(grid.chunks_exact(9).all(|tile| tile[0] <= 8));
        assert_eq!(
            grid[(7 * 10 + 9) * 9],
            0,
            "Distant tiles should not receive local lights"
        );
        light_grid(&[], 640, 480, &mut grid);
        assert!(grid.iter().all(|&v| v == 0));
    }
}
