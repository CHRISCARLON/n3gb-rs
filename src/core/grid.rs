use crate::core::constants::{CELL_RADIUS, CELL_WIDTHS, GRID_EXTENTS, MAX_ZOOM_LEVEL};
use crate::util::coord::Coordinate;
use crate::util::error::N3gbError;
use geo_types::Point;

/// Converts a BNG coordinate to hex grid row/column indices.
///
/// Returns `(row, col)` for the cell containing the given point at the specified zoom level.
pub fn point_to_hex<C: Coordinate>(coord: &C, z: u8) -> Result<(i64, i64), N3gbError> {
    if z > MAX_ZOOM_LEVEL {
        return Err(N3gbError::InvalidZoomLevel(z));
    }

    let hex_width = CELL_WIDTHS[z as usize];
    let r = CELL_RADIUS[z as usize];
    let dx = hex_width;
    let dy = 1.5 * r;

    let qx = (coord.x() - GRID_EXTENTS[0]) / dx;
    let ry = (coord.y() - GRID_EXTENTS[1]) / dy;

    let row = ry.round() as i64;
    let col = (qx - (row % 2) as f64).round() as i64;

    Ok((row, col))
}

/// Converts hex grid row/column indices to a BNG center point.
///
/// Returns the center point of the cell at the given row, column, and zoom level.
pub fn hex_to_point(row: i64, col: i64, z: u8) -> Result<Point<f64>, N3gbError> {
    if z > MAX_ZOOM_LEVEL {
        return Err(N3gbError::InvalidZoomLevel(z));
    }

    let hex_width = CELL_WIDTHS[z as usize];
    let r = CELL_RADIUS[z as usize];
    let dx = hex_width;
    let dy = 1.5 * r;

    let x = GRID_EXTENTS[0] + col as f64 * dx + ((row % 2) as f64 * (dx / 2.0));
    let y = GRID_EXTENTS[1] + row as f64 * dy;

    Ok(Point::new(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::point;

    #[test]
    fn test_point_to_hex_and_back() -> Result<(), N3gbError> {
        let easting = 457996.0;
        let northing = 339874.0;
        let zoom = 10;

        let (row, col) = point_to_hex(&(easting, northing), zoom)?;
        let point = hex_to_point(row, col, zoom)?;

        assert!((point.x() - 457925.0).abs() < 100.0);
        assert!((point.y() - 339888.99).abs() < 100.0);
        Ok(())
    }

    #[test]
    fn test_point_to_hex_with_point() -> Result<(), N3gbError> {
        let pt = point! { x: 457996.0, y: 339874.0 };
        let zoom = 10;

        let (row, col) = point_to_hex(&pt, zoom)?;
        let center = hex_to_point(row, col, zoom)?;

        assert!((center.x() - 457925.0).abs() < 100.0);
        assert!((center.y() - 339888.99).abs() < 100.0);
        Ok(())
    }

    #[test]
    fn test_invalid_zoom_level() {
        let result = point_to_hex(&(457996.0, 339874.0), 20);
        assert!(matches!(result, Err(N3gbError::InvalidZoomLevel(20))));
    }

    #[test]
    fn test_hex_to_point_invalid_zoom() {
        let result = hex_to_point(100, 100, 16);
        assert!(result.is_err());
    }
}
