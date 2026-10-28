//! The system's own danger check only adds caution, and only to the system's own facts.
use pctwin_settings::judge::{Facts, use_system_danger_check};

fn says_ps1(e: &str) -> bool {
    e == "ps1"
}

#[test]
fn the_systems_check_adds_caution_to_the_system_facts_only() {
    use_system_danger_check(says_ps1);
    assert!(Facts::system().ending_runs(".ps1"));
    assert!(Facts::system().ending_runs("PS1"));
    // Facts given directly stay exactly what they say.
    let given = Facts::new(["exe"], None);
    assert!(!given.ending_runs("ps1"));
    assert!(given.ending_runs("exe"));
    // A later check cannot replace the first.
    use_system_danger_check(|_| false);
    assert!(Facts::system().ending_runs("ps1"));
}
