//! How a value is read from keys at a tick, and how joined parts combine.
use super::format::{Ease, Key, Track};

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
