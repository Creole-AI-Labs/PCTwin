//! A Windows Installer package (MSI): whether it is one, what it is built for, and who signed it.
//!
//! An MSI is a compound (OLE) file. Its signature is a PKCS #7 SignedData in the stream named
//! `\u{5}DigitalSignature` (with an optional `\u{5}MsiDigitalSignatureEx` beside it). The digest
//! of an MSI covers every stream in a set order; PCTwin does not work it out itself (there is no
//! maintained Rust crate for it, and it is not for us to write), so whether the bytes are the bytes
//! signed is Windows' own check alone for an MSI. This reader gives the signer, for the maker
//! comparison and for cross-checking Windows' answer.

use std::io::{Read, Seek};

use crate::facts::{Arch, WindowsSigner};
use crate::signer::{MAX_SIGNATURE, read_signature};
use crate::sniff::msi_template_arch;

/// The class ID a Windows Installer package carries on its root.
const MSI_CLSID: [u8; 16] = [
    0x84, 0x10, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];
/// The most summary bytes read.
const MAX_SUMMARY: u64 = 64 * 1024;

/// What the package says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsiRead {
    /// False for a compound file that is not an installer package (an old Word file, a patch).
    pub is_package: bool,
    pub arch: Option<Arch>,
    pub signature: MsiSignature,
}

#[expect(
    clippy::large_enum_variant,
    reason = "made once per check; boxing buys nothing"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MsiSignature {
    None,
    Broken(String),
    Signed(Result<WindowsSigner, String>),
}

fn read_stream<F: Read + Seek>(
    file: &mut cfb::CompoundFile<F>,
    name: &str,
    max: u64,
) -> Result<Option<Vec<u8>>, String> {
    if !file.is_stream(name) {
        return Ok(None);
    }
    let s = file.open_stream(name).map_err(|e| e.to_string())?;
    if s.len() > max {
        return Err(format!("the stream {name:?} is too large"));
    }
    let mut out = Vec::new();
    s.take(max)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    Ok(Some(out))
}

