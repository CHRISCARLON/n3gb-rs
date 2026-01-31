mod bng_transformations;

pub use bng_transformations::{
    wgs84_line_to_bng, wgs84_multipolygon_to_bng, wgs84_polygon_to_bng, wgs84_to_bng,
};

use geo_types::Point;

/// Trait for types that can provide x/y coordinates.
///
/// Implemented for `(f64, f64)` tuples and `geo_types::Point<f64>`.
/// This allows functions to accept either type.
pub trait Coordinate {
    /// Returns the x-coordinate (easting or longitude).
    fn x(&self) -> f64;
    /// Returns the y-coordinate (northing or latitude).
    fn y(&self) -> f64;
}

impl Coordinate for (f64, f64) {
    fn x(&self) -> f64 {
        self.0
    }
    fn y(&self) -> f64 {
        self.1
    }
}

impl Coordinate for Point<f64> {
    fn x(&self) -> f64 {
        Point::x(*self)
    }
    fn y(&self) -> f64 {
        Point::y(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_trait_tuple() {
        let tuple = (100.0, 200.0);
        assert_eq!(tuple.x(), 100.0);
        assert_eq!(tuple.y(), 200.0);
    }

    #[test]
    fn test_coordinate_trait_point() {
        let point = Point::new(100.0, 200.0);
        assert_eq!(point.x(), 100.0);
        assert_eq!(point.y(), 200.0);
    }
}
