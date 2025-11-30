use geo_types::{Point, Polygon};
use crate::core::constants::CELL_RADIUS;
use crate::core::geometry::create_hexagon;
use crate::core::grid::{hex_to_point, point_to_hex};
use crate::util::coord::{wgs84_to_bng, Coordinate};
use crate::util::error::N3gbError;
use crate::util::identifier::{generate_identifier, decode_hex_identifier};

#[derive(Debug, Clone, PartialEq)]
pub struct HexCell {
    pub id: String,
    pub center: Point<f64>,
    pub zoom_level: u8,
    pub row: i64,
    pub col: i64,
}

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

    pub fn easting(&self) -> f64 {
        self.center.x()
    }

    pub fn northing(&self) -> f64 {
        self.center.y()
    }

    pub fn to_polygon(&self) -> Polygon<f64> {
        create_hexagon(&self.center, CELL_RADIUS[self.zoom_level as usize])
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
        // Should be in Manchester area
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
