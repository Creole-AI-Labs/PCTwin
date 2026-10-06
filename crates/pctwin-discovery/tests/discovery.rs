//! Finding each other on the same network (Product Spec, Finding each other).
//!
//! These tests use real multicast DNS on this computer. Each test uses its own unique name so
//! tests running at the same time do not see each other's announcements by mistake.

use std::time::{Duration, Instant};

use pctwin_discovery::{Discovery, Found, Label, SERVICE_TYPE};

const WAIT: Duration = Duration::from_secs(3);

fn unique(name: &str) -> Label {
    let mut tag = [0u8; 4];
    getrandom::fill(&mut tag).unwrap();
    Label::named(&format!(
        "{name} {:02x}{:02x}{:02x}{:02x}",
        tag[0], tag[1], tag[2], tag[3]
    ))
    .unwrap()
}

fn find<'a>(found: &'a [Found], label: &Label) -> Vec<&'a Found> {
    found.iter().filter(|f| &f.label == label).collect()
}

#[test]
fn the_new_laptop_finds_the_old_laptop_and_where_to_connect() {
    let old = Discovery::new().unwrap();
    let label = unique("old laptop");
    let _announcing = old.announce(&label, 47_123).unwrap();

    let new = Discovery::new().unwrap();
    let found = new.browse(WAIT).unwrap();
    let mine = find(&found, &label);
    assert_eq!(mine.len(), 1, "found exactly once: {found:?}");
    assert!(!mine[0].addrs.is_empty());
    assert!(mine[0].addrs.iter().all(|a| a.port() == 47_123));
    assert!(!mine[0].same_label_nearby);
}

#[test]
fn a_random_label_is_found_as_the_same_colour_and_animal() {
    let old = Discovery::new().unwrap();
    // Random labels cannot be made unique per test, so match on this test's own port too.
    let label = Label::random().unwrap();
    let _announcing = old.announce(&label, 47_124).unwrap();
    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    assert!(
        find(&found, &label)
            .iter()
            .any(|f| f.addrs.iter().all(|a| a.port() == 47_124))
    );
}

#[test]
fn once_pairing_stops_the_old_laptop_is_no_longer_announced() {
    let old = Discovery::new().unwrap();
    let label = unique("gone soon");
    let announcing = old.announce(&label, 47_125).unwrap();
    assert_eq!(
        find(&Discovery::new().unwrap().browse(WAIT).unwrap(), &label).len(),
        1
    );
    drop(announcing);
    // Give the goodbye a moment to go out, then a fresh search must not find it.
    std::thread::sleep(Duration::from_millis(500));
    assert!(find(&Discovery::new().unwrap().browse(WAIT).unwrap(), &label).is_empty());
}

#[test]
fn two_old_laptops_with_the_same_label_are_both_flagged() {
    let a = Discovery::new().unwrap();
    let b = Discovery::new().unwrap();
    let label = unique("twin");
    let _one = a.announce(&label, 47_126).unwrap();
    let _two = b.announce(&label, 47_127).unwrap();
    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    let twins = find(&found, &label);
    assert_eq!(twins.len(), 2, "{found:?}");
    assert!(twins.iter().all(|f| f.same_label_nearby));
}

#[test]
fn the_announcement_carries_only_the_label_and_the_port() {
    let old = Discovery::new().unwrap();
    let label = unique("private");
    let _announcing = old.announce(&label, 47_128).unwrap();

    // Look at the raw announcement as any device on the network would.
    let daemon = mdns_sd::ServiceDaemon::new().unwrap();
    let events = daemon.browse(SERVICE_TYPE).unwrap();
    let real_name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
        .to_lowercase();
    let deadline = Instant::now() + WAIT;
    let mut seen = false;
    while let Ok(event) = events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
            if info.get_property_val_str("n") != label_name(&label) {
                continue;
            }
            seen = true;
            let mut keys: Vec<String> = info
                .get_properties()
                .iter()
                .map(|p| p.key().to_string())
                .collect();
            keys.sort();
            assert_eq!(keys, ["n", "v"], "nothing but the label and the version");
            let host = info.get_hostname().to_lowercase();
            let instance = info.get_fullname().to_lowercase();
            assert!(host.starts_with("pctwin-"), "{host}");
            if !real_name.is_empty() {
                assert!(!host.contains(&real_name), "real name leaked in {host}");
                assert!(
                    !instance.contains(&real_name),
                    "real name leaked in {instance}"
                );
            }
            break;
        }
    }
    let _ = daemon.shutdown();
    assert!(seen, "the announcement was seen");
}

#[test]
fn announcements_with_broken_labels_are_ignored() {
    // Someone else's device announces a PCTwin service with a nonsense label.
    let daemon = mdns_sd::ServiceDaemon::new().unwrap();
    let mut tag = [0u8; 4];
    getrandom::fill(&mut tag).unwrap();
    let instance = format!("pctwin-junk-{:02x}{:02x}", tag[0], tag[1]);
    let info = mdns_sd::ServiceInfo::new(
        SERVICE_TYPE,
        &instance,
        &format!("{instance}.local."),
        "",
        47_129,
        &[("v", "1"), ("c", "99"), ("a", "7")][..],
    )
    .unwrap()
    .enable_addr_auto();
    daemon.register(info).unwrap();

    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    assert!(
        found
            .iter()
            .all(|f| f.addrs.iter().all(|a| a.port() != 47_129)),
        "{found:?}"
    );
    let _ = daemon.shutdown();
}

