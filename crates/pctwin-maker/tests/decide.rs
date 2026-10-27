//! The decision: install only when every required fact holds; anything else ends with plain
//! reasons, most serious first. And the maker comparison each system uses.

use pctwin_maker::facts::*;
use pctwin_types::{ItemStatus, Reason};

fn subject(cn: &str, o: &str, l: &str, st: &str, c: &str) -> Subject {
    Subject {
        common_name: Some(cn.into()),
        organization: Some(o.into()),
        locality: Some(l.into()),
        state: Some(st.into()),
        country: Some(c.into()),
        ..Subject::default()
    }
}

fn signer(s: Subject, cert: u8) -> WindowsSigner {
    WindowsSigner {
        subject: s,
        certificate: Sha256([cert; 32]),
        issuer: "CN=Some CA".into(),
    }
}

fn example() -> Subject {
    subject("Example Ltd", "Example Ltd", "Lagos", "Lagos", "NG")
}

fn bytes() -> Bytes {
    Bytes {
        size: 10,
        sha256: Sha256([7; 32]),
    }
}

fn good(system: System) -> Verdict {
    Verdict {
        system,
        bytes: bytes(),
        steady: Steady::Yes,
        expected_bytes: None,
        format: FormatFact::Matches {
            found: Format::WindowsProgram,
        },
        arch: ArchFact::Matches,
        signature: Signature::Trusted,
        same_maker: SameMaker::Yes {
            by: MatchedBy::SameCertificate,
        },
        malware: Malware::NoDetection {
            coverage: Coverage {
                scanner: "Microsoft Defender".into(),
                later_downloads_not_checked: true,
            },
        },
        found_maker: None,
    }
}

fn why(v: &Verdict) -> Vec<Why> {
    match v.decide() {
        Decision::NeedsAction { why } => why,
        Decision::Install { .. } => vec![],
    }
}

#[test]
fn every_fact_holding_installs_bound_to_the_bytes() {
    let d = good(System::Windows).decide();
    assert_eq!(d, Decision::Install { bytes: bytes() });
    assert_eq!(d.item_result(), None);
    assert!(approved_bytes_match(&d, &bytes()));
    // T21-035: other bytes at the installer step are refused.
    let swapped = Bytes {
        size: 10,
        sha256: Sha256([8; 32]),
    };
    assert!(!approved_bytes_match(&d, &swapped));
    assert!(!approved_bytes_match(
        &d,
        &Bytes {
            size: 11,
            ..bytes()
        }
    ));
}

#[test]
fn each_missing_fact_has_its_own_reason() {
    type Change = Box<dyn Fn(&mut Verdict)>;
    let cases: Vec<(Change, Why)> = vec![
        (
            Box::new(|v| v.steady = Steady::Changed),
            Why::ChangedDuringCheck,
        ),
        (
            Box::new(|v| {
                v.malware = Malware::Detected {
                    name: "EICAR".into(),
                }
            }),
            Why::MalwareFound,
        ),
        (
            Box::new(|v| v.expected_bytes = Some(Sha256([9; 32]))),
            Why::NotExpectedBytes,
        ),
        (
            Box::new(|v| {
                v.format = FormatFact::Wrong {
                    found: Format::WebPage,
                }
            }),
            Why::WrongKindOfFile,
        ),
        (
            Box::new(|v| v.signature = Signature::Broken { why: "x".into() }),
            Why::SignatureBroken,
        ),
        (
            Box::new(|v| {
                v.same_maker = SameMaker::No {
                    found: "Other".into(),
                }
            }),
            Why::OtherMaker,
        ),
        (
            Box::new(|v| v.arch = ArchFact::Wrong { found: Arch::X64 }),
            Why::WrongArchitecture,
        ),
        (
            Box::new(|v| v.signature = Signature::Missing),
            Why::Unsigned,
        ),
        (
            Box::new(|v| {
                v.signature = Signature::NotTrusted {
                    why: "expired".into(),
                }
            }),
            Why::NotTrusted,
        ),
        (
            Box::new(|v| v.same_maker = SameMaker::Unknown { why: "x".into() }),
            Why::MakerUnknown,
        ),
        (
            Box::new(|v| v.arch = ArchFact::Unknown),
            Why::ArchitectureUnknown,
        ),
        (
            Box::new(|v| v.signature = Signature::NotChecked { why: "x".into() }),
            Why::NotFullyChecked,
        ),
        (
            Box::new(|v| {
                v.malware = Malware::NotFullyChecked {
                    why: "timeout".into(),
                }
            }),
            Why::NotFullyChecked,
        ),
        (
            Box::new(|v| v.malware = Malware::Unavailable { why: "off".into() }),
            Why::NotFullyChecked,
        ),
    ];
    for (change, want) in cases {
        let mut v = good(System::Windows);
        change(&mut v);
        let d = v.decide();
        assert_eq!(d, Decision::NeedsAction { why: vec![want] }, "{want:?}");
        assert_eq!(
            d.item_result().unwrap().status(),
            ItemStatus::NeedsAction,
            "{want:?}"
        );
        assert_eq!(
            d.item_result().unwrap().reason(),
            Some(Reason::ManualInstall)
        );
    }
    // Expected bytes that match are fine.
    let mut v = good(System::Windows);
    v.expected_bytes = Some(bytes().sha256);
    assert!(matches!(v.decide(), Decision::Install { .. }));
}