/// Reads a compound file. Err when it cannot be read as one at all.
pub fn read_msi<F: Read + Seek>(f: F) -> Result<MsiRead, String> {
    let mut file =
        cfb::CompoundFile::open(f).map_err(|e| format!("the package cannot be read ({e})"))?;
    let clsid = *file.root_entry().clsid().as_bytes();
    // A uuid's bytes are in big-endian field order; the class ID above is as stored (mixed-endian).
    let stored = uuid::Uuid::from_bytes_le(MSI_CLSID);
    if clsid != *stored.as_bytes() {
        return Ok(MsiRead {
            is_package: false,
            arch: None,
            signature: MsiSignature::None,
        });
    }
    let arch = match read_stream(&mut file, "/\u{5}SummaryInformation", MAX_SUMMARY) {
        Ok(Some(b)) => template(&b).and_then(|t| msi_template_arch(&t)),
        _ => None,
    };
    let signature = match read_stream(&mut file, "/\u{5}DigitalSignature", MAX_SIGNATURE as u64) {
        Ok(None) => MsiSignature::None,
        Ok(Some(b)) => match read_signature(&b) {
            Ok(s) => MsiSignature::Signed(s.signer),
            Err(why) => MsiSignature::Broken(why),
        },
        Err(why) => MsiSignature::Broken(why),
    };
    Ok(MsiRead {
        is_package: true,
        arch,
        signature,
    })
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// The Template property (number 7, a short text) of a summary information property set.
pub fn template(b: &[u8]) -> Option<String> {
    if b.get(0..2)? != [0xFE, 0xFF] {
        return None;
    }
    let section = usize::try_from(u32_at(b, 44)?).ok()?;
    let count = usize::try_from(u32_at(b, section.checked_add(4)?)?).ok()?;
    for i in 0..count.min(256) {
        let entry = section.checked_add(8)?.checked_add(i.checked_mul(8)?)?;
        if u32_at(b, entry)? != 7 {
            continue;
        }
        let at = section.checked_add(usize::try_from(u32_at(b, entry + 4)?).ok()?)?;
        // VT_LPSTR: a length (with the closing zero), then the bytes.
        if u32_at(b, at)? != 0x1E {
            return None;
        }
        let len = usize::try_from(u32_at(b, at + 4)?).ok()?;
        let text = b.get(at.checked_add(8)?..at.checked_add(8)?.checked_add(len)?)?;
        let text = text.split(|c| *c == 0).next().unwrap_or(&[]);
        return Some(String::from_utf8_lossy(text).into_owned());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A summary property set holding one Template property.
    pub(crate) fn summary(template_text: &str) -> Vec<u8> {
        let mut b = vec![0u8; 48];
        b[0] = 0xFE;
        b[1] = 0xFF;
        b[24..28].copy_from_slice(&1u32.to_le_bytes());
        b[44..48].copy_from_slice(&48u32.to_le_bytes());
        let mut text = template_text.as_bytes().to_vec();
        text.push(0);
        let mut section = Vec::new();
        section.extend(0u32.to_le_bytes());
        section.extend(2u32.to_le_bytes());
        // A codepage property first, so the reader must look past it.
        section.extend(1u32.to_le_bytes());
        section.extend(24u32.to_le_bytes());
        section.extend(7u32.to_le_bytes());
        section.extend(32u32.to_le_bytes());
        section.extend(2u32.to_le_bytes());
        section.extend(1252u32.to_le_bytes());
        section.extend(0x1Eu32.to_le_bytes());
        section.extend((text.len() as u32).to_le_bytes());
        section.extend(&text);
        b.extend(section);
        b
    }

    #[test]
    fn template_is_found_past_other_properties() {
        assert_eq!(template(&summary("x64;1033")).as_deref(), Some("x64;1033"));
        assert_eq!(template(&summary("Arm64;0")).as_deref(), Some("Arm64;0"));
    }

    #[test]
    fn broken_property_sets_say_nothing() {
        assert_eq!(template(b""), None);
        let mut wrong_order = summary("x64;1033");
        wrong_order[0] = 0xFF;
        assert_eq!(template(&wrong_order), None);
        let mut not_text = summary("x64;1033");
        let at = 48 + 32;
        not_text[at..at + 4].copy_from_slice(&0x03u32.to_le_bytes());
        assert_eq!(template(&not_text), None);
        let mut too_long = summary("x64;1033");
        too_long[at + 4..at + 8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(template(&too_long), None);
        let mut no_template = summary("x64;1033");
        no_template[48 + 16..48 + 20].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(template(&no_template), None);
    }

    fn compound(clsid_le: [u8; 16], streams: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut c = cfb::CompoundFile::create(std::io::Cursor::new(Vec::new())).unwrap();
        c.set_storage_clsid("/", uuid::Uuid::from_bytes_le(clsid_le))
            .unwrap();
        for (name, data) in streams {
            let mut s = c.create_stream(name).unwrap();
            std::io::Write::write_all(&mut s, data).unwrap();
        }
        c.flush().unwrap();
        c.into_inner().into_inner()
    }

    #[test]
    fn an_old_office_file_is_not_a_package() {
        let mut word = MSI_CLSID;
        word[0] = 0x06;
        let b = compound(word, &[]);
        let r = read_msi(std::io::Cursor::new(b)).unwrap();
        assert!(!r.is_package);
    }

    #[test]
    fn a_package_with_its_summary_and_no_signature() {
        let b = compound(
            MSI_CLSID,
            &[("/\u{5}SummaryInformation", summary("Arm64;1033"))],
        );
        let r = read_msi(std::io::Cursor::new(b)).unwrap();
        assert!(r.is_package);
        assert_eq!(r.arch, Some(Arch::Arm64));
        assert_eq!(r.signature, MsiSignature::None);
    }

    #[test]
    fn a_package_with_a_garbage_signature_is_broken() {
        let b = compound(
            MSI_CLSID,
            &[("/\u{5}DigitalSignature", b"not a signature".to_vec())],
        );
        let r = read_msi(std::io::Cursor::new(b)).unwrap();
        assert!(matches!(r.signature, MsiSignature::Broken(_)));
        assert_eq!(r.arch, None);
    }

    #[test]
    fn a_huge_signature_stream_is_not_read() {
        let b = compound(
            MSI_CLSID,
            &[("/\u{5}DigitalSignature", vec![0u8; MAX_SIGNATURE + 1])],
        );
        let r = read_msi(std::io::Cursor::new(b)).unwrap();
        assert!(matches!(r.signature, MsiSignature::Broken(w) if w.contains("too large")));
    }

    #[test]
    fn not_a_compound_file_is_an_error() {
        assert!(read_msi(std::io::Cursor::new(b"hello".to_vec())).is_err());
    }

    #[test]
    fn a_signature_stream_of_exactly_the_limit_is_read() {
        let b = compound(
            MSI_CLSID,
            &[("/\u{5}DigitalSignature", vec![0u8; MAX_SIGNATURE])],
        );
        let r = read_msi(std::io::Cursor::new(b)).unwrap();
        assert!(matches!(r.signature, MsiSignature::Broken(w) if w.contains("cannot be read")));
    }

    #[test]
    fn only_the_first_256_properties_are_looked_at() {
        let mut b = vec![0u8; 48];
        b[0] = 0xFE;
        b[1] = 0xFF;
        b[44..48].copy_from_slice(&48u32.to_le_bytes());
        let n = 300u32;
        let mut section = Vec::new();
        section.extend(0u32.to_le_bytes());
        section.extend(n.to_le_bytes());
        let text_at = 8 + 8 * n;
        for i in 0..n {
            section.extend((if i == 290 { 7u32 } else { 99 }).to_le_bytes());
            section.extend(text_at.to_le_bytes());
        }
        section.extend(0x1Eu32.to_le_bytes());
        section.extend(4u32.to_le_bytes());
        section.extend(b"x64\0");
        b.extend(section);
        assert_eq!(template(&b), None);
    }
}
