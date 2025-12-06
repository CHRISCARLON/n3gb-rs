use crate::api::hex_arrow::HexCellsToArrow;
use crate::api::hex_cell::HexCell;
use crate::util::error::N3gbError;
use arrow_array::RecordBatch;
use geoparquet::writer::{
    GeoParquetRecordBatchEncoder, GeoParquetWriterEncoding, GeoParquetWriterOptionsBuilder,
};
use parquet::arrow::ArrowWriter;
use std::fs::File;
use std::path::Path;

pub fn write_geoparquet(batch: &RecordBatch, path: impl AsRef<Path>) -> Result<(), N3gbError> {
    let schema = batch.schema();

    let options = GeoParquetWriterOptionsBuilder::default()
        .set_encoding(GeoParquetWriterEncoding::WKB)
        .build();

    let mut encoder = GeoParquetRecordBatchEncoder::try_new(&schema, &options)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    let file = File::create(path).map_err(|e| N3gbError::IoError(e.to_string()))?;
    let mut writer = ArrowWriter::try_new(file, encoder.target_schema(), None)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    let encoded_batch = encoder
        .encode_record_batch(batch)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    writer
        .write(&encoded_batch)
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    let kv_metadata = encoder
        .into_keyvalue()
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    writer.append_key_value_metadata(kv_metadata);
    writer
        .finish()
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

    Ok(())
}

pub trait HexCellsToGeoParquet: HexCellsToArrow {
    fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError>;
}

impl HexCellsToGeoParquet for [HexCell] {
    fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError> {
        let batch = self.to_record_batch()?;
        write_geoparquet(&batch, path)
    }
}

impl HexCellsToGeoParquet for Vec<HexCell> {
    fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError> {
        self.as_slice().to_geoparquet(path)
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
