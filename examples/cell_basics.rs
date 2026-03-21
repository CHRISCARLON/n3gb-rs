/// Demonstrates basic HexCell creation and inspection.
///
/// Shows how to create cells from BNG and WGS84 coordinates, inspect
/// their properties, and compare the same location across zoom levels.
///
/// Run with:
///   cargo run --example cell_basics
use n3gb_rs::{ConversionMethod, HexCell, N3gbError};

fn main() -> Result<(), N3gbError> {
    // --- From BNG coordinates ---
    println!("=== From BNG coordinates ===");
    let bng_coord = (530000.0_f64, 180000.0_f64); // Central London
    let cell = HexCell::from_bng(&bng_coord, 10)?;

    println!("  ID:       {}", cell.id);
    println!("  Zoom:     {}", cell.zoom_level);
    println!("  Easting:  {:.1}", cell.easting());
    println!("  Northing: {:.1}", cell.northing());
    println!("  Row:      {}", cell.row);
    println!("  Col:      {}", cell.col);

    // --- Round-trip: ID back to cell ---
    println!("\n=== Round-trip from ID ===");
    let from_id = HexCell::from_hex_id(&cell.id)?;
    println!("  Original ID:   {}", cell.id);
    println!("  Round-trip ID: {}", from_id.id);
    println!("  Match: {}", cell.id == from_id.id);

    // --- From WGS84 coordinates ---
    println!("\n=== From WGS84 coordinates (Ostn15 backend) ===");
    let wgs84_coord = (-0.1276_f64, 51.508_f64); // Trafalgar Square
    let cell_wgs84 = HexCell::from_wgs84(&wgs84_coord, 10, ConversionMethod::Ostn15)?;

    println!("  ID:       {}", cell_wgs84.id);
    println!("  Easting:  {:.1}", cell_wgs84.easting());
    println!("  Northing: {:.1}", cell_wgs84.northing());

    // --- Same point at different zoom levels ---
    println!("\n=== Same point across zoom levels ===");
    println!(
        "  {:<6} {:<16} {:>12} {:>12}",
        "Zoom", "ID", "Easting", "Northing"
    );
    println!("  {}", "-".repeat(52));

    for zoom in [4, 6, 8, 10, 12] {
        let c = HexCell::from_bng(&bng_coord, zoom)?;
        println!(
            "  {:<6} {:<16} {:>12.1} {:>12.1}",
            zoom,
            c.id,
            c.easting(),
            c.northing()
        );
    }

    Ok(())
}
