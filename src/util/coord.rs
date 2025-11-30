use geo_types::Point;
use proj::Proj;
use crate::util::error::N3gbError;

pub trait Coordinate {
    fn x(&self) -> f64;
    fn y(&self) -> f64;
}

impl Coordinate for (f64, f64) {
    fn x(&self) -> f64 { self.0 }
    fn y(&self) -> f64 { self.1 }
}

impl Coordinate for Point<f64> {
    fn x(&self) -> f64 { Point::x(*self) }
    fn y(&self) -> f64 { Point::y(*self) }
}

pub fn wgs84_to_bng<C: Coordinate>(coord: &C) -> Result<Point<f64>, N3gbError> {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:27700", None)
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;

    let (easting, northing) = proj.convert((coord.x(), coord.y()))
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;
    Ok(Point::new(easting, northing))
}

pub fn bng_to_wgs84<C: Coordinate>(coord: &C) -> Result<Point<f64>, N3gbError> {
    let proj = Proj::new_known_crs("EPSG:27700", "EPSG:4326", None)
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;

    let (lon, lat) = proj.convert((coord.x(), coord.y()))
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;
    Ok(Point::new(lon, lat))
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

    #[test]
    fn test_roundtrip() -> Result<(), N3gbError> {
        let lon = -2.2479699500757597;
        let lat = 53.48082746395233;

        let bng = wgs84_to_bng(&(lon, lat))?;
        let back = bng_to_wgs84(&bng)?;

        assert!((lon - back.x()).abs() < 0.0001);
        assert!((lat - back.y()).abs() < 0.0001);
        Ok(())
    }

    #[test]
    fn test_point_conversion() -> Result<(), N3gbError> {
        let wgs84_point = Point::new(-2.2479699500757597, 53.48082746395233);
        let bng_point = wgs84_to_bng(&wgs84_point)?;
        let back = bng_to_wgs84(&bng_point)?;

        assert!((wgs84_point.x() - back.x()).abs() < 0.0001);
        assert!((wgs84_point.y() - back.y()).abs() < 0.0001);
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
