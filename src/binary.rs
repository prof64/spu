//! Reading and writing `.spu` files.
//!
//! A file is a 16-byte header and then chunks. Everything is little-endian.
//! A chunk is a 16-byte header (type, length of the data as stored, CRC-32 of
//! type and stored data, and flags), the data, and zero padding to a multiple
//! of 8 bytes, so every chunk's data starts on an 8-byte boundary. Flag 1
//! means the data is stored compressed, as a zlib stream.
use crate::sprite::*;
use anyhow::{Context, Result, bail, ensure};

/// The first 8 bytes of every file. Like PNG's, they catch a file that was
/// damaged by being handled as text.
pub const MAGIC: [u8; 8] = [0x89, b'S', b'P', b'U', 0x0D, 0x0A, 0x1A, 0x0A];
/// The version number in the header of every file this crate reads and writes.
pub const VERSION: u16 = 2;
const NONE: u16 = u16::MAX;

/// CRC-32 as used by PNG and ZIP.
pub fn crc32(parts: &[&[u8]]) -> u32 {
    let mut crc = !0u32;
    for part in parts {
        for &byte in *part {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
    }
    !crc
}

#[derive(Default)]
struct Out(Vec<u8>);
impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend(v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend(v.to_le_bytes());
    }
    fn index(&mut self, v: Option<u16>) {
        self.u16(v.unwrap_or(NONE));
    }
    fn count(&mut self, n: usize) -> Result<()> {
        self.u16(u16::try_from(n).ok().filter(|&n| n != NONE).context("Too many items to store")?);
        Ok(())
    }
    fn text(&mut self, s: &str) -> Result<()> {
        self.u16(u16::try_from(s.len()).ok().context("A name is longer than 65,535 bytes")?);
        self.0.extend(s.as_bytes());
        Ok(())
    }
    fn align(&mut self, to: usize) {
        while !self.0.len().is_multiple_of(to) {
            self.0.push(0);
        }
    }
}

struct In<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> In<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        ensure!(n <= self.data.len() - self.at, "A chunk ends before its contents do");
        self.at += n;
        Ok(&self.data[self.at - n..self.at])
    }
    fn rest(&mut self) -> &'a [u8] {
        let rest = &self.data[self.at..];
        self.at = self.data.len();
        rest
    }
    fn done(&self) -> Result<()> {
        ensure!(self.at == self.data.len(), "A chunk is longer than its contents");
        Ok(())
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn index(&mut self) -> Result<Option<u16>> {
        Ok(Some(self.u16()?).filter(|&v| v != NONE))
    }
    fn flag(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => bail!("A yes-or-no value is neither 0 nor 1"),
        }
    }
    fn text(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        Ok(std::str::from_utf8(self.take(n)?).context("A name is not UTF-8")?.to_owned())
    }
    fn align(&mut self, to: usize) -> Result<()> {
        while !self.at.is_multiple_of(to) {
            ensure!(self.u8()? == 0, "Padding must be zero");
        }
        Ok(())
    }
}

/// Flag on a chunk: its data is stored as a zlib stream.
const COMPRESSED: u32 = 1;

fn chunk(file: &mut Vec<u8>, kind: &[u8; 4], data: &[u8], compress: bool) {
    // Sheets are PNG streams and already compressed; anything else is
    // stored compressed when that is smaller.
    let packed = (compress && !matches!(kind, b"SCOL" | b"SLIT") && data.len() >= 64).then(|| miniz_oxide::deflate::compress_to_vec_zlib(data, 9)).filter(|p| p.len() < data.len());
    let (data, flags) = match &packed {
        Some(p) => (p.as_slice(), COMPRESSED),
        None => (data, 0),
    };
    file.extend(kind);
    file.extend((data.len() as u32).to_le_bytes());
    file.extend(crc32(&[kind, data]).to_le_bytes());
    file.extend(flags.to_le_bytes());
    file.extend(data);
    while !file.len().is_multiple_of(8) {
        file.push(0);
    }
}

