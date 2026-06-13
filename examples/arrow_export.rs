/// Demonstrates the `HexCellsToArrow` trait.
///
/// The trait is implemented once via a blanket impl for any `T: AsRef<[HexCell]>`,
/// so the same three methods (`to_arrow_points`, `to_arrow_polygons`,
/// `to_record_batch`) work on a `Vec<HexCell>`, a `&[HexCell]` slice, and — through
/// forwarding in `cell.rs` — a single `HexCell`.
///
/// Run with:
///   cargo run --example arrow_export
use n3gb_rs::HexCell;
use n3gb_rs::HexCellsToArrow;
use n3gb_rs::HexGrid;
// `GeoArrowArray` brings `.len()` into scope for the point/polygon arrays.
use n3gb_rs::geoarrow_array::GeoArrowArray;

fn main() -> Result<(), n3gb_rs::N3gbError> {
    // A handful of cells around central London at zoom 10.
    let cells: Vec<HexCell> = vec![
        HexCell::from_bng(&(530000.0, 180000.0), 10)?,
        HexCell::from_bng(&(530500.0, 180500.0), 10)?,
        HexCell::from_bng(&(531000.0, 181000.0), 10)?,
    ];

    // --- On a Vec<HexCell> (T = Vec<HexCell>) ---
    println!("=== Vec<HexCell> ===");
    let points = cells.to_arrow_points();
    let polygons = cells.to_arrow_polygons();
    println!("  to_arrow_points():   {} point(s)", points.len());
    println!(
        "  to_arrow_polygons(): {} hexagon polygon(s)",
        polygons.len()
    );

    println!("  to_arrow_polygons(): {:?} hexagon polygon(s)", polygons);

    // The RecordBatch is the full table: id, zoom_level, row, col, easting,
    // northing, geometry.
    let batch = cells.to_record_batch()?;
    println!(
        "  to_record_batch():   {} row(s) x {} column(s)",
        batch.num_rows(),
        batch.num_columns()
    );
    // Bind the schema to a local: `batch.schema()` returns an owned `Arc<Schema>`,
    // and the collected `&str`s borrow from it, so it must outlive `columns`.
    let schema = batch.schema();
    let columns: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    println!("  columns: {}", columns.join(", "));

    // --- On a &[HexCell] slice (T = &[HexCell]) — same trait, no allocation ---
    println!("\n=== &[HexCell] slice ===");
    let slice: &[HexCell] = cells.as_slice();
    println!(
        "  to_arrow_points(): {} point(s)",
        slice.to_arrow_points().len()
    );

    // --- On a single HexCell — forwards via std::slice::from_ref internally ---
    println!("\n=== single HexCell ===");
    let one = &cells[0];
    let one_batch = one.to_record_batch()?;
    println!("  to_record_batch(): {} row(s)", one_batch.num_rows());

    // A HexGrid does NOT implement HexCellsToArrow itself. It has its own inherent
    // to_arrow_* / to_record_batch methods that forward to the inner Vec<HexCell>
    // (which is what hits the blanket impl). To call the trait directly on a grid,
    // go through `grid.cells()` to get a `&[HexCell]`.
    println!("\n=== HexGrid ===");
    let grid = HexGrid::from_bng_extent(&(530000.0, 180000.0), &(531000.0, 181000.0), 10)?;

    // Inherent method — forwards to the trait under the hood.
    let grid_batch = grid.to_record_batch()?;
    println!(
        "  grid.to_record_batch(): {} row(s) (inherent, forwards to the Vec)",
        grid_batch.num_rows()
    );

    // The trait itself, used on the grid's cell slice.
    let grid_points = grid.cells().to_arrow_points();
    println!(
        "  grid.cells().to_arrow_points(): {} point(s) (trait on &[HexCell])",
        grid_points.len()
    );

    Ok(())
}
