/// Demonstrates every way to create a HexCell, and how to inspect one.
///
/// Covers the typed constructors for known input (`from_bng`, `from_wgs84`,
/// `from_line_string_bng`, `from_hex_id`) and the general-purpose dispatcher
/// (`from_geometry`) for arbitrary or parsed geometry whose type is only known
/// at runtime.
///
/// Run with:
///   cargo run --example cell_basics
use n3gb_rs::geo_types::LineString;
use n3gb_rs::{ConversionMethod, Crs, HexCell, N3gbError, parse_geometry};

fn main() -> Result<(), N3gbError> {
    // --- 1. From a known BNG point (typed, returns a single cell) ---
    println!("=== from_bng: known BNG point ===");
    let bng_coord = (530000.0_f64, 180000.0_f64); // Central London
    let cell = HexCell::from_bng(&bng_coord, 10)?;
    println!("  ID:       {}", cell.id);
    println!("  Zoom:     {}", cell.zoom_level);
    println!("  Easting:  {:.1}", cell.easting());
    println!("  Northing: {:.1}", cell.northing());
    println!("  Row/Col:  {} / {}", cell.row, cell.col);

    // --- 2. From a known WGS84 point (typed; needs a conversion backend) ---
    println!("\n=== from_wgs84: known WGS84 point (Ostn15 backend) ===");
    let wgs84_coord = (-0.1276_f64, 51.508_f64); // Trafalgar Square
    let cell_wgs84 = HexCell::from_wgs84(&wgs84_coord, 10, ConversionMethod::Ostn15)?;
    println!("  ID:       {}", cell_wgs84.id);
    println!("  Easting:  {:.1}", cell_wgs84.easting());
    println!("  Northing: {:.1}", cell_wgs84.northing());

    // --- 3. Round-trip: reconstruct a cell from its ID ---
    println!("\n=== from_hex_id: round-trip from an ID ===");
    let from_id = HexCell::from_hex_id(&cell.id)?;
    println!("  Original ID:   {}", cell.id);
    println!("  Round-trip ID: {}", from_id.id);
    println!("  Match: {}", cell.id == from_id.id);

    // --- 4. From a known line (typed, returns every cell the line crosses) ---
    println!("\n=== from_line_string_bng: a BNG line (returns a Vec) ===");
    let line = LineString::from(vec![
        (530000.0, 180000.0),
        (532000.0, 181500.0),
        (534000.0, 181000.0),
    ]);
    let line_cells = HexCell::from_line_string_bng(&line, 10)?;
    println!("  line crosses {} cell(s)", line_cells.len());
    println!("  first cell ID: {}", line_cells[0].id);

    // --- 5. From arbitrary / parsed geometry (the dynamic front door) ---
    // `from_geometry` is what you reach for when the input type isn't known until
    // runtime — here, geometry parsed from WKT. It always returns a Vec.
    println!("\n=== from_geometry: dispatch on parsed geometry (WKT) ===");
    for wkt in [
        "POINT (530000 180000)",
        "LINESTRING (530000 180000, 533000 182000)",
        "POLYGON ((530000 180000, 531000 180000, 531000 181000, 530000 181000, 530000 180000))",
    ] {
        let geom = parse_geometry(wkt)?;
        let cells = HexCell::from_geometry(geom, 10, Crs::Bng, ConversionMethod::default())?;
        // Point -> 1, Polygon -> 1 (centroid), LineString -> many.
        let kind = wkt.split_whitespace().next().unwrap_or("");
        println!("  {:<11} -> {} cell(s)", kind, cells.len());
    }

    // --- Same point across zoom levels (smaller cells at higher zoom) ---
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
