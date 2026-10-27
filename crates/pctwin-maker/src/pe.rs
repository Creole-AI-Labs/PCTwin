//! A Windows program's embedded signature (Authenticode), read in pure Rust and streamed, so a
//! large installer is never held in memory.
//!
//! The digest is worked out exactly as the `authenticode` crate does it (and as Microsoft's
//! Authenticode format describes): the whole file except the header checksum, the pointer to the
//! certificate table and the certificate table itself, sections in file order. The tests check
//! this against the crate on real Windows files.
//!
//! A program signed through a Windows catalog (most of Windows' own programs) carries no embedded
//! signature; downloaded installers carry theirs inside, so only that kind is read.

use std::io::{self, Read, Seek, SeekFrom};

use crate::facts::WindowsSigner;
use crate::signer::{Hasher, read_signature};

/// The most header bytes read to find the sections.
const MAX_HEADER: u64 = 1024 * 1024;
/// The PE format's own limit on sections.
const MAX_SECTIONS: usize = 96;
/// Bytes hashed per read.
const CHUNK: usize = 64 * 1024;

/// What a Windows program's embedded signature says.
#[expect(
    clippy::large_enum_variant,
    reason = "made once per check; boxing buys nothing"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeSignature {
    /// There is no embedded signature.
    None,
    /// There is one, but it or the file is malformed: why.
    Broken(String),
    /// There is one. `digest_matches` says whether the file's bytes are the bytes signed.
    Signed {
        signer: Result<WindowsSigner, String>,
        digest_matches: bool,
    },
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// Where the parts the digest skips are.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Layout {
    check_sum: u64,
    security_dir: u64,
    after_header: u64,
    /// (start, size) of each section's bytes in the file, in file order.
    sections: Vec<(u64, u64)>,
    /// The certificate table, if the program has one.
    table: Option<(u64, u64)>,
}

fn layout(head: &[u8], size: u64) -> Result<Layout, String> {
    let bad = |what: &str| format!("the program's header is malformed ({what})");
    let pe = u32_at(head, 0x3C).ok_or_else(|| bad("too short"))? as usize;
    if head.get(pe..pe.saturating_add(4)) != Some(b"PE\0\0") {
        return Err(bad("no PE signature"));
    }
    let coff = pe + 4;
    let n_sections = usize::from(u16_at(head, coff + 2).ok_or_else(|| bad("too short"))?);
    let opt_size = usize::from(u16_at(head, coff + 16).ok_or_else(|| bad("too short"))?);
    let opt = coff + 20;
    let (dirs, count_at) = match u16_at(head, opt) {
        Some(0x10B) => (opt + 96, opt + 92),
        Some(0x20B) => (opt + 112, opt + 108),
        _ => return Err(bad("unknown optional header")),
    };
    let after_header = u64::from(u32_at(head, opt + 60).ok_or_else(|| bad("too short"))?);
    let dir_count = u32_at(head, count_at).ok_or_else(|| bad("too short"))?;
    let security_dir = dirs + 4 * 8;
    let table = if dir_count > 4 {
        let at = u64::from(u32_at(head, security_dir).ok_or_else(|| bad("too short"))?);
        let len = u64::from(u32_at(head, security_dir + 4).ok_or_else(|| bad("too short"))?);
        (at != 0 && len != 0).then_some((at, len))
    } else {
        None
    };
    if n_sections > MAX_SECTIONS {
        return Err(bad("too many sections"));
    }
    let table_start = opt + opt_size;
    let mut sections = Vec::with_capacity(n_sections);
    for i in 0..n_sections {
        let s = table_start + i * 40;
        let raw_size = u32_at(head, s + 16).ok_or_else(|| bad("section table cut short"))?;
        let raw_at = u32_at(head, s + 20).ok_or_else(|| bad("section table cut short"))?;
        sections.push((u64::from(raw_at), u64::from(raw_size)));
    }
    sections.sort_unstable_by_key(|s| s.0);
    let check_sum = (opt + 64) as u64;
    let security_dir = security_dir as u64;
    // The checksum always comes before the directories (fixed offsets), so only these can be wrong.
    if !(security_dir + 8 <= after_header && after_header <= size) {
        return Err(bad("header parts out of order"));
    }
    let mut total = after_header;
    for &(at, len) in &sections {
        if at.checked_add(len).is_none_or(|end| end > size) {
            return Err(bad("a section lies past the end of the file"));
        }
        total += len;
    }
    // Overlapping sections would make the digest read the same bytes again and again.
    if total > size {
        return Err(bad("sections overlap"));
    }
    if let Some((at, len)) = table
        && at.checked_add(len).is_none_or(|end| end > size)
    {
        return Err(bad("the certificate table lies past the end of the file"));
    }
    Ok(Layout {
        check_sum,
        security_dir,
        after_header,
        sections,
        table,
    })
}

