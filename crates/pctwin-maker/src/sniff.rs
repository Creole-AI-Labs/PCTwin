//! What kind of file it is, and what it is built for, from its first and last bytes only. A name
//! or a server's word for the type is never used (2.1 plan, section 9, point 6).

use crate::facts::{Arch, ArchSet, Format};

/// How many leading bytes the sniffer looks at.
pub const HEAD: usize = 64 * 1024;
/// How many trailing bytes it looks at (a Mac disk image ends in a 512-byte trailer).
pub const TAIL: usize = 512;

/// What the first and last bytes say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sniffed {
    pub format: Format,
    /// None when the bytes do not say (an RPM, a disk image), or for a compound file until the
    /// installer reader has looked inside.
    pub arch: Option<Arch>,
    /// A compound (OLE) file: an MSI is one, but so is an old Word file. The MSI reader decides.
    pub compound: bool,
}

const CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// Reads the kind of file from `head` (up to [`HEAD`] bytes from the start) and `tail` (up to
/// [`TAIL`] bytes from the end; for a file shorter than that, the whole file).
pub fn sniff(head: &[u8], tail: &[u8]) -> Sniffed {
    let plain = |format, arch| Sniffed {
        format,
        arch,
        compound: false,
    };
    if head.starts_with(b"MZ") {
        return match pe_machine(head) {
            Some(arch) => plain(Format::WindowsProgram, arch),
            None => plain(Format::Unknown, None),
        };
    }
    if head.starts_with(&CFB_MAGIC) {
        return Sniffed {
            format: Format::Unknown,
            arch: None,
            compound: true,
        };
    }
    if let Some(arch) = mach_o(head) {
        return plain(Format::MacProgram, arch);
    }
    if head.starts_with(b"xar!") {
        return plain(Format::MacInstaller, None);
    }
    if head.starts_with(b"\x7fELF") {
        return plain(Format::LinuxProgram, elf_machine(head));
    }
    if head.starts_with(b"!<arch>\ndebian-binary") {
        return plain(Format::DebPackage, None);
    }
    if head.starts_with(&[0xED, 0xAB, 0xEE, 0xDB]) {
        return plain(Format::RpmPackage, None);
    }
    if head.starts_with(b"[Flatpak Ref]") {
        // Flatpak picks the build for the laptop itself.
        let all = ArchSet {
            x86: true,
            x64: true,
            arm64: true,
        };
        return plain(Format::FlatpakRef, Some(Arch::Several(all)));
    }
    if tail.len() == TAIL && tail.starts_with(b"koly") {
        return plain(Format::MacDiskImage, None);
    }
    if head.starts_with(b"PK\x03\x04") {
        return plain(Format::Zip, None);
    }
    if looks_like_web_page(head) {
        return plain(Format::WebPage, None);
    }
    plain(Format::Unknown, None)
}