#[test]
fn an_unsigned_installer_is_never_installed_by_this_route() {
    // T21-034: everything else perfect.
    for system in [System::Windows, System::Mac, System::Linux] {
        let mut v = good(system);
        v.signature = Signature::Missing;
        assert_eq!(why(&v), vec![Why::Unsigned]);
    }
}

#[test]
fn a_scan_that_claims_more_than_it_saw_is_not_clean() {
    // T21-038: a scan covers the file's own bytes only.
    let mut v = good(System::Windows);
    v.malware = Malware::NoDetection {
        coverage: Coverage {
            scanner: "x".into(),
            later_downloads_not_checked: false,
        },
    };
    assert_eq!(why(&v), vec![Why::NotFullyChecked]);
}

#[test]
fn system_trust_stands_in_for_a_scan_only_where_no_scan_exists() {
    let covered = Malware::CoveredBySystemTrust {
        how: "notarised".into(),
    };
    for system in [System::Mac, System::Linux] {
        let mut v = good(system);
        v.malware = covered.clone();
        assert!(matches!(v.decide(), Decision::Install { .. }), "{system:?}");
    }
    let mut v = good(System::Windows);
    v.malware = covered;
    assert_eq!(why(&v), vec![Why::NotFullyChecked]);
}

#[test]
fn reasons_come_most_serious_first_without_repeats() {
    let mut v = good(System::Windows);
    v.arch = ArchFact::Unknown;
    v.signature = Signature::NotChecked { why: "x".into() };
    v.malware = Malware::Detected { name: "x".into() };
    v.steady = Steady::Changed;
    v.same_maker = SameMaker::No { found: "x".into() };
    v.malware = Malware::NotFullyChecked { why: "x".into() };
    assert_eq!(
        why(&v),
        vec![
            Why::ChangedDuringCheck,
            Why::OtherMaker,
            Why::ArchitectureUnknown,
            Why::NotFullyChecked
        ]
    );
    let mut v = good(System::Windows);
    v.malware = Malware::Detected { name: "x".into() };
    v.format = FormatFact::Wrong {
        found: Format::WebPage,
    };
    assert_eq!(why(&v), vec![Why::MalwareFound, Why::WrongKindOfFile]);
}

