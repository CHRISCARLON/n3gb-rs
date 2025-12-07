use arrow_array::{Float64Array, Int64Array, StringArray, UInt8Array};
use geo_types::LineString;
use n3gb_rs::{HexCell, HexGrid, N3gbError, write_geoparquet};
use std::time::Instant;

fn main() -> Result<(), N3gbError> {
    // Test from_line_string_wgs84 with a gas pipe from Manchester
    let coords = vec![
        (-2.248423716278411, 53.4804537960769),
        (-2.248817614533952, 53.480510340167925),
        (-2.249255070278722, 53.480573578320396),
        (-2.249632113002486, 53.48061535991179),
        (-2.249727351292022, 53.48062675585576),
        (-2.249783362652013, 53.48063231040347),
        (-2.250089215187727, 53.48066240167042),
        (-2.250244759514899, 53.48066909573824),
    ];
    let line: LineString<f64> = coords.into();

    let cells = HexCell::from_line_string_wgs84(&line, 12)?;
    println!("LineString test: {} hex cells", cells.len());
    for cell in &cells {
        println!("  {} | row={} col={}", cell.id, cell.row, cell.col);
    }
    println!();

    // Test HexGrids, Arrow, and GeoParquet
    let start = Instant::now();
    let grid = HexGrid::builder()
        .zoom_level(12)
        .bng_extent(&(500000.0, 150000.0), &(550000.0, 200000.0))
        .build();
    let grid_time = start.elapsed();

    println!("Grid generated: {} cells in {:?}", grid.len(), grid_time);

    let start = Instant::now();
    let batch = grid.to_record_batch()?;
    let batch_time = start.elapsed();

    println!(
        "\nRecordBatch: {} rows, {} columns in {:?}",
        batch.num_rows(),
        batch.num_columns(),
        batch_time
    );
    println!("Schema:");
    for field in batch.schema().fields() {
        println!("  - {}: {:?}", field.name(), field.data_type());
        if !field.metadata().is_empty() {
            for (k, v) in field.metadata() {
                println!("      {}: {}", k, v);
            }
        }
    }

    let ids = batch
        .column(0)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    let zooms = batch
        .column(1)
        .as_any()
        .downcast_ref::<UInt8Array>()
        .unwrap();
    let rows = batch
        .column(2)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let cols = batch
        .column(3)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    let eastings = batch
        .column(4)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();
    let northings = batch
        .column(5)
        .as_any()
        .downcast_ref::<Float64Array>()
        .unwrap();

    println!("\nFirst 5 rows:");
    for i in 0..5.min(batch.num_rows()) {
        println!(
            "  {} | zoom={} | row={} col={} | E={:.1} N={:.1}",
            ids.value(i),
            zooms.value(i),
            rows.value(i),
            cols.value(i),
            eastings.value(i),
            northings.value(i)
        );
    }

    let start = Instant::now();
    write_geoparquet(&batch, "grid.parquet")?;
    let parquet_time = start.elapsed();

    println!("\nGeoParquet written in {:?}", parquet_time);

    Ok(())
}
