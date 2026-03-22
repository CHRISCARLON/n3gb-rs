/// Checks whether PROJ has OSTN15 grid files installed and compares both
/// conversion backends on a set of known UK coordinates.
///
/// Sets PROJ_DEBUG=2 before any conversion runs so libproj logs which
/// pipeline and grid files it selects to stderr.
///
/// Run with:
///   cargo run --example check_proj
use n3gb_rs::{ConversionMethod, HexCell};

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
        let proj = HexCell::from_wgs84(&(*lon, *lat), 12, ConversionMethod::Proj)
            .expect("proj conversion failed");
        let ostn15 = HexCell::from_wgs84(&(*lon, *lat), 12, ConversionMethod::Ostn15)
            .expect("ostn15 conversion failed");

        let diff_e = (proj.easting() - ostn15.easting()).abs();
        let diff_n = (proj.northing() - ostn15.northing()).abs();
        max_diff = max_diff.max(diff_e).max(diff_n);

        println!(
            "{:<30} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>10.4} {:>10.4}",
            name,
            proj.easting(),
            proj.northing(),
            ostn15.easting(),
            ostn15.northing(),
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
