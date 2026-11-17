//! Older files: Smeargle's version 0, the `.sprite` prototype and version 1.
//! All three are ZIP archives of `sprite.json` and PNGs. They are read here,
//! upgraded to version 1 in memory, and turned into the current format by
//! [`crate::convert`]. `reference` draws version 1's own picture, which the
//! converter is checked against.
pub mod format;
pub mod migrate;
pub mod reference;
pub mod timing;
