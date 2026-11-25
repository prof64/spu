//! SPU: a file format for animated 2D sprites.
//!
//! This crate is the format's reference implementation. Where the
//! specification (`docs/spec.md`) and this code disagree, one of them has a
//! bug; the conformance suite (`conformance/`) says which.
//!
//! - [`sprite`]: what a sprite holds, and the rules a file must keep.
//! - [`binary`]: reading and writing `.spu` files.
//! - [`picture`]: the exact lighting-free picture of a sprite at a tick.
pub mod binary;
pub mod picture;
pub mod sprite;
