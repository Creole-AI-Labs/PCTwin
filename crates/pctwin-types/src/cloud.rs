//! Whether a file's contents are stored online only: reading such a file makes its cloud app
//! download it, so PCTwin never opens one to look inside (the scan, the app-data classifier, the
//! installer check and the sender all ask here; research notes 2.6, 29 October 2026). One place,
//! so no reader can forget a system.
//!
//! - **Windows** (OneDrive Files On-Demand and every other cloud folder built on the system's
//!   cloud files support): any of the attributes `OFFLINE`, `RECALL_ON_OPEN` or
//!   `RECALL_ON_DATA_ACCESS`.
//! - **macOS 14 and later**: the file is "dataless" (`SF_DATALESS`), its contents only in iCloud.
//!   Before macOS 14 an evicted file was replaced by a hidden `.<name>.icloud` stand-in
//!   ([`is_icloud_stub_name`]).
//! - **Linux**: no system-wide mark; nothing is online only.
//!
//! Ask with the metadata of a file already opened (its handle) where possible: then nothing can
//! make it online only between the question and the read.

/// Whether the file with metadata `meta` is stored online only (reading it would download it).
#[cfg(windows)]
pub fn is_cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    cloud_only_windows(meta.file_attributes())
}

/// Whether the file with metadata `meta` is stored online only (reading it would download it).
#[cfg(target_os = "macos")]
pub fn is_cloud_only(meta: &std::fs::Metadata) -> bool {
    use std::os::macos::fs::MetadataExt;
    cloud_only_mac(meta.st_flags())
}

/// Whether the file with metadata `meta` is stored online only (never, here).
#[cfg(not(any(windows, target_os = "macos")))]
pub fn is_cloud_only(_meta: &std::fs::Metadata) -> bool {
    false
}

/// Windows: the file's contents are not on the laptop, from its attributes.
pub fn cloud_only_windows(attributes: u32) -> bool {
    const OFFLINE: u32 = 0x1000;
    const RECALL_ON_OPEN: u32 = 0x4_0000;
    const RECALL_ON_DATA_ACCESS: u32 = 0x40_0000;
    attributes & (OFFLINE | RECALL_ON_OPEN | RECALL_ON_DATA_ACCESS) != 0
}

/// macOS 14 and later: the file is "dataless", its contents only in iCloud (`SF_DATALESS`).
pub fn cloud_only_mac(flags: u32) -> bool {
    const SF_DATALESS: u32 = 0x4000_0000;
    flags & SF_DATALESS != 0
}

/// Before macOS 14, a file evicted to iCloud was replaced by a hidden stand-in named
/// `.<name>.icloud`.
pub fn is_icloud_stub_name(name: &str) -> bool {
    name.strip_prefix('.')
        .and_then(|n| n.strip_suffix(".icloud"))
        .is_some_and(|inner| !inner.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_windows_mark_means_online_only_and_others_do_not() {
        for a in [0x1000, 0x4_0000, 0x40_0000, 0x40_0000 | 0x20] {
            assert!(cloud_only_windows(a), "{a:#x}");
        }
        // Read-only, hidden, archive, a link: still here.
        for a in [0, 0x1, 0x2, 0x20, 0x400, 0x80] {
            assert!(!cloud_only_windows(a), "{a:#x}");
        }
    }

    #[test]
    fn a_dataless_mac_file_is_online_only() {
        assert!(cloud_only_mac(0x4000_0000));
        assert!(cloud_only_mac(0x4000_0000 | 0x8000));
        assert!(!cloud_only_mac(0x8000));
        assert!(!cloud_only_mac(0));
    }

    #[test]
    fn an_old_icloud_stand_in_is_known_by_its_name() {
        assert!(is_icloud_stub_name(".report.pdf.icloud"));
        assert!(!is_icloud_stub_name("report.pdf.icloud"));
        assert!(!is_icloud_stub_name("..icloud"));
        assert!(!is_icloud_stub_name(".report.pdf"));
    }
}
