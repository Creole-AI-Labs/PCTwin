//! The laptop's own chip, read even when PCTwin runs under emulation (an x64 copy of PCTwin on an
//! Arm laptop, through Windows' Prism or Apple's Rosetta, is told it runs on x64): the app
//! builds chosen for the new laptop follow the chip it really has.
//!
//! - Windows: `IsWow64Process2` gives the machine's native architecture (the one fenced call).
//! - macOS: `hw.optional.arm64` is 1 on Apple silicon, under Rosetta too (read with the system's
//!   own `sysctl`; an Intel Mac has no such entry).
//! - Linux: `uname`'s machine field, through rustix (Linux runs no other chip's programs unless
//!   asked to, and then says so here).

/// A chip family, as app builds are made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chip {
    X86_64,
    Arm64,
    X86,
    /// Anything else, by the name the system gave.
    Other,
}

/// The chip this copy of PCTwin was built for.
pub const fn this_build() -> Chip {
    if cfg!(target_arch = "x86_64") {
        Chip::X86_64
    } else if cfg!(target_arch = "aarch64") {
        Chip::Arm64
    } else if cfg!(target_arch = "x86") {
        Chip::X86
    } else {
        Chip::Other
    }
}

/// The laptop's own chip, and whether this copy of PCTwin runs under emulation on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChipReading {
    pub native: Chip,
    pub emulated: bool,
}

/// Reads the laptop's own chip (`None`: the system would not say).
pub fn native_chip() -> Option<ChipReading> {
    let native = read_native()?;
    Some(ChipReading {
        native,
        emulated: native != this_build(),
    })
}

/// The chip a Windows machine number names (`IMAGE_FILE_MACHINE_*`).
pub fn chip_of_windows_machine(machine: u16) -> Chip {
    match machine {
        0x8664 => Chip::X86_64,
        0xAA64 => Chip::Arm64,
        0x014C => Chip::X86,
        _ => Chip::Other,
    }
}

/// The chip `uname -m` names.
pub fn chip_of_uname(machine: &str) -> Chip {
    match machine {
        "x86_64" | "amd64" => Chip::X86_64,
        "aarch64" | "arm64" => Chip::Arm64,
        "i386" | "i486" | "i586" | "i686" | "x86" => Chip::X86,
        _ => Chip::Other,
    }
}

#[cfg(windows)]
fn read_native() -> Option<Chip> {
    crate::win::native_machine()
        .ok()
        .map(chip_of_windows_machine)
}

#[cfg(target_os = "macos")]
fn read_native() -> Option<Chip> {
    let out = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "hw.optional.arm64"])
        .output()
        .ok()?;
    // Apple silicon says 1, Rosetta or not; an Intel Mac has no such entry and fails.
    Some(if out.status.success() && out.stdout.trim_ascii() == b"1" {
        Chip::Arm64
    } else {
        Chip::X86_64
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn read_native() -> Option<Chip> {
    let name = rustix::system::uname();
    Some(chip_of_uname(&name.machine().to_string_lossy()))
}

#[cfg(not(any(windows, unix)))]
fn read_native() -> Option<Chip> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_names_are_read_as_chips() {
        assert_eq!(chip_of_windows_machine(0x8664), Chip::X86_64);
        assert_eq!(chip_of_windows_machine(0xAA64), Chip::Arm64);
        assert_eq!(chip_of_windows_machine(0x014C), Chip::X86);
        assert_eq!(chip_of_windows_machine(0x01C4), Chip::Other);
        assert_eq!(chip_of_uname("x86_64"), Chip::X86_64);
        assert_eq!(chip_of_uname("aarch64"), Chip::Arm64);
        assert_eq!(chip_of_uname("i686"), Chip::X86);
        assert_eq!(chip_of_uname("riscv64"), Chip::Other);
    }

    /// On a test machine with no emulation (every runner PCTwin tests on), the chip read is
    /// the one this test was built for.
    #[test]
    fn the_chip_read_here_is_known() {
        let r = native_chip().expect("the system says which chip it has");
        assert_ne!(r.native, Chip::Other);
        if std::env::var_os("PCTWIN_EXPECT_EMULATED").is_none() {
            assert_eq!(r.native, this_build());
            assert!(!r.emulated);
        }
    }
}