fn u16_le(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u32_be(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// A Windows program's machine, or None if the bytes are not a whole PE header. Some(None) for a
/// PE built for a machine PCTwin does not install for.
fn pe_machine(head: &[u8]) -> Option<Option<Arch>> {
    let pe = usize::try_from(u32_le(head, 0x3C)?).ok()?;
    if head.get(pe..pe.checked_add(4)?)? != b"PE\0\0" {
        return None;
    }
    let machine = u16_le(head, pe.checked_add(4)?)?;
    Some(match machine {
        0x014C => Some(Arch::X86),
        0x8664 => Some(Arch::X64),
        0xAA64 => Some(Arch::Arm64),
        _ => None,
    })
}

const CPU_X86: u32 = 7;
const CPU_X64: u32 = 0x0100_0007;
const CPU_ARM64: u32 = 0x0100_000C;

fn cpu(t: u32) -> Option<Arch> {
    match t {
        CPU_X86 => Some(Arch::X86),
        CPU_X64 => Some(Arch::X64),
        CPU_ARM64 => Some(Arch::Arm64),
        _ => None,
    }
}

/// A Mac program's architectures, or None if the bytes are not one.
fn mach_o(head: &[u8]) -> Option<Option<Arch>> {
    let magic = u32_be(head, 0)?;
    match magic {
        // Thin, little-endian (every Mac this century): cputype follows, little-endian.
        0xCEFA_EDFE | 0xCFFA_EDFE => Some(cpu(u32_le(head, 4)?)),
        // Thin, big-endian.
        0xFEED_FACE | 0xFEED_FACF => Some(cpu(u32_be(head, 4)?)),
        // Universal: a count, then 20-byte (or 32-byte for the 64-bit form) entries. Java class
        // files share CAFEBABE; their next word is a version of 45 or more, so a universal file
        // is only taken with fewer than 20 entries, as file(1) does.
        0xCAFE_BABE | 0xCAFE_BABF => {
            let n = u32_be(head, 4)?;
            if n == 0 || n >= 20 {
                return None;
            }
            let step = if magic == 0xCAFE_BABE { 20 } else { 32 };
            let mut set = ArchSet::default();
            for i in 0..usize::try_from(n).ok()? {
                let t = u32_be(head, 8 + i * step)?;
                match cpu(t) {
                    Some(Arch::X86) => set.x86 = true,
                    Some(Arch::X64) => set.x64 = true,
                    Some(Arch::Arm64) => set.arm64 = true,
                    _ => {}
                }
            }
            Some(Some(Arch::Several(set)))
        }
        _ => None,
    }
}

fn elf_machine(head: &[u8]) -> Option<Arch> {
    let m = match head.get(5)? {
        1 => u16_le(head, 18)?,
        2 => u16::from_be_bytes(head.get(18..20)?.try_into().ok()?),
        _ => return None,
    };
    match m {
        3 => Some(Arch::X86),
        62 => Some(Arch::X64),
        183 => Some(Arch::Arm64),
        _ => None,
    }
}

/// A web page saved under an installer's name: after an optional byte-order mark and white space,
/// it opens with a tag (`<!doctype html>`, `<html`, `<head`, `<!--`, `<?xml`...).
fn looks_like_web_page(head: &[u8]) -> bool {
    let b = head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head);
    let start = b.iter().position(|c| !c.is_ascii_whitespace());
    let Some(start) = start else { return false };
    let rest = &b[start..];
    rest.first() == Some(&b'<')
        && rest
            .get(1)
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'!' || *c == b'?')
}