/// A sheet's pixels as a PNG stream.
fn png(width: u32, height: u32, color: png::ColorType, depth: png::BitDepth, bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(color);
    encoder.set_depth(depth);
    encoder.set_compression(png::Compression::Best);
    encoder.write_header()?.write_image_data(bytes)?;
    Ok(out)
}
fn unpng(bytes: &[u8], color: png::ColorType, depth: png::BitDepth, what: &str) -> Result<(u32, u32, Vec<u8>)> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().with_context(|| format!("The {what} sheet is not a PNG"))?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    ensure!(info.color_type == color && info.bit_depth == depth && !info.interlaced, "The {what} sheet is the wrong kind of PNG");
    ensure!(width <= MAX_SHEET && height <= MAX_SHEET, "A sheet is at most {MAX_SHEET} pixels a side");
    let mut data = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut data)?;
    data.truncate(frame.buffer_size());
    Ok((width, height, data))
}

/// The bytes of a `.spu` file. The sprite must be valid.
pub fn write(s: &Sprite) -> Result<Vec<u8>> {
    s.validate()?;
    encode(s, true)
}
/// [`write`] without checking the sprite, for making files that break a rule.
pub fn write_unchecked(s: &Sprite) -> Result<Vec<u8>> {
    encode(s, true)
}
/// [`write`] with no chunk stored compressed.
pub fn write_stored(s: &Sprite) -> Result<Vec<u8>> {
    s.validate()?;
    encode(s, false)
}

fn encode(s: &Sprite, compress: bool) -> Result<Vec<u8>> {
    let chunk = |file: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]| chunk(file, kind, data, compress);
    let mut file = Vec::new();
    file.extend(MAGIC);
    file.extend(VERSION.to_le_bytes());
    file.extend([0; 2]);
    file.extend([0; 4]); // the file's length, filled in at the end

    let mut o = Out::default();
    o.text(&s.name)?;
    o.text(&s.author)?;
    o.u32(s.version);
    o.f32(s.ticks_per_second);
    for n in [s.views.len(), s.parts.len(), s.drawings.len(), s.palettes.len(), s.materials.len(), s.variants.len(), s.animations.len()] {
        o.count(n)?;
    }
    chunk(&mut file, b"HEAD", &o.0);

    let mut o = Out::default();
    for v in &s.views {
        o.text(&v.id)?;
        o.text(&v.name)?;
        o.i32(v.x);
        o.i32(v.y);
        o.u32(v.width);
        o.u32(v.height);
        o.count(v.parts.len())?;
        for p in &v.parts {
            o.u16(p.part);
            o.index(p.parent);
        }
    }
    chunk(&mut file, b"VIEW", &o.0);

    let mut o = Out::default();
    for p in &s.parts {
        o.text(&p.id)?;
        o.text(&p.name)?;
        o.u8(p.extra as u8);
        o.f32(p.depth);
    }
    chunk(&mut file, b"PART", &o.0);

    let mut o = Out::default();
    for d in &s.drawings {
        o.text(&d.id)?;
        o.text(&d.name)?;
        for place in &d.places {
            o.u8(place.is_some() as u8);
            if let Some(p) = place {
                for v in [p.x, p.y, p.width, p.height] {
                    o.u16(v);
                }
                o.f32(p.offset_x);
                o.f32(p.offset_y);
            }
        }
    }
    chunk(&mut file, b"DRAW", &o.0);

    let mut o = Out::default();
    o.u32(s.palettes.first().map_or(0, |p| p.colors.len()) as u32);
    for p in &s.palettes {
        o.text(&p.id)?;
        o.text(&p.name)?;
        for c in &p.colors {
            o.0.extend(c);
        }
    }
    chunk(&mut file, b"PALS", &o.0);

    let mut o = Out::default();
    for m in &s.materials {
        o.text(&m.name)?;
        o.f32(m.finish);
        o.f32(m.body);
        o.u16(m.base);
        o.count(m.shades.len())?;
        for &c in &m.shades {
            o.u16(c);
        }
    }
    chunk(&mut file, b"MATS", &o.0);

    let mut o = Out::default();
    for v in &s.variants {
        o.text(&v.id)?;
        o.text(&v.name)?;
        o.index(v.palette);
        o.count(v.replace.len())?;
        for r in &v.replace {
            o.u16(r.view);
            o.u16(r.from);
            o.u16(r.to);
        }
        for list in [&v.show, &v.hide] {
            o.count(list.len())?;
            for &p in list {
                o.u16(p);
            }
        }
    }
    chunk(&mut file, b"VARS", &o.0);

    for a in &s.animations {
        let mut o = Out::default();
        o.text(&a.id)?;
        o.text(&a.name)?;
        for motion in &a.views {
            o.u8(motion.is_some() as u8);
            let Some(m) = motion else { continue };
            o.u32(m.length);
            o.u8(m.looping as u8);
            o.count(m.tracks.len())?;
            for t in &m.tracks {
                o.u16(t.part);
                o.u32(t.image.len() as u32);
                o.align(4);
                for (tick, _) in &t.image {
                    o.u32(*tick);
                }
                for (_, drawing) in &t.image {
                    o.index(*drawing);
                }
                for keys in &t.numbers {
                    o.align(4);
                    o.u32(keys.len() as u32);
                    for k in keys {
                        o.u32(k.tick);
                    }
                    o.align(8);
                    for k in keys {
                        o.f64(k.value);
                    }
                    for k in keys {
                        o.u8(k.ease as u8);
                    }
                }
            }
        }
        chunk(&mut file, b"ANIM", &o.0);
    }

    for (i, v) in s.views.iter().enumerate() {
        let c = &v.color;
        let mut pixels = Vec::with_capacity(c.number.len() * 4);
        for (&number, &opacity) in c.number.iter().zip(&c.opacity) {
            pixels.extend(number.to_be_bytes());
            pixels.extend((opacity as u16 * 257).to_be_bytes());
        }
        let mut o = Out::default();
        o.u16(i as u16);
        o.0.extend(png(c.width, c.height, png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen, &pixels)?);
        chunk(&mut file, b"SCOL", &o.0);
    }
    for (i, v) in s.views.iter().enumerate() {
        let Some(l) = &v.light else { continue };
        let mut o = Out::default();
        o.u16(i as u16);
        o.0.extend(png(l.width, l.height, png::ColorType::Rgba, png::BitDepth::Eight, &l.data)?);
        chunk(&mut file, b"SLIT", &o.0);
    }
    for e in &s.extras {
        let mut o = Out::default();
        o.text(&e.path)?;
        o.0.extend(&e.bytes);
        chunk(&mut file, b"xtra", &o.0);
    }
    for (kind, data) in &s.unknown {
        chunk(&mut file, kind, data);
    }
    chunk(&mut file, b"IEND", &[]);

    ensure!(file.len() <= MAX_FILE, "A file is at most 64 MiB");
    let length = file.len() as u32;
    file[12..16].copy_from_slice(&length.to_le_bytes());
    Ok(file)
}

