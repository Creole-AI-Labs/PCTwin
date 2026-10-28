//! How long the old laptop's reading takes here (read only; prints counts and times, never values).
fn main() {
    let home = std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default());
    let t = std::time::Instant::now();
    let x = pctwin_fullmove::extras::here(Vec::new());
    println!("settings: {} found, {:?}, {} bytes", x.settings.len(), t.elapsed(), x.encode().len());
    let t = std::time::Instant::now();
    let f = pctwin_fullmove::extras::special_here(&pctwin_record::Owner::Shared, &home, &[]);
    println!(
        "special: {} lines, {} not moved, {} not fully checked, {:?}",
        f.lines.len(),
        f.not_moved,
        f.not_checked,
        t.elapsed()
    );
}