/// An MSI's architecture from its Template summary property ("x64;1033", "Intel;1033",
/// "Arm64;1033", or ";1033", which means 32-bit Intel). None for a platform PCTwin does not
/// install for, or text that is not a template.
pub fn msi_template_arch(template: &str) -> Option<Arch> {
    let platforms = template.split(';').next()?;
    let mut set = ArchSet::default();
    for p in platforms.split(',') {
        match p.trim() {
            "" | "Intel" => set.x86 = true,
            "x64" | "AMD64" => set.x64 = true,
            "Arm64" => set.arm64 = true,
            _ => return None,
        }
    }
    match (set.x86, set.x64, set.arm64) {
        (true, false, false) => Some(Arch::X86),
        (false, true, false) => Some(Arch::X64),
        (false, false, true) => Some(Arch::Arm64),
        _ => Some(Arch::Several(set)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pe(machine: u16) -> Vec<u8> {
        let mut b = vec![0u8; 256];
        b[0] = b'M';
        b[1] = b'Z';
        b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        b[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
        b
    }

    #[test]
    fn windows_programs_by_machine() {
        assert_eq!(sniff(&pe(0x8664), &[]).arch, Some(Arch::X64));
        assert_eq!(sniff(&pe(0x014C), &[]).arch, Some(Arch::X86));
        assert_eq!(sniff(&pe(0xAA64), &[]).arch, Some(Arch::Arm64));
        assert_eq!(sniff(&pe(0x8664), &[]).format, Format::WindowsProgram);
        // An Itanium build is a program PCTwin does not install for.
        let s = sniff(&pe(0x0200), &[]);
        assert_eq!((s.format, s.arch), (Format::WindowsProgram, None));
    }

    #[test]
    fn mz_without_a_pe_header_is_not_a_program() {
        let mut b = pe(0x8664);
        b[0x80] = b'X';
        assert_eq!(sniff(&b, &[]).format, Format::Unknown);
        // The PE offset points past the bytes.
        let mut b = pe(0x8664);
        b[0x3C..0x40].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
        assert_eq!(sniff(&b, &[]).format, Format::Unknown);
        assert_eq!(sniff(b"MZ", &[]).format, Format::Unknown);
    }

    #[test]
    fn an_html_sign_in_page_named_setup_exe_is_a_web_page() {
        // T21-023: a 200 response that is a sign-in page.
        for page in [
            &b"<!DOCTYPE html><html><body>Sign in</body></html>"[..],
            b"\xEF\xBB\xBF\r\n  <html lang=en>",
            b"<?xml version=\"1.0\"?><html/>",
            b"<!-- error -->",
            b"\n<head><title>Login</title>",
        ] {
            assert_eq!(sniff(page, &[]).format, Format::WebPage, "{page:?}");
        }
        for not_page in [&b"< not a tag"[..], b"", b"   ", b"<", b"{\"error\":1}"] {
            assert_eq!(sniff(not_page, &[]).format, Format::Unknown, "{not_page:?}");
        }
    }

    #[test]
    fn compound_files_are_left_to_the_msi_reader() {
        let mut b = CFB_MAGIC.to_vec();
        b.extend([0u8; 100]);
        let s = sniff(&b, &[]);
        assert!(s.compound);
        assert_eq!(s.format, Format::Unknown);
        assert!(!sniff(&pe(0x8664), &[]).compound);
    }

    fn fat(entries: &[u32], magic: u32) -> Vec<u8> {
        let step = if magic == 0xCAFE_BABE { 20 } else { 32 };
        let mut b = magic.to_be_bytes().to_vec();
        b.extend((entries.len() as u32).to_be_bytes());
        for e in entries {
            let mut entry = vec![0u8; step];
            entry[..4].copy_from_slice(&e.to_be_bytes());
            b.extend(entry);
        }
        b
    }

    #[test]
    fn mac_programs_thin_and_universal() {
        let mut thin = 0xCFFA_EDFEu32.to_be_bytes().to_vec();
        thin.extend(CPU_ARM64.to_le_bytes());
        let s = sniff(&thin, &[]);
        assert_eq!((s.format, s.arch), (Format::MacProgram, Some(Arch::Arm64)));
        let mut thin32 = 0xCEFA_EDFEu32.to_be_bytes().to_vec();
        thin32.extend(CPU_X86.to_le_bytes());
        assert_eq!(sniff(&thin32, &[]).arch, Some(Arch::X86));
        let mut be = 0xFEED_FACFu32.to_be_bytes().to_vec();
        be.extend(CPU_X64.to_be_bytes());
        assert_eq!(sniff(&be, &[]).arch, Some(Arch::X64));
        let mut be32 = 0xFEED_FACEu32.to_be_bytes().to_vec();
        be32.extend(CPU_X86.to_be_bytes());
        assert_eq!(sniff(&be32, &[]).arch, Some(Arch::X86));

        let uni = sniff(&fat(&[CPU_X64, CPU_ARM64], 0xCAFE_BABE), &[]);
        let both = ArchSet {
            x86: false,
            x64: true,
            arm64: true,
        };
        assert_eq!(uni.arch, Some(Arch::Several(both)));
        let uni64 = sniff(&fat(&[CPU_X86, CPU_X64, CPU_ARM64], 0xCAFE_BABF), &[]);
        let all = ArchSet {
            x86: true,
            x64: true,
            arm64: true,
        };
        assert_eq!(uni64.arch, Some(Arch::Several(all)));
        // A Java class file (version 52) is not a Mac program; nor is an empty universal header.
        let mut java = 0xCAFE_BABEu32.to_be_bytes().to_vec();
        java.extend(52u32.to_be_bytes());
        assert_ne!(sniff(&java, &[]).format, Format::MacProgram);
        assert_ne!(
            sniff(&fat(&[], 0xCAFE_BABE), &[]).format,
            Format::MacProgram
        );
        let nineteen = fat(&[CPU_X64; 19], 0xCAFE_BABE);
        assert_eq!(sniff(&nineteen, &[]).format, Format::MacProgram);
        let twenty = fat(&[CPU_X64; 20], 0xCAFE_BABE);
        assert_ne!(sniff(&twenty, &[]).format, Format::MacProgram);
    }

    #[test]
    fn linux_and_other_kinds() {
        let mut elf = b"\x7fELF\x02\x01".to_vec();
        elf.resize(20, 0);
        elf[18..20].copy_from_slice(&62u16.to_le_bytes());
        let s = sniff(&elf, &[]);
        assert_eq!((s.format, s.arch), (Format::LinuxProgram, Some(Arch::X64)));
        elf[18..20].copy_from_slice(&183u16.to_le_bytes());
        assert_eq!(sniff(&elf, &[]).arch, Some(Arch::Arm64));
        elf[18..20].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(sniff(&elf, &[]).arch, Some(Arch::X86));
        elf[5] = 2;
        elf[18..20].copy_from_slice(&62u16.to_be_bytes());
        assert_eq!(sniff(&elf, &[]).arch, Some(Arch::X64));
        elf[5] = 9;
        assert_eq!(sniff(&elf, &[]).arch, None);

        assert_eq!(
            sniff(b"!<arch>\ndebian-binary   ", &[]).format,
            Format::DebPackage
        );
        assert_eq!(
            sniff(&[0xED, 0xAB, 0xEE, 0xDB, 3, 0], &[]).format,
            Format::RpmPackage
        );
        assert_eq!(sniff(b"xar!\0\x1c", &[]).format, Format::MacInstaller);
        assert_eq!(sniff(b"PK\x03\x04", &[]).format, Format::Zip);
        let fp = sniff(b"[Flatpak Ref]\nName=org.example.App\n", &[]);
        assert_eq!(fp.format, Format::FlatpakRef);
        assert!(fp.arch.is_some_and(|a| a.runs_on(Arch::Arm64)));

        let mut koly = b"koly".to_vec();
        koly.resize(TAIL, 0);
        assert_eq!(sniff(b"\0\0\0\0", &koly).format, Format::MacDiskImage);
        assert_eq!(sniff(b"\0\0\0\0", b"koly").format, Format::Unknown);
    }

    #[test]
    fn msi_templates() {
        assert_eq!(msi_template_arch("x64;1033"), Some(Arch::X64));
        assert_eq!(msi_template_arch("AMD64;1033"), Some(Arch::X64));
        assert_eq!(msi_template_arch("Intel;1033"), Some(Arch::X86));
        assert_eq!(msi_template_arch(";1033"), Some(Arch::X86));
        assert_eq!(msi_template_arch("Arm64;0"), Some(Arch::Arm64));
        assert_eq!(msi_template_arch("Intel64;1033"), None);
        let both = ArchSet {
            x86: true,
            x64: true,
            arm64: false,
        };
        assert_eq!(
            msi_template_arch("Intel,x64;1033"),
            Some(Arch::Several(both))
        );
    }

    #[test]
    fn runs_on_is_exact() {
        assert!(Arch::X64.runs_on(Arch::X64));
        assert!(!Arch::X86.runs_on(Arch::X64));
        assert!(!Arch::X64.runs_on(Arch::Arm64));
        let uni = Arch::Several(ArchSet {
            x86: false,
            x64: true,
            arm64: true,
        });
        assert!(uni.runs_on(Arch::Arm64));
        assert!(uni.runs_on(Arch::X64));
        assert!(!uni.runs_on(Arch::X86));
        assert!(!uni.runs_on(uni));
    }
}