#[test]
fn plain_reasons_are_distinct_and_free_of_jargon() {
    let all = [
        Why::ChangedDuringCheck,
        Why::MalwareFound,
        Why::NotExpectedBytes,
        Why::WrongKindOfFile,
        Why::SignatureBroken,
        Why::OtherMaker,
        Why::WrongArchitecture,
        Why::Unsigned,
        Why::NotTrusted,
        Why::MakerUnknown,
        Why::ArchitectureUnknown,
        Why::NotFullyChecked,
    ];
    let texts: std::collections::HashSet<_> = all.iter().map(|w| w.plain()).collect();
    assert_eq!(texts.len(), all.len());
    for t in texts {
        for jargon in [
            "hash",
            "SHA",
            "Authenticode",
            "certificate",
            "PE",
            "MSI",
            "digest",
        ] {
            assert!(!t.contains(jargon), "{t}");
        }
    }
}

// The maker comparison.

#[test]
fn windows_same_certificate_or_same_company_in_the_same_place() {
    let old = Maker::Windows(signer(example(), 1));
    assert_eq!(
        compare(&old, &Maker::Windows(signer(example(), 1))),
        SameMaker::Yes {
            by: MatchedBy::SameCertificate
        }
    );
    // A renewed certificate.
    assert_eq!(
        compare(&old, &Maker::Windows(signer(example(), 2))),
        SameMaker::Yes {
            by: MatchedBy::SameCompany
        }
    );
    // Same certificate wins even if the subject reads differently.
    let odd = signer(Subject::default(), 1);
    assert!(matches!(
        compare(&old, &Maker::Windows(odd)),
        SameMaker::Yes { .. }
    ));
}

#[test]
fn windows_a_look_alike_company_is_another_maker() {
    // T21-030, and T21-001: a familiar name is not enough.
    let old = Maker::Windows(signer(example(), 1));
    type Change = Box<dyn Fn(&mut Subject)>;
    let changes: Vec<Change> = vec![
        Box::new(|s| s.common_name = Some("Example Ltd.".into())),
        Box::new(|s| s.organization = Some("Example Limited".into())),
        Box::new(|s| s.country = Some("US".into())),
        Box::new(|s| s.locality = Some("Abuja".into())),
        Box::new(|s| s.state = Some("FCT".into())),
        Box::new(|s| s.organization = None),
        Box::new(|s| s.common_name = None),
        Box::new(|s| s.country = None),
    ];
    for change in changes {
        let mut s = example();
        change(&mut s);
        let found = Maker::Windows(signer(s.clone(), 2));
        assert!(
            matches!(compare(&old, &found), SameMaker::No { .. }),
            "{s:?}"
        );
    }
    // Blank names never match each other.
    let blank = subject(" ", " ", "x", "x", "x");
    let a = Maker::Windows(signer(blank.clone(), 1));
    assert!(matches!(
        compare(&a, &Maker::Windows(signer(blank, 2))),
        SameMaker::No { .. }
    ));
}

#[test]
fn windows_extended_validation_parts_count_where_both_have_them() {
    let mut ev = example();
    ev.registration_number = Some("RC123".into());
    ev.jurisdiction_country = Some("NG".into());
    ev.jurisdiction_state = Some("Lagos".into());
    let old = Maker::Windows(signer(ev.clone(), 1));
    // A renewal without extended validation.
    assert!(matches!(
        compare(&old, &Maker::Windows(signer(example(), 2))),
        SameMaker::Yes { .. }
    ));
    // Missing locality and state on one side do not block either.
    let mut bare = example();
    bare.locality = None;
    bare.state = None;
    assert!(matches!(
        compare(&old, &Maker::Windows(signer(bare, 2))),
        SameMaker::Yes { .. }
    ));
    for change in [
        |s: &mut Subject| s.registration_number = Some("RC999".into()),
        |s: &mut Subject| s.jurisdiction_country = Some("GH".into()),
        |s: &mut Subject| s.jurisdiction_state = Some("Ogun".into()),
    ] {
        let mut other = ev.clone();
        change(&mut other);
        assert!(matches!(
            compare(&old, &Maker::Windows(signer(other, 2))),
            SameMaker::No { .. }
        ));
    }
}

