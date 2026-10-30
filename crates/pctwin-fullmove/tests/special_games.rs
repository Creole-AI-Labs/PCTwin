//! The move's special-category scan takes games (2.5): saves the shipped rows name are found by
//! their places, and the app data holding them is shown as a game with its notes. Every byte
//! here is invented.

use std::time::{Duration, Instant};

use pctwin_record::Owner;
use pctwin_special::category::Category;
use pctwin_special::roots::System;

fn put(p: &std::path::Path, b: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, b).unwrap();
}

#[test]
fn a_games_saves_in_app_data_show_as_a_game() {
    pctwin_parse::testing::install_readers();
    let t = tempfile::tempdir().unwrap();
    let home = t.path().join("home");
    // A shipped row's save place (an example game), under the app data root for local files.
    put(
        &home.join("AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat"),
        &[3; 64],
    );
    let owner = Owner::Person {
        account_id: "S-1-5-21-0-0-0-1001".into(),
    };
    let found = pctwin_fullmove::extras::special_of(
        System::Windows,
        &owner,
        &home,
        &[],
        Instant::now() + Duration::from_secs(120),
    );
    let game = found
        .lines
        .iter()
        .find(|l| l.category == Category::Game)
        .unwrap_or_else(|| panic!("no game line: {:?}", found.lines));
    assert!(game.files >= 1);
    // Without the games part the same folder is plain app data.
    assert!(found.planned.iter().any(|p| p.category == Category::Game
        && p.component.members.iter().any(|m| m.ends_with("user1.dat"))));
}
