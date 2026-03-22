/// Demonstrates grid distance between HexCells.
///
/// Shows how to measure the number of hex steps between two cells,
/// and how distance grows as you move further away.
///
/// Run with:
///   cargo run --example grid_distance
use n3gb_rs::{CELL_WIDTHS, HexCell, N3gbError};

fn main() -> Result<(), N3gbError> {
    let zoom = 12;
    let width = CELL_WIDTHS[zoom as usize];

    // --- Same cell ---
    println!("=== Same cell ===");
    let a = HexCell::from_bng(&(530000.0, 180000.0), zoom)?;
    println!("  distance to self: {}", a.grid_distance(&a)?);

    // --- Adjacent cell (one step right) ---
    println!("\n=== Adjacent cell (1 step right) ===");
    let b = HexCell::from_bng(&(a.easting() + width, a.northing()), zoom)?;
    println!("  A: ({}, {})", a.row, a.col);
    println!("  B: ({}, {})", b.row, b.col);
    println!("  distance: {}", a.grid_distance(&b)?);

    // --- Moving further away ---
    println!("\n=== Distance grows with steps ===");
    println!("  {:<8} {:<8} cell id", "steps", "dist");
    println!("  {}", "-".repeat(40));

    for steps in 0..=5 {
        let x = a.easting() + width * steps as f64;
        let cell = HexCell::from_bng(&(x, a.northing()), zoom)?;
        println!("  {:<8} {:<8} {}", steps, a.grid_distance(&cell)?, cell.id);
    }

    // --- Two far-apart place ---
    println!("\n=== London to Manchester ===");
    let london = HexCell::from_bng(&(530000.0, 180000.0), zoom)?;
    let manchester = HexCell::from_bng(&(383640.0, 398260.0), zoom)?;
    println!("  London:     row={}, col={}", london.row, london.col);
    println!(
        "  Manchester: row={}, col={}",
        manchester.row, manchester.col
    );
    println!(
        "  distance:   {} hex steps",
        london.grid_distance(&manchester)?
    );

    // --- Zoom level mismatch error ---
    println!("\n=== Zoom level mismatch ===");
    let c = HexCell::from_bng(&(530000.0, 180000.0), 10)?;
    match london.grid_distance(&c) {
        Ok(d) => println!("  distance: {}", d),
        Err(e) => println!("  error (expected): {}", e),
    }

    Ok(())
}
