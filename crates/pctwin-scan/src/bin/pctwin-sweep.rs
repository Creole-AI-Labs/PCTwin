//! Sweeps this laptop for the signed-in person and writes everything found as JSON: the people,
//! each drive's coverage line, the places, the blind spots, the top line and the move record.
//! Only reads. Used by the "nothing forgotten" tests in a fresh process as the person moving, and
//! handy for diagnosing a laptop.
//!
//!     pctwin-sweep <out.json>

use std::process::ExitCode;
use std::sync::atomic::AtomicBool;

fn main() -> ExitCode {
    let Some(out) = std::env::args().nth(1) else {
        eprintln!("usage: pctwin-sweep <out.json>");
        return ExitCode::from(2);
    };
    let laptop = match pctwin_record::LaptopId::generate() {
        Ok(id) => id,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let stop = AtomicBool::new(false);
    let started = std::time::Instant::now();
    let sweep = pctwin_scan::sweep_this_laptop(laptop, &stop, &mut |_| {}, None);
    let json = match serde_json::to_vec_pretty(&sweep) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("could not write the answer: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = std::fs::write(&out, json) {
        eprintln!("could not save {out}: {e}");
        return ExitCode::FAILURE;
    }
    println!("{}", sweep.top_line.words());
    println!(
        "{} items, {} places, {} blind spots, {} volumes, {:.1} s",
        sweep.record.items.len(),
        sweep.places.len(),
        sweep.blind_spots.len(),
        sweep.volumes.len(),
        started.elapsed().as_secs_f64()
    );
    ExitCode::SUCCESS
}
