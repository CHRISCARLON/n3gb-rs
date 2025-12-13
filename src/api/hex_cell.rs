use crate::api::hex_arrow::HexCellsToArrow;
use crate::api::hex_parquet::HexCellsToGeoParquet;
use crate::core::constants::CELL_RADIUS;
use crate::core::geometry::create_hexagon;
use crate::core::grid::{hex_to_point, point_to_hex};
use crate::util::coord::{Coordinate, wgs84_line_to_bng, wgs84_to_bng};
use crate::util::error::N3gbError;
use crate::util::identifier::{decode_hex_identifier, generate_identifier};
use arrow_array::RecordBatch;
use geo_types::{LineString, Point, Polygon};
use geoarrow_array::array::{PointArray, PolygonArray};
use std::collections::HashSet;
use std::path::Path;

/// A single hexagonal cell in the n3gb spatial indexing system.
///
/// Each `HexCell` represents one hexagon in the grid, with a unique identifier,
/// center point in British National Grid (BNG) coordinates, and grid position.
///
/// # Example
///
/// ```
/// use n3gb_rs::HexCell;
///
/// # fn main() -> Result<(), n3gb_rs::N3gbError> {
/// // Create from BNG coordinates
/// let cell = HexCell::from_bng(&(383640.0, 398260.0), 12)?;
/// println!("Cell ID: {}", cell.id);
/// println!("Center: ({}, {})", cell.easting(), cell.northing());
///
/// // Convert to polygon for GIS operations
/// let polygon = cell.to_polygon();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HexCell {
    /// Unique encoded identifier for this cell (Base64 URL-safe)
    pub id: String,
    /// Center point in British National Grid coordinates (EPSG:27700)
    pub center: Point<f64>,
    /// Zoom level (0-15), where higher values mean smaller cells
    pub zoom_level: u8,
    /// Row index in the hexagonal grid
    pub row: i64,
    /// Column index in the hexagonal grid
    pub col: i64,
}

// TODO: Maybe I could merge all the diff ways to create a HexCell into a
impl HexCell {
    pub(crate) fn new(id: String, center: Point<f64>, zoom_level: u8, row: i64, col: i64) -> Self {
        Self {
            id,
            center,
            zoom_level,
            row,
            col,
        }
    }

    /// Create a HexCell from an encoded hex identifier
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexCell;
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// let cell = HexCell::from_bng(&(383640.0, 398260.0), 12)?;
    /// let restored = HexCell::from_hex_id(&cell.id)?;
    /// assert_eq!(cell.id, restored.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_hex_id(id: &str) -> Result<Self, N3gbError> {
        let (_, easting, northing, zoom) = decode_hex_identifier(id)?;
        let (row, col) = point_to_hex(&(easting, northing), zoom)?;

