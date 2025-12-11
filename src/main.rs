use geo_types::LineString;
use n3gb_rs::{HexCell, HexCellsToArrow, N3gbError, write_geoparquet};
use std::time::Instant;

fn main() -> Result<(), N3gbError> {
    let base_coords = [
        (-2.248423716278411, 53.4804537960769),
        (-2.248817614533952, 53.480510340167925),
        (-2.249255070278722, 53.480573578320396),
        (-2.249632113002486, 53.48061535991179),
        (-2.249727351292022, 53.48062675585576),
        (-2.249783362652013, 53.48063231040347),
        (-2.250089215187727, 53.48066240167042),
        (-2.250244759514899, 53.48066909573824),
    ];

    const NUM_LINES: usize = 10_000;

    println!("Generating {} LineStrings...", NUM_LINES);
    let lines: Vec<LineString<f64>> = (0..NUM_LINES)
        .map(|i| {
            let offset_lon = (i % 100) as f64 * 0.001;
            let offset_lat = (i / 100) as f64 * 0.001;
            let coords: Vec<(f64, f64)> = base_coords
                .iter()
                .map(|(lon, lat)| (lon + offset_lon, lat + offset_lat))
                .collect();
            LineString::from(coords)
        })
        .collect();

    println!(
        "Converting {} LineStrings to hex cells (WGS84 -> BNG)...",
        NUM_LINES
    );
    let start = Instant::now();

    let mut all_cells: Vec<HexCell> = Vec::new();
    for line in &lines {
        let cells = HexCell::from_line_string_wgs84(line, 13)?;
        all_cells.extend(cells);
    }

    let conversion_time = start.elapsed();
    println!(
        "Converted {} LineStrings -> {} hex cells in {:?}",
        NUM_LINES,
        all_cells.len(),
        conversion_time
    );
    println!(
        "Average: {:.2}ms per LineString, {:.0} cells/sec",
        conversion_time.as_secs_f64() * 1000.0 / NUM_LINES as f64,
        all_cells.len() as f64 / conversion_time.as_secs_f64()
    );

    println!("\nConverting to Arrow RecordBatch...");
    let start = Instant::now();
    let batch = all_cells.to_record_batch()?;
    let arrow_time = start.elapsed();

    println!(
        "RecordBatch: {} rows, {} columns in {:?}",
        batch.num_rows(),
        batch.num_columns(),
        arrow_time
    );

    println!("\nWriting to GeoParquet...");
    let start = Instant::now();
    write_geoparquet(&batch, "lines.parquet")?;
    let parquet_time = start.elapsed();

    println!("GeoParquet written in {:?}", parquet_time);

    println!("\n=== Summary ===");
    println!("LineStrings processed: {}", NUM_LINES);
    println!("Total hex cells: {}", all_cells.len());
    println!("WGS84 conversion: {:?}", conversion_time);
    println!("Arrow conversion: {:?}", arrow_time);
    println!("Parquet write: {:?}", parquet_time);
    println!(
        "Total time: {:?}",
        conversion_time + arrow_time + parquet_time
    );

    Ok(())
}
