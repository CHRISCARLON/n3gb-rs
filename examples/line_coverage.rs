/// Finds all hex cells that a line passes through.
///
/// Useful for corridor analysis — e.g. a pipeline route, road, or river.
/// Demonstrates HexCell::from_line_string_bng and from_line_string_wgs84.
///
/// Run with:
///   cargo run --example line_coverage [zoom]
use geo_types::LineString;
use n3gb_rs::{ConversionMethod, HexCell, N3gbError};

fn main() -> Result<(), N3gbError> {
    let zoom: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    println!("=== BNG line: London to Birmingham ===");
    let bng_line: LineString = vec![
        (530000.0, 180000.0), // Central London
        (510000.0, 220000.0),
        (490000.0, 260000.0),
        (408000.0, 287000.0), // Birmingham city centre
    ]
    .into();

    let cells = HexCell::from_line_string_bng(&bng_line, zoom)?;
    println!("  Zoom level: {}", zoom);
    println!("  Cells crossed: {}", cells.len());
    for cell in cells.iter().take(5) {
        println!(
            "    {} ({:.0}, {:.0})",
            cell.id,
            cell.easting(),
            cell.northing()
        );
    }
    if cells.len() > 5 {
        println!("    ... ({} more)", cells.len() - 5);
    }

    // --- WGS84 line: same route ---
    println!("\n=== WGS84 line: same route (not 100% match) (Ostn15 backend) ===");
    let wgs84_line: LineString = vec![
        (-0.1276, 51.508), // London
        (-0.5000, 52.000),
        (-1.0000, 52.300),
        (-1.8998, 52.4862), // Birmingham
    ]
    .into();

    let cells_wgs84 = HexCell::from_line_string_wgs84(&wgs84_line, zoom, ConversionMethod::Ostn15)?;
    println!("  Zoom level: {}", zoom);
    println!("  Cells crossed: {}", cells_wgs84.len());
    for cell in cells_wgs84.iter().take(5) {
        println!(
            "    {} ({:.0}, {:.0})",
            cell.id,
            cell.easting(),
            cell.northing()
        );
    }
    if cells_wgs84.len() > 5 {
        println!("    ... ({} more)", cells_wgs84.len() - 5);
    }

    Ok(())
}