        Ok(Self {
            id: id.to_string(),
            center: Point::new(easting, northing),
            zoom_level: zoom,
            row,
            col,
        })
    }

    /// Create HexCells along a LineString in BNG coordinates.
    ///
    /// Samples points along the line and returns all unique cells that intersect it.
    pub fn from_line_string_bng(line: &LineString, zoom: u8) -> Result<Vec<Self>, N3gbError> {
        let cell_radius = CELL_RADIUS[zoom as usize];
        let step_size = cell_radius * 0.5;

        let total_length: f64 = line
            .0
            .windows(2)
            .map(|w| {
                let dx = w[1].x - w[0].x;
                let dy = w[1].y - w[0].y;
                (dx * dx + dy * dy).sqrt()
            })
            .sum();
        let estimated_cells = ((total_length / cell_radius) * 1.5) as usize + line.0.len();

        let mut seen: HashSet<(i64, i64)> = HashSet::with_capacity(estimated_cells);
        let mut cells: Vec<HexCell> = Vec::with_capacity(estimated_cells);

        for window in line.0.windows(2) {
            let start = &window[0];
            let end = &window[1];

            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let segment_length = (dx * dx + dy * dy).sqrt();
            let steps = (segment_length / step_size).ceil() as usize;

            for i in 0..=steps {
                let t = if steps == 0 {
                    0.0
                } else {
                    i as f64 / steps as f64
                };
                let x = start.x + t * dx;
                let y = start.y + t * dy;

                let (row, col) = point_to_hex(&(x, y), zoom)?;

                if seen.insert((row, col)) {
                    let center = hex_to_point(row, col, zoom)?;
                    let id = generate_identifier(center.x(), center.y(), zoom);
                    cells.push(HexCell::new(id, center, zoom, row, col));
                }
            }
        }

        Ok(cells)
    }

    /// Create HexCells along a LineString in WGS84 coordinates.
    ///
    /// Converts the line to BNG and returns all unique cells that intersect it.
    pub fn from_line_string_wgs84(line: &LineString, zoom: u8) -> Result<Vec<Self>, N3gbError> {
        let bng_line = wgs84_line_to_bng(line)?;
        Self::from_line_string_bng(&bng_line, zoom)
    }

    /// Create a HexCell from British National Grid coordinates
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexCell;
    /// use geo_types::Point;
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// // From tuple
    /// let cell = HexCell::from_bng(&(383640.0, 398260.0), 12)?;
    /// // From Point
    /// let cell = HexCell::from_bng(&Point::new(383640.0, 398260.0), 12)?;
    /// println!("Cell ID: {}", cell.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bng(coord: &impl Coordinate, zoom: u8) -> Result<Self, N3gbError> {
        let (row, col) = point_to_hex(coord, zoom)?;
        let center = hex_to_point(row, col, zoom)?;
        let id = generate_identifier(center.x(), center.y(), zoom);

        Ok(Self {
            id,
            center,
            zoom_level: zoom,
            row,
            col,
        })
    }

    /// Create a HexCell from WGS84 (lon/lat) coordinates
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::HexCell;
    /// use geo_types::Point;
    ///
    /// # fn main() -> Result<(), n3gb_rs::N3gbError> {
    /// // From tuple
    /// let cell = HexCell::from_wgs84(&(-2.248, 53.481), 12)?;
    /// // From Point
    /// let cell = HexCell::from_wgs84(&Point::new(-2.248, 53.481), 12)?;
    /// println!("Cell ID: {}", cell.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_wgs84(coord: &impl Coordinate, zoom: u8) -> Result<Self, N3gbError> {
        let bng = wgs84_to_bng(coord)?;
        Self::from_bng(&bng, zoom)
    }

    /// Returns the easting (x-coordinate) of the cell center in meters.
    pub fn easting(&self) -> f64 {
        self.center.x()
    }

    /// Returns the northing (y-coordinate) of the cell center in meters.
    pub fn northing(&self) -> f64 {
        self.center.y()
    }

    /// Converts this cell to a hexagonal polygon.
    ///
    /// Returns a `geo_types::Polygon` representing the hexagon boundary,
    /// suitable for spatial operations or GeoJSON export.
    pub fn to_polygon(&self) -> Polygon<f64> {
        create_hexagon(&self.center, CELL_RADIUS[self.zoom_level as usize])
    }

    /// Converts this cell's center to an Arrow PointArray.
    pub fn to_arrow_points(&self) -> PointArray {
        std::slice::from_ref(self).to_arrow_points()
    }

    /// Converts this cell to an Arrow PolygonArray.
    pub fn to_arrow_polygons(&self) -> PolygonArray {
        std::slice::from_ref(self).to_arrow_polygons()
    }

    /// Converts this cell to an Arrow RecordBatch with all attributes.
    pub fn to_record_batch(&self) -> Result<RecordBatch, N3gbError> {
        std::slice::from_ref(self).to_record_batch()
    }

    /// Writes this cell to a GeoParquet file.
    pub fn to_geoparquet(&self, path: impl AsRef<Path>) -> Result<(), N3gbError> {
        std::slice::from_ref(self).to_geoparquet(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bng_tuple() -> Result<(), N3gbError> {
        let cell = HexCell::from_bng(&(383640.0, 398260.0), 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        assert!(cell.row > 0);
        assert!(cell.col > 0);
        Ok(())
    }

    #[test]
    fn test_from_bng_point() -> Result<(), N3gbError> {
        let point = Point::new(383640.0, 398260.0);
        let cell = HexCell::from_bng(&point, 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        assert!(cell.row > 0);
        assert!(cell.col > 0);
        Ok(())
    }

    #[test]
    fn test_from_wgs84_tuple() -> Result<(), N3gbError> {
        let cell = HexCell::from_wgs84(&(-2.248, 53.481), 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        // Should be Manchester area
        assert!(cell.easting() > 380000.0 && cell.easting() < 390000.0);
        assert!(cell.northing() > 390000.0 && cell.northing() < 400000.0);
        Ok(())
    }

    #[test]
    fn test_from_wgs84_point() -> Result<(), N3gbError> {
        let point = Point::new(-2.248, 53.481);
        let cell = HexCell::from_wgs84(&point, 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        assert!(cell.easting() > 380000.0 && cell.easting() < 390000.0);
        assert!(cell.northing() > 390000.0 && cell.northing() < 400000.0);
        Ok(())
    }

    #[test]
    fn test_same_point_same_cell() -> Result<(), N3gbError> {
        // The same point should always return the same cell
        let cell1 = HexCell::from_bng(&(383640.0, 398260.0), 10)?;
        let cell2 = HexCell::from_bng(&(383640.0, 398260.0), 10)?;

        assert_eq!(cell1.id, cell2.id);
        assert_eq!(cell1.row, cell2.row);
        assert_eq!(cell1.col, cell2.col);

        // A point very close to center should be in the same cell
        let cell3 = HexCell::from_bng(&(cell1.easting() + 1.0, cell1.northing() + 1.0), 10)?;
        assert_eq!(cell1.id, cell3.id);
        Ok(())
    }

    #[test]
    fn test_tuple_and_point_same_result() -> Result<(), N3gbError> {
        let from_tuple = HexCell::from_bng(&(383640.0, 398260.0), 12)?;
        let from_point = HexCell::from_bng(&Point::new(383640.0, 398260.0), 12)?;

        assert_eq!(from_tuple.id, from_point.id);
        assert_eq!(from_tuple.row, from_point.row);
        assert_eq!(from_tuple.col, from_point.col);
        Ok(())
    }
}
