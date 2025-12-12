use crate::util::error::N3gbError;
use geo_types::{Coord, LineString, Point};
use proj::Proj;
use rayon::prelude::*;
use std::cell::RefCell;

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

thread_local! {
    static WGS84_TO_BNG_PROJ_OBJECT: RefCell<Option<Proj>> = const { RefCell::new(None) };
}

fn with_wgs84_to_bng_proj<T, F>(proj_closure: F) -> Result<T, N3gbError>
where
    F: FnOnce(&Proj) -> Result<T, N3gbError>,
{
    WGS84_TO_BNG_PROJ_OBJECT.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if borrow.is_none() {
            *borrow = Some(
                Proj::new_known_crs("EPSG:4326", "EPSG:27700", None)
                    .map_err(|e| N3gbError::ProjectionError(e.to_string()))?,
            );
        }
        proj_closure(borrow.as_ref().unwrap()) // Note to self to remember that this is where we deref and get the &Proj to call the closure we passed in
    })
}

/// Converts WGS84 (longitude, latitude) coordinates to British National Grid.
///
/// # Example
///
/// ```
/// use n3gb_rs::wgs84_to_bng;
///
/// # fn main() -> Result<(), n3gb_rs::N3gbError> {
/// let bng = wgs84_to_bng(&(-2.248, 53.481))?;
/// println!("Easting: {}, Northing: {}", bng.x(), bng.y());
/// # Ok(())
/// # }
/// ```
pub fn wgs84_to_bng<C: Coordinate>(coord: &C) -> Result<Point<f64>, N3gbError> {
    with_wgs84_to_bng_proj(|proj| {
        let (easting, northing) = proj
            .convert((coord.x(), coord.y()))
            .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;
        Ok(Point::new(easting, northing))
    })
}

pub fn wgs84_line_to_bng(line: &LineString) -> Result<LineString, N3gbError> {
    let coords: Result<Vec<Coord>, N3gbError> = line
        .0
        .par_iter()
        .map(|c| {
            with_wgs84_to_bng_proj(|proj| {
                let (e, n) = proj
                    .convert((c.x, c.y))
                    .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;
                Ok(Coord { x: e, y: n })
            })
        })
        .collect();
    Ok(LineString::new(coords?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgs84_to_bng() -> Result<(), N3gbError> {
        let bng = wgs84_to_bng(&(-2.2479699500757597, 53.48082746395233))?;

        assert!(bng.x() > 380000.0 && bng.x() < 390000.0);
        assert!(bng.y() > 390000.0 && bng.y() < 400000.0);
        Ok(())
    }

    // Tests for Coordinate trait generics
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

    #[test]
    fn test_same_result_tuple_and_point() -> Result<(), N3gbError> {
        let lon = -2.2479699500757597;
        let lat = 53.48082746395233;

        let from_tuple = wgs84_to_bng(&(lon, lat))?;
        let from_point = wgs84_to_bng(&Point::new(lon, lat))?;

        assert_eq!(from_tuple.x(), from_point.x());
        assert_eq!(from_tuple.y(), from_point.y());
        Ok(())
    }

    #[test]
    fn test_generic_function_accepts_both_types() -> Result<(), N3gbError> {
        fn convert_and_sum<C: Coordinate>(coord: &C) -> Result<f64, N3gbError> {
            let bng = wgs84_to_bng(coord)?;
            Ok(bng.x() + bng.y())
        }

        let tuple_result = convert_and_sum(&(-2.248, 53.481))?;
        let point_result = convert_and_sum(&Point::new(-2.248, 53.481))?;

        assert_eq!(tuple_result, point_result);
        Ok(())
    }
}