fn hash_range<R: Read + Seek>(r: &mut R, h: &mut Hasher, start: u64, end: u64) -> io::Result<()> {
    r.seek(SeekFrom::Start(start))?;
    let mut left = end - start;
    let mut buf = vec![0u8; CHUNK];
    while left > 0 {
        let n = usize::try_from(left.min(CHUNK as u64)).unwrap_or(CHUNK);
        r.read_exact(&mut buf[..n])?;
        h.update(&buf[..n]);
        left -= n as u64;
    }
    Ok(())
}

/// Reads the embedded signature of a Windows program of `size` bytes and checks that the bytes
/// are the bytes signed. Never runs the program. Reads that fail are Err.
pub fn read_pe<R: Read + Seek>(r: &mut R, size: u64) -> io::Result<PeSignature> {
    let head_len = usize::try_from(size.min(MAX_HEADER)).unwrap_or(0);
    let mut head = vec![0u8; head_len];
    r.seek(SeekFrom::Start(0))?;
    r.read_exact(&mut head)?;
    let layout = match layout(&head, size) {
        Ok(l) => l,
        Err(why) => return Ok(PeSignature::Broken(why)),
    };
    let Some((table_at, table_len)) = layout.table else {
        return Ok(PeSignature::None);
    };
    if table_len > crate::signer::MAX_SIGNATURE as u64 {
        return Ok(PeSignature::Broken(
            "the certificate table is too large".into(),
        ));
    }
    let mut table = vec![0u8; table_len as usize];
    r.seek(SeekFrom::Start(table_at))?;
    r.read_exact(&mut table)?;
    // The first entry: length, revision 2.0, type "PKCS #7 SignedData".
    let entry_len = u32_at(&table, 0).unwrap_or(0) as usize;
    if entry_len < 8
        || entry_len > table.len()
        || u16_at(&table, 4) != Some(0x0200)
        || u16_at(&table, 6) != Some(0x0002)
    {
        return Ok(PeSignature::Broken(
            "the certificate table's first entry is not a signature".into(),
        ));
    }
    let signed = match read_signature(&table[8..entry_len]) {
        Ok(s) => s,
        Err(why) => return Ok(PeSignature::Broken(why)),
    };

    let mut h = Hasher::new(signed.alg);
    hash_range(r, &mut h, 0, layout.check_sum)?;
    hash_range(r, &mut h, layout.check_sum + 4, layout.security_dir)?;
    hash_range(r, &mut h, layout.security_dir + 8, layout.after_header)?;
    let mut hashed = layout.after_header;
    for &(at, len) in &layout.sections {
        hash_range(r, &mut h, at, at + len)?;
        hashed += len;
    }
    // Whatever follows the sections, less the certificate table, is signed too.
    let Some(extra) = (size - hashed).checked_sub(table_len) else {
        return Ok(PeSignature::Broken(
            "the certificate table overlaps the program".into(),
        ));
    };
    hash_range(r, &mut h, hashed, hashed + extra)?;
    let digest_matches = h.finish() == signed.signed_digest;
    Ok(PeSignature::Signed {
        signer: signed.signer,
        digest_matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal 64-bit PE: header, one section, an optional certificate table.
    pub(crate) fn tiny_pe(table: Option<&[u8]>) -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84;
        b[coff..coff + 2].copy_from_slice(&0x8664u16.to_le_bytes());
        b[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
        b[coff + 16..coff + 18].copy_from_slice(&240u16.to_le_bytes());
        let opt = coff + 20;
        b[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes());
        b[opt + 60..opt + 64].copy_from_slice(&0x200u32.to_le_bytes());
        b[opt + 108..opt + 112].copy_from_slice(&16u32.to_le_bytes());
        let sec = opt + 240;
        b[sec + 16..sec + 20].copy_from_slice(&0x200u32.to_le_bytes());
        b[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes());
        for (i, x) in b[0x200..0x400].iter_mut().enumerate() {
            *x = i as u8;
        }
        if let Some(t) = table {
            let at = b.len() as u32;
            let mut entry = ((t.len() + 8) as u32).to_le_bytes().to_vec();
            entry.extend(0x0200u16.to_le_bytes());
            entry.extend(0x0002u16.to_le_bytes());
            entry.extend(t);
            while !entry.len().is_multiple_of(8) {
                entry.push(0);
            }
            let dir = opt + 112 + 32;
            b[dir..dir + 4].copy_from_slice(&at.to_le_bytes());
            b[dir + 4..dir + 8].copy_from_slice(&(entry.len() as u32).to_le_bytes());
            b.extend(entry);
        }
        b
    }

    fn read(b: &[u8]) -> PeSignature {
        read_pe(&mut std::io::Cursor::new(b), b.len() as u64).unwrap()
    }

    #[test]
    fn a_program_without_a_table_is_unsigned() {
        assert_eq!(read(&tiny_pe(None)), PeSignature::None);
    }

    #[test]
    fn garbage_in_the_table_is_broken_not_unsigned() {
        assert!(matches!(
            read(&tiny_pe(Some(b"not a signature"))),
            PeSignature::Broken(_)
        ));
    }

    #[test]
    fn layout_matches_the_authenticode_crate() {
        use object::read::pe::PeFile64;
        let b = tiny_pe(Some(&[0u8; 16]));
        let pe = PeFile64::parse(&b[..]).unwrap();
        let theirs = authenticode::PeTrait::offsets(&pe).unwrap();
        let ours = layout(&b, b.len() as u64).unwrap();
        assert_eq!(ours.check_sum, theirs.check_sum as u64);
        assert_eq!(ours.security_dir, theirs.security_data_dir as u64);
        assert_eq!(ours.after_header, theirs.after_header as u64);
        assert_eq!(ours.sections, vec![(0x200, 0x200)]);
        assert_eq!(ours.table, Some((0x400, 24)));
    }

    #[test]
    fn hostile_headers_are_broken() {
        let good = tiny_pe(None);
        let opt = 0x84 + 20;
        let sec = opt + 240;
        type Change = Box<dyn Fn(&mut Vec<u8>)>;
        let cases: Vec<(&str, Change)> = vec![
            ("no PE", Box::new(|b| b[0x80] = b'X')),
            (
                "PE offset past the end",
                Box::new(|b| b[0x3C..0x40].copy_from_slice(&0xFFFF_FF00u32.to_le_bytes())),
            ),
            ("unknown optional header", Box::new(move |b| b[opt] = 0x99)),
            (
                "too many sections",
                Box::new(|b| b[0x86..0x88].copy_from_slice(&97u16.to_le_bytes())),
            ),
            (
                "section past the end",
                Box::new(move |b| b[sec + 20..sec + 24].copy_from_slice(&0x300u32.to_le_bytes())),
            ),
            (
                "section size wraps",
                Box::new(move |b| b[sec + 16..sec + 20].copy_from_slice(&u32::MAX.to_le_bytes())),
            ),
            (
                "header past the end",
                Box::new(move |b| b[opt + 60..opt + 64].copy_from_slice(&0x500u32.to_le_bytes())),
            ),
            (
                "header before the directory",
                Box::new(move |b| b[opt + 60..opt + 64].copy_from_slice(&0x100u32.to_le_bytes())),
            ),
            (
                "section table cut short",
                Box::new(move |b| {
                    b[0x84 + 16..0x84 + 18].copy_from_slice(&0xFFF0u16.to_le_bytes())
                }),
            ),
            (
                "table past the end",
                Box::new(move |b| {
                    let dir = opt + 112 + 32;
                    b[dir..dir + 4].copy_from_slice(&0x3F0u32.to_le_bytes());
                    b[dir + 4..dir + 8].copy_from_slice(&0x100u32.to_le_bytes());
                }),
            ),
        ];
        for (name, f) in cases {
            let mut b = good.clone();
            f(&mut b);
            assert!(
                matches!(read(&b), PeSignature::Broken(_)),
                "{name}: {:?}",
                read(&b)
            );
        }
        // Two sections over the same bytes.
        let mut b = good.clone();
        b[0x86..0x88].copy_from_slice(&2u16.to_le_bytes());
        b[sec + 40 + 16..sec + 40 + 20].copy_from_slice(&0x200u32.to_le_bytes());
        b[sec + 40 + 20..sec + 40 + 24].copy_from_slice(&0x200u32.to_le_bytes());
        assert!(matches!(read(&b), PeSignature::Broken(_)));
        // Exactly at the end is fine.
        assert_eq!(read(&good), PeSignature::None);
    }

    #[test]
    fn a_huge_table_is_not_read() {
        let mut b = tiny_pe(None);
        let dir = 0x84 + 20 + 112 + 32;
        b[dir..dir + 4].copy_from_slice(&0x400u32.to_le_bytes());
        let len = crate::signer::MAX_SIGNATURE as u32 + 8;
        b[dir + 4..dir + 8].copy_from_slice(&len.to_le_bytes());
        b.resize(0x400 + len as usize, 0);
        assert_eq!(
            read(&b),
            PeSignature::Broken("the certificate table is too large".into())
        );
    }

    #[test]
    fn an_entry_that_is_not_a_signature_is_broken() {
        for (at, v) in [(4usize, 0x0100u16), (6, 0x0001)] {
            let mut b = tiny_pe(Some(&[0u8; 16]));
            b[0x400 + at..0x400 + at + 2].copy_from_slice(&v.to_le_bytes());
            assert!(matches!(read(&b), PeSignature::Broken(w) if w.contains("first entry")));
        }
        for len in [7u32, 25] {
            let mut b = tiny_pe(Some(&[0u8; 16]));
            b[0x400..0x404].copy_from_slice(&len.to_le_bytes());
            assert!(matches!(read(&b), PeSignature::Broken(w) if w.contains("first entry")));
        }
    }

    /// The same program as a 32-bit PE: a shorter optional header.
    fn tiny_pe32(table: bool) -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84;
        b[coff..coff + 2].copy_from_slice(&0x014Cu16.to_le_bytes());
        b[coff + 2..coff + 4].copy_from_slice(&1u16.to_le_bytes());
        b[coff + 16..coff + 18].copy_from_slice(&224u16.to_le_bytes());
        let opt = coff + 20;
        b[opt..opt + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        b[opt + 60..opt + 64].copy_from_slice(&0x200u32.to_le_bytes());
        b[opt + 92..opt + 96].copy_from_slice(&16u32.to_le_bytes());
        let sec = opt + 224;
        b[sec + 16..sec + 20].copy_from_slice(&0x200u32.to_le_bytes());
        b[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes());
        if table {
            let dir = opt + 96 + 32;
            b[dir..dir + 4].copy_from_slice(&0x400u32.to_le_bytes());
            b[dir + 4..dir + 8].copy_from_slice(&16u32.to_le_bytes());
            b.extend([0u8; 16]);
        }
        b
    }

    #[test]
    fn layout_of_a_32_bit_program_matches_the_authenticode_crate() {
        use object::read::pe::PeFile32;
        let b = tiny_pe32(true);
        let pe = PeFile32::parse(&b[..]).unwrap();
        let theirs = authenticode::PeTrait::offsets(&pe).unwrap();
        let ours = layout(&b, b.len() as u64).unwrap();
        assert_eq!(ours.check_sum, theirs.check_sum as u64);
        assert_eq!(ours.security_dir, theirs.security_data_dir as u64);
        assert_eq!(ours.after_header, theirs.after_header as u64);
        assert_eq!(ours.table, Some((0x400, 16)));
        assert_eq!(layout(&tiny_pe32(false), 0x400).unwrap().table, None);
        // Four directories in a 32-bit program: no certificate table either.
        let mut four = tiny_pe32(true);
        let opt = 0x84 + 20;
        four[opt + 92..opt + 96].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(layout(&four, four.len() as u64).unwrap().table, None);
    }

    #[test]
    fn a_table_needs_a_fifth_directory_an_address_and_a_size() {
        let opt = 0x84 + 20;
        // Only four directories: there is no certificate table to read.
        let mut b = tiny_pe(Some(&[0u8; 16]));
        b[opt + 108..opt + 112].copy_from_slice(&4u32.to_le_bytes());
        assert_eq!(layout(&b, b.len() as u64).unwrap().table, None);
        b[opt + 108..opt + 112].copy_from_slice(&5u32.to_le_bytes());
        assert!(layout(&b, b.len() as u64).unwrap().table.is_some());
        let dir = opt + 112 + 32;
        let mut no_address = tiny_pe(Some(&[0u8; 16]));
        no_address[dir..dir + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(read(&no_address), PeSignature::None);
        let mut no_size = tiny_pe(Some(&[0u8; 16]));
        no_size[dir + 4..dir + 8].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(read(&no_size), PeSignature::None);
    }

    #[test]
    fn the_largest_number_of_sections_is_allowed() {
        let mut b = tiny_pe(None);
        b.resize(0x1200, 0);
        b[0x86..0x88].copy_from_slice(&96u16.to_le_bytes());
        // Every other section is empty (its table entry lies over the first section's bytes).
        let first_entry_end = 0x84 + 20 + 240 + 40;
        b[first_entry_end..].fill(0);
        assert_eq!(read(&b), PeSignature::None);
    }

    #[test]
    fn header_bounds_are_exact() {
        let opt = 0x84 + 20;
        let security_dir = (opt + 112 + 32) as u32;
        let set_header =
            |b: &mut Vec<u8>, v: u32| b[opt + 60..opt + 64].copy_from_slice(&v.to_le_bytes());
        // The header may end right after the directory, or right at the end of the file.
        let mut b = tiny_pe(None);
        set_header(&mut b, security_dir + 8);
        assert_eq!(
            layout(&b, b.len() as u64).unwrap().after_header,
            u64::from(security_dir + 8)
        );
        let mut b = tiny_pe(None);
        b.truncate(0x200);
        b[0x84 + 2..0x84 + 4].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(read(&b), PeSignature::None);
        // Not inside the directory.
        let mut b = tiny_pe(None);
        set_header(&mut b, security_dir + 4);
        assert!(matches!(read(&b), PeSignature::Broken(_)));
    }

    #[test]
    fn table_and_entry_sizes_are_exact() {
        // A table of exactly the largest size is read (and is not a signature).
        let mut b = tiny_pe(None);
        let dir = 0x84 + 20 + 112 + 32;
        let len = crate::signer::MAX_SIGNATURE as u32;
        b[dir..dir + 4].copy_from_slice(&0x400u32.to_le_bytes());
        b[dir + 4..dir + 8].copy_from_slice(&len.to_le_bytes());
        b.resize(0x400 + len as usize, 0);
        assert!(matches!(read(&b), PeSignature::Broken(w) if w.contains("first entry")));
        // An entry exactly filling the table, or holding nothing at all, is read as a signature.
        let mut b = tiny_pe(Some(&[0u8; 16]));
        b[0x400..0x404].copy_from_slice(&24u32.to_le_bytes());
        assert!(matches!(read(&b), PeSignature::Broken(w) if w.contains("cannot be read")));
        b[0x400..0x404].copy_from_slice(&8u32.to_le_bytes());
        assert!(matches!(read(&b), PeSignature::Broken(w) if w.contains("cannot be read")));
    }
}
