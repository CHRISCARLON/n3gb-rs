/// Creates a hex grid clipped to a polygon boundary.
///
/// Demonstrates building a grid over a defined area (rather than a bounding box),
/// so only cells that actually intersect the polygon are included.
///
/// Run with:
///   cargo run --example grid_from_polygon [zoom]
use geo_types::{LineString, Polygon, coord};
use n3gb_rs::{ConversionMethod, HexGrid, N3gbError};

fn main() -> Result<(), N3gbError> {
    let zoom: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);

    // Roughly the outline of Greater Manchester in WGS84
    let polygon = Polygon::new(
        LineString::from(vec![
            coord! { x: -2.450, y: 53.350 },
            coord! { x: -1.950, y: 53.350 },
            coord! { x: -1.950, y: 53.650 },
            coord! { x: -2.450, y: 53.650 },
            coord! { x: -2.450, y: 53.350 },
        ]),
        vec![],
    );

    println!("Building hex grid clipped to polygon...");
    println!("  Zoom level: {}", zoom);
    println!("  Backend:    Ostn15");

    let grid = HexGrid::builder()
        .zoom_level(zoom)
        .conversion_method(ConversionMethod::Ostn15)
        .wgs84_polygon(polygon)?
        .build()?;

    println!("  Cells: {}", grid.len());

    Ok(())
}