#[test]
fn mac_by_team_id() {
    let team = |t: &str| Maker::Mac(MacTeam { team_id: t.into() });
    assert_eq!(
        compare(&team("SKMME9E2Y8"), &team("SKMME9E2Y8")),
        SameMaker::Yes {
            by: MatchedBy::SameTeam
        }
    );
    assert!(matches!(
        compare(&team("SKMME9E2Y8"), &team("ABCDE12345")),
        SameMaker::No { .. }
    ));
    // A malformed Team ID never matches, even itself.
    assert!(matches!(
        compare(&team("short"), &team("short")),
        SameMaker::No { .. }
    ));
    assert!(valid_team_id("ABCDE12345"));
    assert!(!valid_team_id("abcde12345"));
    assert!(!valid_team_id("ABCDE1234"));
    assert!(!valid_team_id("ABCDE123456"));
    assert!(!valid_team_id("ABCDE1234-"));
}

#[test]
fn linux_by_key() {
    let rpm = |k: &str| Maker::Linux(LinuxSource::Rpm { key: k.into() });
    let full = "5d4a1b2c3d4e5f60718293a4b5c6d7e824901d0c";
    let yes = SameMaker::Yes {
        by: MatchedBy::SameKey,
    };
    assert_eq!(compare(&rpm(full), &rpm(full)), yes);
    assert_eq!(compare(&rpm(full), &rpm("24901D0C")), yes);
    assert_eq!(compare(&rpm("24901d0c"), &rpm(full)), yes);
    assert_eq!(compare(&rpm("a5c6d7e824901d0c"), &rpm("24901d0c")), yes);
    assert!(matches!(
        compare(&rpm(full), &rpm("00000000")),
        SameMaker::No { .. }
    ));
    assert!(matches!(
        compare(&rpm("24901d0"), &rpm("24901d0")),
        SameMaker::No { .. }
    ));
    assert!(matches!(
        compare(&rpm("2490zzzz"), &rpm("2490zzzz")),
        SameMaker::No { .. }
    ));
    assert!(matches!(
        compare(&rpm(""), &rpm(full)),
        SameMaker::No { .. }
    ));

    let flat = |u: &str, a: &str, k: u8| {
        Maker::Linux(LinuxSource::Flatpak {
            remote_url: u.into(),
            app_id: a.into(),
            key: Sha256([k; 32]),
        })
    };
    let old = flat("https://dl.flathub.org/repo/", "org.example.App", 1);
    assert_eq!(compare(&old, &old.clone()), yes);
    for other in [
        flat("https://evil.example/repo/", "org.example.App", 1),
        flat("https://dl.flathub.org/repo/", "org.example.App2", 1),
        flat("https://dl.flathub.org/repo/", "org.example.App", 2),
    ] {
        assert!(matches!(compare(&old, &other), SameMaker::No { .. }));
    }
    assert!(matches!(compare(&old, &rpm(full)), SameMaker::No { .. }));
}

#[test]
fn makers_of_different_systems_are_unknown() {
    let w = Maker::Windows(signer(example(), 1));
    let m = Maker::Mac(MacTeam {
        team_id: "ABCDE12345".into(),
    });
    assert!(matches!(compare(&w, &m), SameMaker::Unknown { .. }));
    assert!(matches!(
        compare(
            &m,
            &Maker::Linux(LinuxSource::Rpm {
                key: "24901d0c".into()
            })
        ),
        SameMaker::Unknown { .. }
    ));
}

#[test]
fn subjects_are_described_plainly() {
    assert_eq!(
        describe_subject(&example()),
        "Example Ltd, Example Ltd, Lagos, Lagos, NG"
    );
    assert_eq!(
        describe_subject(&Subject::default()),
        "a signer with no name"
    );
    let s = Sha256([0xab; 32]).to_string();
    assert_eq!(s.len(), 64);
    assert!(s.starts_with("abab"));
    assert!(format!("{:?}", Sha256([1; 32])).starts_with("Sha256(0101"));
}
