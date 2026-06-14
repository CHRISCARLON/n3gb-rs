/// Finds all hex cells that a line passes through.
///
/// Useful for corridor analysis — e.g. a pipeline route, road, or river.
/// Demonstrates `HexCell::from_line_string_bng` and `from_line_string_wgs84`.
///
/// The route is defined ONCE in BNG, and the equivalent WGS84 line is derived by
/// reprojecting it. Feeding each into the two entry points should produce nearly
/// identical cell sets: the conversion is essentially exact (see the `check_proj`
/// example — the backends agree to <1mm when PROJ grid files are present), so any
/// remaining difference is boundary-sampling, not coordinate error.
///
/// Run with:
///   cargo run --example line_coverage [zoom]
use std::collections::HashSet;

use geo_types::LineString;
use n3gb_rs::{ConversionMethod, HexCell, N3gbError};
use proj::Proj;

fn main() -> Result<(), N3gbError> {
    let zoom: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    // The route, defined once in BNG (easting, northing).
    let route_bng = [
        (530046.0, 180452.0), // London (Trafalgar Square)
        (510000.0, 220000.0),
        (490000.0, 260000.0),
        (407500.0, 286000.0), // Birmingham
    ];
    let bng_line: LineString = route_bng.to_vec().into();

    // Derive the SAME route in WGS84 by reprojecting each vertex BNG -> WGS84.
    // (PROJ normalizes to lon/lat axis order, matching what the WGS84 entry point
    // expects.)
    let to_wgs84 = Proj::new_known_crs("EPSG:27700", "EPSG:4326", None)
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;
    let wgs84_vertices: Result<Vec<(f64, f64)>, N3gbError> = route_bng
        .iter()
        .map(|&(e, n)| {
            to_wgs84
                .convert((e, n))
                .map_err(|err| N3gbError::ProjectionError(err.to_string()))
        })
        .collect();
    let wgs84_line: LineString = wgs84_vertices?.into();

    // Cover the line from each entry point.
    let cells_bng = HexCell::from_line_string_bng(&bng_line, zoom)?;
    let cells_wgs84 = HexCell::from_line_string_wgs84(&wgs84_line, zoom, ConversionMethod::Ostn15)?;

    println!(
        "=== Line coverage: London -> Birmingham (zoom {}) ===",
        zoom
    );
    println!("  Route defined in BNG; WGS84 line derived by reprojection.\n");
    println!("  from_line_string_bng:   {} cells", cells_bng.len());
    println!("  from_line_string_wgs84: {} cells", cells_wgs84.len());

    // Compare the two cell sets by ID.
    let set_bng: HashSet<&str> = cells_bng.iter().map(|c| c.id.as_str()).collect();
    let set_wgs84: HashSet<&str> = cells_wgs84.iter().map(|c| c.id.as_str()).collect();
    let shared = set_bng.intersection(&set_wgs84).count();
    let only_bng = set_bng.difference(&set_wgs84).count();
    let only_wgs84 = set_wgs84.difference(&set_bng).count();
    let distinct = shared + only_bng + only_wgs84;

    println!("\n  Shared cells:         {}", shared);
    println!("  Only in BNG result:   {}", only_bng);
    println!("  Only in WGS84 result: {}", only_wgs84);
    println!(
        "  Agreement: {:.2}% ({} shared of {} distinct)",
        100.0 * shared as f64 / distinct as f64,
        shared,
        distinct
    );
    println!(
        "\n  Any non-shared cells would be boundary-sampling artifacts, not\n  \
         conversion error — see `cargo run --example check_proj`."
    );

    // Show the first few cells of the BNG result.
    println!("\n  First cells (BNG):");
    for cell in cells_bng.iter().take(5) {
        println!(
            "    {} ({:.0}, {:.0})",
            cell.id,
            cell.easting(),
            cell.northing()
        );
    }
    if cells_bng.len() > 5 {
        println!("    ... ({} more)", cells_bng.len() - 5);
    }

    Ok(())
}
