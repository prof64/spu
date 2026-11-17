//! SPU (Sprite Universal): a file format for animated 2D sprites.
//!
//! This crate is the format's reference implementation. Where the
//! specification (`docs/spec.md`) and this code disagree, one of them has a
//! bug; the conformance suite (`conformance/`) says which.
//!
//! - [`sprite`]: what a sprite holds, and the rules a file must keep.
//! - [`binary`]: reading and writing `.spu` files.
//! - [`picture`]: the exact lighting-free picture of a sprite at a tick.
//! - [`convert`]: turning older files into the current format.
//! - [`legacy`]: reading those older files.
pub mod binary;
pub mod convert;
pub mod legacy;
pub mod picture;
pub mod sprite;

use anyhow::{Context, Result};

/// Open any `.spu` file. Older files are upgraded in memory; the second
/// value names what the file was upgraded from.
pub fn open(bytes: &[u8]) -> Result<(sprite::Sprite, Option<String>)> {
    if bytes.starts_with(&binary::MAGIC) {
        return Ok((binary::read(bytes)?, None));
    }
    let package = legacy::format::Package::read(bytes).context("Not an SPU file")?;
    let from = package.migrated_from.clone().unwrap_or_else(|| "version 1".into());
    Ok((convert::from_legacy(&package, None, true)?, Some(from)))
}
