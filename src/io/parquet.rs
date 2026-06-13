use crate::cell::HexCell;
use crate::error::N3gbError;
use crate::io::arrow::HexCellsToArrow;
use arrow_array::RecordBatch;
use geoparquet::writer::{
    GeoParquetRecordBatchEncoder, GeoParquetWriterEncoding, GeoParquetWriterOptionsBuilder,
};
use parquet::arrow::ArrowWriter;
use std::fs::File;
use std::path::Path;

/// Writes an Arrow RecordBatch to a GeoParquet file.
///
/// The batch should contain a geometry column (normally from [`HexCellsToArrow::to_record_batch`]).
///
/// # Arguments
///
/// * `batch` - The Arrow [`RecordBatch`] to encode, containing a geometry column.
/// * `path` - Filesystem path where the GeoParquet file is written.
///
/// # Returns
///
/// `()` on success, after the GeoParquet file has been fully written and finalized.
///
/// # Errors
///
/// Returns [`N3gbError::IoError`] if the GeoParquet encoder cannot be created, if the
/// batch cannot be encoded, if the key-value metadata cannot be produced, or if the
/// underlying file cannot be created or written (via `From<io::Error>` / `From<ParquetError>`).
pub fn write_geoparquet(batch: &RecordBatch, path: impl AsRef<Path>) -> Result<(), N3gbError> {
    let schema = batch.schema();

    let options = GeoParquetWriterOptionsBuilder::default()
        .set_encoding(GeoParquetWriterEncoding::WKB)
        .build();

    let mut encoder = GeoParquetRecordBatchEncoder::try_new(&schema, &options)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, encoder.target_schema(), None)?;

    let encoded_batch = encoder
        .encode_record_batch(batch)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    writer.write(&encoded_batch)?;

    let kv_metadata = encoder
        .into_keyvalue()
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    writer.append_key_value_metadata(kv_metadata);
    writer.finish()?;

    Ok(())
}

/// Trait for writing collections of [`HexCell`]s directly to GeoParquet.
///
/// Implemented for any type that dereferences to `[HexCell]` (e.g. `Vec<HexCell>`, `&[HexCell]`).
pub trait HexCellsToGeoParquet: HexCellsToArrow {
    /// Writes cells to a GeoParquet file at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Filesystem path where the GeoParquet file is written.
    ///
    /// # Returns
    ///
    /// `()` on success, after the cells have been written to the GeoParquet file.
    ///
    /// # Errors
    ///
    /// Returns [`N3gbError::IoError`] if the record batch cannot be built or if the
    /// GeoParquet file cannot be encoded or written (via `From<ArrowError>` /
    /// `From<ParquetError>` / `From<io::Error>`).
    fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError>;
}

impl<T: AsRef<[HexCell]>> HexCellsToGeoParquet for T {
    fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError> {
        let batch = self.to_record_batch()?;
        write_geoparquet(&batch, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cells_to_geoparquet() -> Result<(), N3gbError> {
        let cells = vec![
            HexCell::from_bng(&(383640.0, 398260.0), 12)?,
            HexCell::from_bng(&(383700.0, 398300.0), 12)?,
        ];

        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let path = dir.path().join("test.parquet");

        cells.to_geoparquet(&path)?;

        assert!(path.exists());
        let metadata = std::fs::metadata(&path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        assert!(metadata.len() > 0);
        Ok(())
    }
}