fn label_name(label: &Label) -> Option<&str> {
    match label {
        Label::Named(n) => Some(n.as_str()),
        Label::Picked { .. } => None,
    }
}

#[test]
fn an_old_laptop_that_stops_during_the_search_drops_out_of_the_list() {
    let label = unique("leaves mid-search");
    let old = Discovery::new().unwrap();
    let announcing = old.announce(&label, 47_130).unwrap();
    let searching = std::thread::spawn(|| Discovery::new().unwrap().browse(Duration::from_secs(5)));
    // Seen first, then it stops pairing while the search is still running.
    std::thread::sleep(Duration::from_millis(2500));
    drop(announcing);
    let found = searching.join().unwrap().unwrap();
    assert!(find(&found, &label).is_empty(), "{found:?}");
}

#[test]
fn two_old_laptops_showing_the_same_colour_and_animal_are_both_flagged() {
    let label = Label::random().unwrap();
    let a = Discovery::new().unwrap();
    let b = Discovery::new().unwrap();
    let _one = a.announce(&label, 47_131).unwrap();
    let _two = b.announce(&label, 47_132).unwrap();
    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    let mine: Vec<&Found> = found
        .iter()
        .filter(|f| f.addrs.iter().all(|a| [47_131, 47_132].contains(&a.port())))
        .collect();
    assert_eq!(mine.len(), 2, "{found:?}");
    assert!(mine.iter().all(|f| f.same_label_nearby));
}

#[test]
fn names_differing_only_in_case_are_flagged_as_the_same() {
    let lower = unique("twin case");
    let Label::Named(name) = &lower else {
        unreachable!()
    };
    let upper = Label::named(&name.to_uppercase()).unwrap();
    let a = Discovery::new().unwrap();
    let b = Discovery::new().unwrap();
    let _one = a.announce(&lower, 47_133).unwrap();
    let _two = b.announce(&upper, 47_134).unwrap();
    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    let mine: Vec<&Found> = found
        .iter()
        .filter(|f| f.addrs.iter().all(|a| [47_133, 47_134].contains(&a.port())))
        .collect();
    assert_eq!(mine.len(), 2, "{found:?}");
    assert!(mine.iter().all(|f| f.same_label_nearby));
}

#[test]
fn closing_discovery_stops_its_announcements() {
    let label = unique("closing");
    let old = Discovery::new().unwrap();
    let announcing = old.announce(&label, 47_135).unwrap();
    assert_eq!(
        find(&Discovery::new().unwrap().browse(WAIT).unwrap(), &label).len(),
        1
    );
    drop(old); // the app leaves the pairing screen
    std::thread::sleep(Duration::from_millis(500));
    assert!(find(&Discovery::new().unwrap().browse(WAIT).unwrap(), &label).is_empty());
    drop(announcing);
}

#[test]
fn found_addresses_are_on_this_network_and_never_loopback() {
    let label = unique("addresses");
    let old = Discovery::new().unwrap();
    let _announcing = old.announce(&label, 47_136).unwrap();
    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    let mine = find(&found, &label);
    assert_eq!(mine.len(), 1);
    assert!(mine[0].addrs.len() <= pctwin_discovery::MAX_ADDRS);
    for a in &mine[0].addrs {
        assert!(!a.ip().is_loopback(), "{a}");
        assert!(pctwin_link::is_local_peer(a.ip()), "{a}");
    }
}

#[test]
fn an_old_laptop_whose_announcement_turns_invalid_drops_out_of_the_list() {
    let label = unique("turns invalid");
    let Label::Named(name) = &label else {
        unreachable!()
    };
    let daemon = mdns_sd::ServiceDaemon::new().unwrap();
    let mut tag = [0u8; 4];
    getrandom::fill(&mut tag).unwrap();
    let instance = format!(
        "pctwin-change-{:02x}{:02x}{:02x}{:02x}",
        tag[0], tag[1], tag[2], tag[3]
    );
    let host = format!("{instance}.local.");
    let announce = |fields: &[(&str, &str)]| {
        let info = mdns_sd::ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", 47_137, fields)
            .unwrap()
            .enable_addr_auto();
        daemon.register(info).unwrap();
    };
    announce(&[("v", "1"), ("n", name.as_str())]);
    let searching = std::thread::spawn(|| Discovery::new().unwrap().browse(Duration::from_secs(5)));
    std::thread::sleep(Duration::from_millis(2500));
    // The same laptop now announces something PCTwin would never write.
    announce(&[("v", "1"), ("n", name.as_str()), ("x", "1")]);
    let found = searching.join().unwrap().unwrap();
    let _ = daemon.shutdown();
    assert!(
        found
            .iter()
            .all(|f| f.addrs.iter().all(|a| a.port() != 47_137)),
        "{found:?}"
    );
}

#[test]
fn the_longest_name_allowed_can_be_announced_and_found() {
    let mut name = "x\u{301}\u{301}\u{301}\u{301}".repeat(26);
    name.push_str("x\u{301}\u{301}x");
    assert_eq!(name.len(), 240);
    let label = Label::named(&name).unwrap();
    let old = Discovery::new().unwrap();
    let _announcing = old.announce(&label, 47_138).unwrap();
    let found = Discovery::new().unwrap().browse(WAIT).unwrap();
    assert_eq!(find(&found, &label).len(), 1);
}
