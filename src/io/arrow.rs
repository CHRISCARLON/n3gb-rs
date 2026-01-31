use crate::cell::HexCell;
use crate::error::N3gbError;
use arrow_array::{Float64Array, Int64Array, RecordBatch, StringArray, UInt8Array};
use arrow_schema::{DataType, Field, Schema};
use geoarrow_array::array::{PointArray, PolygonArray};
use geoarrow_array::builder::{PointBuilder, PolygonBuilder};
use geoarrow_array::IntoArrow;
use geoarrow_schema::{Crs, Dimension, Metadata, PointType, PolygonType};
use rayon::prelude::*;
use std::sync::Arc;

fn bng_metadata() -> Arc<Metadata> {
    let crs = Crs::from_authority_code("EPSG:27700".to_string());
    Arc::new(Metadata::new(crs, None))
}

/// Trait for converting collections of [`HexCell`]s to Arrow arrays.
///
/// Implemented for `[HexCell]` and `Vec<HexCell>`.
pub trait HexCellsToArrow {
    /// Converts cell centers to an Arrow PointArray.
    fn to_arrow_points(&self) -> PointArray;
    /// Converts cells to an Arrow PolygonArray of hexagons.
    fn to_arrow_polygons(&self) -> PolygonArray;
    /// Converts cells to a RecordBatch with id, zoom_level, row, col, easting, northing, and geometry.
    fn to_record_batch(&self) -> Result<RecordBatch, N3gbError>;
}

impl HexCellsToArrow for [HexCell] {
    fn to_arrow_points(&self) -> PointArray {
        let point = PointType::new(Dimension::XY, bng_metadata());
        let mut builder = PointBuilder::with_capacity(point, self.len());

        for cell in self {
            builder.push_point(Some(&cell.center));
        }
        builder.finish()
    }

    fn to_arrow_polygons(&self) -> PolygonArray {
        let poly = PolygonType::new(Dimension::XY, bng_metadata());
        let polygons: Vec<_> = self.par_iter().map(|c: &HexCell| c.to_polygon()).collect();
        PolygonBuilder::from_polygons(&polygons, poly).finish()
    }

    fn to_record_batch(&self) -> Result<RecordBatch, N3gbError> {
        let polygon_array = self.to_arrow_polygons();
        let ids: StringArray = self.iter().map(|c| Some(c.id.as_str())).collect();
        let zoom_levels: UInt8Array = self.iter().map(|c| Some(c.zoom_level)).collect();
        let rows: Int64Array = self.iter().map(|c| Some(c.row)).collect();
        let cols: Int64Array = self.iter().map(|c| Some(c.col)).collect();
        let eastings: Float64Array = self.iter().map(|c| Some(c.easting())).collect();
        let northings: Float64Array = self.iter().map(|c| Some(c.northing())).collect();

        let geometry_field = polygon_array.extension_type().to_field("geometry", false);
        let schema = Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("zoom_level", DataType::UInt8, false),
            Field::new("row", DataType::Int64, false),
            Field::new("col", DataType::Int64, false),
            Field::new("easting", DataType::Float64, false),
            Field::new("northing", DataType::Float64, false),
            geometry_field,
        ]);

        RecordBatch::try_new(
            Arc::new(schema),
            vec![
                Arc::new(ids),
                Arc::new(zoom_levels),
                Arc::new(rows),
                Arc::new(cols),
                Arc::new(eastings),
                Arc::new(northings),
                Arc::new(polygon_array.into_arrow()),
            ],
        )
        .map_err(|e| N3gbError::IoError(e.to_string()))
    }
}

impl HexCellsToArrow for Vec<HexCell> {
    fn to_arrow_points(&self) -> PointArray {
        self.as_slice().to_arrow_points()
    }

    fn to_arrow_polygons(&self) -> PolygonArray {
        self.as_slice().to_arrow_polygons()
    }

    fn to_record_batch(&self) -> Result<RecordBatch, N3gbError> {
        self.as_slice().to_record_batch()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geoarrow_array::GeoArrowArray;

    #[test]
    fn test_cells_to_arrow_points() -> Result<(), N3gbError> {
        let cells = vec![
            HexCell::from_bng(&(383640.0, 398260.0), 12)?,
            HexCell::from_bng(&(383700.0, 398300.0), 12)?,
        ];

        let point_array = cells.to_arrow_points();
        assert_eq!(point_array.len(), 2);
        Ok(())
    }

    #[test]
    fn test_cells_to_arrow_polygons() -> Result<(), N3gbError> {
        let cells = vec![
            HexCell::from_bng(&(383640.0, 398260.0), 12)?,
            HexCell::from_bng(&(383700.0, 398300.0), 12)?,
        ];

        let polygon_array = cells.to_arrow_polygons();
        assert_eq!(polygon_array.len(), 2);
        Ok(())
    }

    #[test]
    fn test_slice_to_arrow() -> Result<(), N3gbError> {
        let cells = vec![
            HexCell::from_bng(&(383640.0, 398260.0), 12)?,
            HexCell::from_bng(&(383700.0, 398300.0), 12)?,
            HexCell::from_bng(&(383760.0, 398340.0), 12)?,
        ];

        let point_array = cells.as_slice().to_arrow_points();
        let polygon_array = cells.as_slice().to_arrow_polygons();

        assert_eq!(point_array.len(), 3);
        assert_eq!(polygon_array.len(), 3);
        Ok(())
    }
}
