use n3gb_rs::{HexCell, N3gbError, decode_hex_identifier};

fn main() -> Result<(), N3gbError> {
    let lon = -2.2479699500757597;
    let lat = 53.48082746395233;

    let cell = HexCell::from_wgs84(lon, lat, 12)?;

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

    Ok(())
}
