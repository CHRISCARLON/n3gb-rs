use n3gb_rs::{Crs, CsvHexConfig, CsvToHex, GeometryFormat};

fn main() {
    let csv_path = std::env::args()
        .nth(1)
        .expect("Usage: csv_hex_density <input.csv> [output.csv]");

    let output_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "hex-density.csv".to_string());

    let config = CsvHexConfig::from_coords("X_COORDINATE", "Y_COORDINATE", 6)
        .crs(Crs::Bng)
        .with_hex_geometry(GeometryFormat::Wkt)
        .hex_density();

    println!("Generating hex density CSV...");
    println!("  Input:  {}", csv_path);
    println!("  Output: {}", output_path);
    println!("  Zoom level: {}", config.zoom_level);

    match csv_path.to_hex_csv(&output_path, &config) {
        Ok(()) => println!("Done!"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
