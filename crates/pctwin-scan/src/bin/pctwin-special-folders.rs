//! Prints where each special folder of the signed-in person really is, as JSON. Used by the tests
//! to check the answer in a fresh process with its own settings, and handy for diagnosing a laptop.

fn main() {
    let found = pctwin_scan::find_special_folders();
    match serde_json::to_string_pretty(&found) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("could not write the answer: {e}");
            std::process::exit(1);
        }
    }
}
