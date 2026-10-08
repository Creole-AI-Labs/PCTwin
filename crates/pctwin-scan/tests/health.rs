//! The old drive's health (Task List 1.4): read from what each system already reports to a normal
//! user, never guessed. "Not supported" or no answer is Unknown, never Good. A weak drive switches
//! the move to careful reading.

use pctwin_scan::{
    Health, ReadPlan, health_from_diskutil, health_from_get_physical_disk, health_from_udisks,
    system_drive_health,
};

#[test]
fn windows_drive_health_is_read_from_get_physical_disk() {
    let one = r#"{"Name":"Samsung SSD 980","Health":"Healthy","Operational":"OK","System":true}"#;
    assert_eq!(health_from_get_physical_disk(one), Health::Good);
    let many = r#"[
        {"Name":"USB Stick","Health":"Healthy","Operational":"OK","System":false},
        {"Name":"WDC HDD","Health":"Warning","Operational":"Predictive Failure","System":true}
    ]"#;
    // The system drive is the one that matters.
    assert!(matches!(
        health_from_get_physical_disk(many),
        Health::Warning(_)
    ));
    let bad = r#"{"Name":"Old HDD","Health":"Unhealthy","Operational":"Lost Communication","System":true}"#;
    assert!(matches!(
        health_from_get_physical_disk(bad),
        Health::Failing(_)
    ));
    let unknown = r#"{"Name":"Virtual Disk","Health":"Unknown","Operational":"OK","System":true}"#;
    assert!(matches!(
        health_from_get_physical_disk(unknown),
        Health::Unknown(_)
    ));
    assert!(matches!(
        health_from_get_physical_disk(""),
        Health::Unknown(_)
    ));
    assert!(matches!(
        health_from_get_physical_disk("not json"),
        Health::Unknown(_)
    ));
    // No disk marked as the system one: no verdict.
    let no_system = r#"{"Name":"X","Health":"Healthy","Operational":"OK","System":false}"#;
    assert!(matches!(
        health_from_get_physical_disk(no_system),
        Health::Unknown(_)
    ));
}

#[test]
fn mac_drive_health_is_read_from_diskutil() {
    let verified =
        "   Device Node:               /dev/disk3s1\n   SMART Status:              Verified\n";
    assert_eq!(health_from_diskutil(verified), Health::Good);
    let failing = "   SMART Status:              Failing\n";
    assert!(matches!(health_from_diskutil(failing), Health::Failing(_)));
    // Apple's own SSDs and most USB drives say this: it is not a clean bill of health.
    let unsupported = "   SMART Status:              Not Supported\n";
    assert!(matches!(
        health_from_diskutil(unsupported),
        Health::Unknown(_)
    ));
    assert!(matches!(
        health_from_diskutil("   Device Node: /dev/disk2\n"),
        Health::Unknown(_)
    ));
}

#[test]
fn linux_drive_health_is_read_from_udisks() {
    let ata_ok = "  org.freedesktop.UDisks2.Drive.Ata:\n    SmartEnabled:               true\n    SmartFailing:               false\n";
    assert_eq!(health_from_udisks(ata_ok), Health::Good);
    let ata_bad = "    SmartFailing:               true\n";
    assert!(matches!(health_from_udisks(ata_bad), Health::Failing(_)));
    let nvme_ok = "  org.freedesktop.UDisks2.NVMe.Controller:\n    SmartCriticalWarning:       \n    SmartUpdated:               1700000000\n";
    assert_eq!(health_from_udisks(nvme_ok), Health::Good);
    let nvme_warn = "    SmartCriticalWarning:       spare\n";
    assert!(matches!(health_from_udisks(nvme_warn), Health::Warning(_)));
    let nvme_ro = "    SmartCriticalWarning:       spare, readonly\n";
    assert!(matches!(health_from_udisks(nvme_ro), Health::Failing(_)));
    assert!(matches!(
        health_from_udisks("  org.freedesktop.UDisks2.Block:\n"),
        Health::Unknown(_)
    ));
}

#[test]
fn a_weak_drive_is_read_carefully() {
    assert_eq!(ReadPlan::for_health(&Health::Good), ReadPlan::Normal);
    // Unknown is not treated as weak; the read-error watch during the move covers it.
    assert_eq!(
        ReadPlan::for_health(&Health::Unknown("x".into())),
        ReadPlan::Normal
    );
    for weak in [Health::Warning("x".into()), Health::Failing("x".into())] {
        assert_eq!(
            ReadPlan::for_health(&weak),
            ReadPlan::Careful {
                most_important_first: true,
                read_each_file_once: true,
                stop_after_read_errors: 5,
            }
        );
    }
}

#[test]
fn this_laptops_drive_health_comes_back_quickly_and_honestly() {
    let started = std::time::Instant::now();
    let health = system_drive_health();
    // Whatever the answer, it never takes long and is never invented.
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    eprintln!("system drive health: {health:?}");
}
