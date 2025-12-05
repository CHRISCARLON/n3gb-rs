use std::time::Instant;
use geo_types::Point;
use n3gb_rs::{HexCell, HexGrid, N3gbError, decode_hex_identifier};

fn main() -> Result<(), N3gbError> {
    let wgs84_coord = Point::new(-2.2479699500757597, 53.48082746395233);

    let cell = HexCell::from_wgs84(&wgs84_coord, 12)?;

    println!("Hex ID: {}", cell.id);
    println!("Center: ({}, {})", cell.easting(), cell.northing());
    println!("Row: {}, Col: {}", cell.row, cell.col);

    let polygon = cell.to_polygon();
    println!("Polygon: {:?}", polygon);

    let (version, easting, northing, zoom_level) = decode_hex_identifier(&cell.id)?;
    println!("\nDecoded from ID:");
    println!("  Version: {}", version);
    println!("  Easting: {}", easting);
    println!("  Northing: {}", northing);
    println!("  Zoom: {}", zoom_level);

    let start = Instant::now();
    let grid = HexGrid::builder()
        .zoom_level(12)
        .bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0))
        .build();
    let elapsed = start.elapsed();

    println!("\nGrid generated: {} cells in {:?}", grid.len(), elapsed);

    Ok(())
}
