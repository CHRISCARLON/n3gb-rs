use geo_types::Point;
use proj::Proj;
use crate::util::error::N3gbError;

pub fn wgs84_to_bng(lon: f64, lat: f64) -> Result<(f64, f64), N3gbError> {
    let proj = Proj::new_known_crs("EPSG:4326", "EPSG:27700", None)
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;

    proj.convert((lon, lat))
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))
}

pub fn wgs84_to_bng_point(point: &Point<f64>) -> Result<Point<f64>, N3gbError> {
    let (easting, northing) = wgs84_to_bng(point.x(), point.y())?;
    Ok(Point::new(easting, northing))
}

pub fn bng_to_wgs84(easting: f64, northing: f64) -> Result<(f64, f64), N3gbError> {
    let proj = Proj::new_known_crs("EPSG:27700", "EPSG:4326", None)
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;

    proj.convert((easting, northing))
        .map_err(|e| N3gbError::ProjectionError(e.to_string()))
}

pub fn bng_to_wgs84_point(point: &Point<f64>) -> Result<Point<f64>, N3gbError> {
    let (lon, lat) = bng_to_wgs84(point.x(), point.y())?;
    Ok(Point::new(lon, lat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgs84_to_bng() -> Result<(), N3gbError> {
        let (easting, northing) = wgs84_to_bng(-2.2479699500757597, 53.48082746395233)?;

        assert!(easting > 380000.0 && easting < 390000.0);
        assert!(northing > 390000.0 && northing < 400000.0);
        Ok(())
    }

    #[test]
    fn test_roundtrip() -> Result<(), N3gbError> {
        let lon = -2.2479699500757597;
        let lat = 53.48082746395233;

        let (easting, northing) = wgs84_to_bng(lon, lat)?;
        let (lon2, lat2) = bng_to_wgs84(easting, northing)?;

        assert!((lon - lon2).abs() < 0.0001);
        assert!((lat - lat2).abs() < 0.0001);
        Ok(())
    }

    #[test]
    fn test_point_conversion() -> Result<(), N3gbError> {
        let wgs84_point = Point::new(-2.2479699500757597, 53.48082746395233);
        let bng_point = wgs84_to_bng_point(&wgs84_point)?;
        let back = bng_to_wgs84_point(&bng_point)?;

        assert!((wgs84_point.x() - back.x()).abs() < 0.0001);
        assert!((wgs84_point.y() - back.y()).abs() < 0.0001);
        Ok(())
    }
}
