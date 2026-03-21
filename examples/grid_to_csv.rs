use n3gb_rs::{HexGrid, N3gbError};
use std::fs::File;
use std::io::Write;
use wkt::ToWkt;

fn main() -> Result<(), N3gbError> {
    let zoom: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let output_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "manchester-grid.csv".to_string());

    // Manchester city centre area in BNG (roughly)
    let min = (380000.0, 393000.0);
    let max = (390000.0, 403000.0);

    println!("Generating hex grid CSV...");
    println!("  Extent: ({}, {}) to ({}, {})", min.0, min.1, max.0, max.1);
    println!("  Zoom level: {}", zoom);

    let grid = HexGrid::from_bng_extent(&min, &max, zoom)?;
    println!("  Cells: {}", grid.len());

    let mut file = File::create(&output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;

    writeln!(file, "hex_id,zoom_level,row,col,easting,northing,geometry")
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    for cell in &grid {
        let polygon = cell.to_polygon();
        writeln!(
            file,
            "{},{},{},{},{},{},\"{}\"",
            cell.id,
            cell.zoom_level,
            cell.row,
            cell.col,
            cell.easting(),
            cell.northing(),
            polygon.wkt_string()
        )
        .map_err(|e| N3gbError::IoError(e.to_string()))?;
    }

    println!("  Output: {}", output_path);
    println!("Done!");
    Ok(())
}