/// Read a `.spu` file and check every rule.
pub fn read(file: &[u8]) -> Result<Sprite> {
    ensure!(file.len() <= MAX_FILE, "A file is at most 64 MiB");
    ensure!(file.len() >= 16 && file[..8] == MAGIC, "Not an SPU file");
    let version = u16::from_le_bytes([file[8], file[9]]);
    ensure!(version == VERSION, "The header's version is {version}; this reader reads version {VERSION}");
    ensure!(file[10..12] == [0, 0], "Unknown header flags");
    ensure!(u32::from_le_bytes(file[12..16].try_into().unwrap()) as usize == file.len(), "The file is not the length its header says");

    // Split into chunks, checking each one's checksum.
    let mut chunks: Vec<([u8; 4], std::borrow::Cow<[u8]>)> = Vec::new();
    let mut unpacked = 0usize;
    let mut at = 16;
    loop {
        ensure!(file.len() - at >= 16, "The file ends without its last chunk");
        let kind: [u8; 4] = file[at..at + 4].try_into().unwrap();
        let length = u32::from_le_bytes(file[at + 4..at + 8].try_into().unwrap()) as usize;
        let sum = u32::from_le_bytes(file[at + 8..at + 12].try_into().unwrap());
        let flags = u32::from_le_bytes(file[at + 12..at + 16].try_into().unwrap());
        ensure!(flags & !COMPRESSED == 0, "Unknown chunk flags");
        let name = String::from_utf8_lossy(&kind).into_owned();
        ensure!(kind.iter().all(u8::is_ascii_alphabetic), "A chunk's type is not four letters");
        ensure!(length <= file.len() - at - 16, "Chunk {name} runs past the end of the file");
        let data = &file[at + 16..at + 16 + length];
        ensure!(crc32(&[&kind, data]) == sum, "Chunk {name} is damaged: its checksum does not match");
        at += 16 + length;
        while at % 8 != 0 {
            ensure!(at < file.len() && file[at] == 0, "Padding must be zero");
            at += 1;
        }
        if &kind == b"IEND" {
            ensure!(length == 0 && at == file.len(), "Nothing may follow the last chunk");
            break;
        }
        let data = if flags & COMPRESSED != 0 {
            let limit = MAX_FILE - unpacked;
            std::borrow::Cow::Owned(miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(data, limit).ok().with_context(|| format!("Chunk {name} does not unpack"))?)
        } else {
            std::borrow::Cow::Borrowed(data)
        };
        unpacked += data.len();
        ensure!(unpacked <= MAX_FILE, "The file unpacks to more than 64 MiB");
        chunks.push((kind, data));
    }

    let mut s = Sprite::default();
    let mut counts = [0usize; 7];
    let mut order = 0; // the fixed chunks must come once each, in this order
    const FIXED: [&[u8; 4]; 7] = [b"HEAD", b"VIEW", b"PART", b"DRAW", b"PALS", b"MATS", b"VARS"];
    let mut sheets = 0;
    for (kind, data) in chunks {
        let name = String::from_utf8_lossy(&kind).into_owned();
        let data: &[u8] = &data;
        let mut i = In { data, at: 0 };
        if let Some(position) = FIXED.iter().position(|k| **k == kind) {
            ensure!(position == order, "Chunk {name} is missing, repeated or out of order");
            order += 1;
        } else {
            ensure!(order == FIXED.len(), "Chunk {name} comes before the sprite is described");
        }
        match &kind {
            b"HEAD" => {
                s.name = i.text()?;
                s.author = i.text()?;
                s.version = i.u32()?;
                s.ticks_per_second = i.f32()?;
                for c in &mut counts {
                    *c = i.u16()? as usize;
                }
                ensure!(counts[0] <= MAX_VIEWS && counts[1] <= MAX_PARTS && counts[2] <= MAX_DRAWINGS && counts[4] <= MAX_MATERIALS, "The sprite is over a limit");
            }
            b"VIEW" => {
                for _ in 0..counts[0] {
                    let mut v = View { id: i.text()?, name: i.text()?, x: i.i32()?, y: i.i32()?, width: i.u32()?, height: i.u32()?, ..View::default() };
                    for _ in 0..i.u16()? {
                        v.parts.push(ViewPart { part: i.u16()?, parent: i.index()? });
                    }
                    s.views.push(v);
                }
            }
            b"PART" => {
                for _ in 0..counts[1] {
                    s.parts.push(Part { id: i.text()?, name: i.text()?, extra: i.flag()?, depth: i.f32()? });
                }
            }
            b"DRAW" => {
                for _ in 0..counts[2] {
                    let mut d = Drawing { id: i.text()?, name: i.text()?, places: Vec::new() };
                    for _ in 0..counts[0] {
                        d.places.push(if i.flag()? {
                            Some(Place { x: i.u16()?, y: i.u16()?, width: i.u16()?, height: i.u16()?, offset_x: i.f32()?, offset_y: i.f32()? })
                        } else {
                            None
                        });
                    }
                    s.drawings.push(d);
                }
            }
            b"PALS" => {
                let colors = i.u32()? as usize;
                ensure!((1..=MAX_COLORS).contains(&colors), "A palette has 1 to {MAX_COLORS} colors");
                for _ in 0..counts[3] {
                    let (id, name) = (i.text()?, i.text()?);
                    let colors = i.take(colors * 4)?.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect();
                    s.palettes.push(Palette { id, name, colors });
                }
            }
            b"MATS" => {
                for _ in 0..counts[4] {
                    let mut m = Material { name: i.text()?, finish: i.f32()?, body: i.f32()?, base: i.u16()?, shades: Vec::new() };
                    for _ in 0..i.u16()? {
                        m.shades.push(i.u16()?);
                    }
                    s.materials.push(m);
                }
            }
            b"VARS" => {
                for _ in 0..counts[5] {
                    let mut v = Variant { id: i.text()?, name: i.text()?, palette: i.index()?, ..Variant::default() };
                    for _ in 0..i.u16()? {
                        v.replace.push(Replace { view: i.u16()?, from: i.u16()?, to: i.u16()? });
                    }
                    for _ in 0..i.u16()? {
                        v.show.push(i.u16()?);
                    }
                    for _ in 0..i.u16()? {
                        v.hide.push(i.u16()?);
                    }
                    s.variants.push(v);
                }
            }
            b"ANIM" => {
                ensure!(sheets == 0, "Animations come before the sheets");
                let mut a = Animation { id: i.text()?, name: i.text()?, views: Vec::new() };
                for _ in 0..counts[0] {
                    if !i.flag()? {
                        a.views.push(None);
                        continue;
                    }
                    let mut m = Motion { length: i.u32()?, looping: i.flag()?, tracks: Vec::new() };
                    for _ in 0..i.u16()? {
                        let mut t = Track { part: i.u16()?, ..Track::default() };
                        let n = i.u32()? as usize;
                        ensure!(n <= data.len(), "A track has more keys than fit in its chunk");
                        i.align(4)?;
                        let ticks = (0..n).map(|_| i.u32()).collect::<Result<Vec<_>>>()?;
                        for tick in ticks {
                            t.image.push((tick, i.index()?));
                        }
                        for keys in &mut t.numbers {
                            i.align(4)?;
                            let n = i.u32()? as usize;
                            ensure!(n <= data.len(), "A track has more keys than fit in its chunk");
                            let ticks = (0..n).map(|_| i.u32()).collect::<Result<Vec<_>>>()?;
                            i.align(8)?;
                            let values = (0..n).map(|_| i.f64()).collect::<Result<Vec<_>>>()?;
                            for k in 0..n {
                                let ease = *Ease::ALL.get(i.u8()? as usize).context("Unknown ease")?;
                                keys.push(Key { tick: ticks[k], value: values[k], ease });
                            }
                        }
                        m.tracks.push(t);
                    }
                    a.views.push(Some(m));
                }
                s.animations.push(a);
            }
            b"SCOL" => {
                ensure!(s.animations.len() == counts[6], "The file has fewer animations than it says");
                let v = i.u16()? as usize;
                ensure!(v == sheets && v < s.views.len(), "Color sheets must come one per view, in order");
                sheets += 1;
                let (width, height, bytes) = unpng(i.rest(), png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen, "Color")?;
                let mut c = ColorSheet { width, height, ..ColorSheet::default() };
                for p in bytes.chunks_exact(4) {
                    c.number.push(u16::from_be_bytes([p[0], p[1]]));
                    ensure!(p[2] == p[3], "An opacity is not one of the 256 allowed values");
                    c.opacity.push(p[2]);
                }
                s.views[v].color = c;
            }
            b"SLIT" => {
                ensure!(sheets == s.views.len(), "Light sheets come after every Color sheet");
                let v = i.u16()? as usize;
                ensure!(v < s.views.len() && s.views[v].light.is_none(), "A Light sheet is for a view that does not exist or already has one");
                let (width, height, data) = unpng(i.rest(), png::ColorType::Rgba, png::BitDepth::Eight, "Light")?;
                s.views[v].light = Some(LightSheet { width, height, data });
            }
            b"xtra" => s.extras.push(Extra { path: i.text()?, bytes: i.rest().to_vec() }),
            _ if kind[0].is_ascii_lowercase() => {
                s.unknown.push((kind, i.rest().to_vec()));
            }
            _ => bail!("This file needs something this reader does not know: chunk {name}"),
        }
        i.done().with_context(|| format!("Chunk {name}"))?;
    }
    ensure!(order == FIXED.len(), "The file is missing a chunk every sprite has");
    ensure!(s.animations.len() == counts[6], "The file has fewer animations than it says");
    ensure!(sheets == s.views.len(), "Every view needs a Color sheet");
    s.validate()?;
    Ok(s)
}
