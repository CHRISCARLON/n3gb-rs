use std::time::Instant;
use arrow_array::{StringArray, UInt8Array, Int64Array, Float64Array};
use n3gb_rs::{HexGrid, N3gbError, write_geoparquet};

fn main() -> Result<(), N3gbError> {
    let start = Instant::now();
    let grid = HexGrid::builder()
        .zoom_level(12)
        .bng_extent(&(530000.0, 180000.0), &(535000.0, 185000.0))
        .build();
    let grid_time = start.elapsed();

    println!("Grid generated: {} cells in {:?}", grid.len(), grid_time);

    let start = Instant::now();
    let batch = grid.to_record_batch()?;
    let batch_time = start.elapsed();

    println!("\nRecordBatch: {} rows, {} columns in {:?}", batch.num_rows(), batch.num_columns(), batch_time);
    println!("Schema:");
    for field in batch.schema().fields() {
        println!("  - {}: {:?}", field.name(), field.data_type());
        if !field.metadata().is_empty() {
            for (k, v) in field.metadata() {
                println!("      {}: {}", k, v);
            }
        }
    }

    // Print first 5 rows from the arrow batch
    let ids = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
    let zooms = batch.column(1).as_any().downcast_ref::<UInt8Array>().unwrap();
    let rows = batch.column(2).as_any().downcast_ref::<Int64Array>().unwrap();
    let cols = batch.column(3).as_any().downcast_ref::<Int64Array>().unwrap();
    let eastings = batch.column(4).as_any().downcast_ref::<Float64Array>().unwrap();
    let northings = batch.column(5).as_any().downcast_ref::<Float64Array>().unwrap();

    println!("\nFirst 5 rows:");
    for i in 0..5.min(batch.num_rows()) {
        println!("  {} | zoom={} | row={} col={} | E={:.1} N={:.1}",
            ids.value(i), zooms.value(i), rows.value(i), cols.value(i),
            eastings.value(i), northings.value(i));
    }

    let start = Instant::now();
    write_geoparquet(&batch, "grid.parquet")?;
    let parquet_time = start.elapsed();

    println!("\nGeoParquet written in {:?}", parquet_time);

    Ok(())
}
