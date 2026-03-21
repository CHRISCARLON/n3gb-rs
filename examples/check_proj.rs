/// Checks whether PROJ has OSTN15 grid files installed and compares both
/// conversion backends on a set of known UK coordinates.
///
/// Sets PROJ_DEBUG=2 before any conversion runs so libproj logs which
/// pipeline and grid files it selects to stderr.
///
/// Run with:
///   cargo run --example check_proj
use n3gb_rs::{wgs84_to_bng, wgs84_to_bng_ostn15};

fn main() {
    // Must be set before any proj object is created
    unsafe {
        std::env::set_var("PROJ_DEBUG", "2");
    }

    let locations = vec![
        ("London (Trafalgar Sq)", -0.1276, 51.5080),
        ("Manchester (Piccadilly)", -2.2317, 53.4808),
        ("Edinburgh (Castle)", -3.1997, 55.9486),
        ("Cardiff (City Centre)", -3.1791, 51.4816),
        ("Bristol (Temple Meads)", -2.5830, 51.4483),
    ];

    eprintln!("--- PROJ debug output ---");

    let mut max_diff: f64 = 0.0;

    println!(
        "{:<30} {:>12} {:>12} {:>12} {:>12} {:>10} {:>10}",
        "Location", "proj E", "proj N", "ostn15 E", "ostn15 N", "diff E(m)", "diff N(m)"
    );
    println!("{}", "-".repeat(102));

    for (name, lon, lat) in &locations {
        let proj = wgs84_to_bng(&(*lon, *lat)).expect("proj conversion failed");
        let ostn15 = wgs84_to_bng_ostn15(&(*lon, *lat)).expect("ostn15 conversion failed");

        let diff_e = (proj.x() - ostn15.x()).abs();
        let diff_n = (proj.y() - ostn15.y()).abs();
        max_diff = max_diff.max(diff_e).max(diff_n);

        println!(
            "{:<30} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>10.4} {:>10.4}",
            name,
            proj.x(),
            proj.y(),
            ostn15.x(),
            ostn15.y(),
            diff_e,
            diff_n,
        );
    }

    println!();
    if max_diff < 1.0 {
        println!(
            "✓ PROJ grid files active — both backends agree to within {:.4}m",
            max_diff
        );
    } else {
        println!(
            "✗ Max diff {:.3}m — PROJ is likely using Helmert fallback (no grid files)",
            max_diff
        );
        println!("  Install grid files: projsync --source-id uk_os");
        println!("  Or use ConversionMethod::Ostn15 for guaranteed accuracy.");
    }
}
