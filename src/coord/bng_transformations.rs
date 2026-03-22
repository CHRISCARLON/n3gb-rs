use crate::coord::ConversionMethod;
use crate::error::N3gbError;
use geo_types::{Coord, LineString, MultiPolygon, Point, Polygon};
use proj::Proj;
use rayon::prelude::*;
use std::cell::RefCell;

/// Dispatch helper functions
/// Select conversion backend at runtime based on [`ConversionMethod`].
pub(crate) fn convert_to_bng<C: super::Coordinate>(
    coord: &C,
    method: ConversionMethod,
) -> Result<Point<f64>, N3gbError> {
    match method {
        ConversionMethod::Proj => wgs84_to_bng(coord),
        ConversionMethod::Ostn15 => wgs84_to_bng_ostn15(coord),
    }
}

pub(crate) fn convert_line_to_bng(
    line: &LineString,
    method: ConversionMethod,
) -> Result<LineString, N3gbError> {
    match method {
        ConversionMethod::Proj => wgs84_line_to_bng(line),
        ConversionMethod::Ostn15 => wgs84_line_to_bng_ostn15(line),
    }
}

pub(crate) fn convert_polygon_to_bng(
    polygon: &Polygon<f64>,
    method: ConversionMethod,
) -> Result<Polygon<f64>, N3gbError> {
    match method {
        ConversionMethod::Proj => wgs84_polygon_to_bng(polygon),
        ConversionMethod::Ostn15 => wgs84_polygon_to_bng_ostn15(polygon),
    }
}

pub(crate) fn convert_multipolygon_to_bng(
    multipolygon: &MultiPolygon<f64>,
    method: ConversionMethod,
) -> Result<MultiPolygon<f64>, N3gbError> {
    match method {
        ConversionMethod::Proj => wgs84_multipolygon_to_bng(multipolygon),
        ConversionMethod::Ostn15 => wgs84_multipolygon_to_bng_ostn15(multipolygon),
    }
}

// Hacky work around for now!
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
        proj_closure(borrow.as_ref().unwrap())
    })
}

/// Converts WGS84 (longitude, latitude) coordinates to British National Grid using PROJ.
///
/// Requires the `libproj` system library. When the OSTN15 grid file
/// (`uk_os_OSTN15_NTv2_OSGBtoETRS.tif`) is installed, accuracy is ~1mm.
/// Without grid files, PROJ silently falls back to a Helmert transform (~5m accuracy).
///
/// If you need guaranteed OSTN15 accuracy without system dependencies, use
/// [`wgs84_to_bng_ostn15`] instead.
///
/// To verify which pipeline PROJ is using at runtime, set `PROJ_DEBUG=2`:
/// ```text
/// PROJ_DEBUG=2 cargo run
/// ```
/// Look for `uk_os_OSTN15_NTv2_OSGBtoETRS.tif - succeeded` (grid active)
/// or `OSGB 1936 to WGS 84 (6)` (Helmert fallback).
///
pub(crate) fn wgs84_to_bng<C: super::Coordinate>(coord: &C) -> Result<Point<f64>, N3gbError> {
    with_wgs84_to_bng_proj(|proj| {
        let (easting, northing) = proj
            .convert((coord.x(), coord.y()))
            .map_err(|e| N3gbError::ProjectionError(e.to_string()))?;
        Ok(Point::new(easting, northing))
    })
}

pub(crate) fn wgs84_line_to_bng(line: &LineString) -> Result<LineString, N3gbError> {
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

pub(crate) fn wgs84_polygon_to_bng(polygon: &Polygon<f64>) -> Result<Polygon<f64>, N3gbError> {
    let exterior = wgs84_line_to_bng(polygon.exterior())?;
    let interiors: Result<Vec<LineString>, N3gbError> =
        polygon.interiors().iter().map(wgs84_line_to_bng).collect();
    Ok(Polygon::new(exterior, interiors?))
}

pub(crate) fn wgs84_multipolygon_to_bng(
    multipolygon: &MultiPolygon<f64>,
) -> Result<MultiPolygon<f64>, N3gbError> {
    let polygons: Result<Vec<Polygon<f64>>, N3gbError> =
        multipolygon.0.iter().map(wgs84_polygon_to_bng).collect();
    Ok(MultiPolygon::new(polygons?))
}

/// Converts WGS84 (longitude, latitude) coordinates to British National Grid using OSTN15.
///
/// Uses the `lonlat_bng` crate with embedded OSTN15 grid shift data.
/// No system PROJ library required. Suitable for surveying-grade accuracy.
///
pub(crate) fn wgs84_to_bng_ostn15<C: super::Coordinate>(
    coord: &C,
) -> Result<Point<f64>, N3gbError> {
    lonlat_bng::convert_osgb36(coord.x(), coord.y())
        .map(|(e, n)| Point::new(e, n))
        .map_err(|_| N3gbError::ProjectionError("OSTN15 conversion failed".into()))
}

pub(crate) fn wgs84_line_to_bng_ostn15(line: &LineString) -> Result<LineString, N3gbError> {
    let coords: Result<Vec<Coord>, N3gbError> = line
        .0
        .par_iter()
        .map(|c| {
            lonlat_bng::convert_osgb36(c.x, c.y)
                .map(|(e, n)| Coord { x: e, y: n })
                .map_err(|_| N3gbError::ProjectionError("OSTN15 conversion failed".into()))
        })
        .collect();
    Ok(LineString::new(coords?))
}

pub(crate) fn wgs84_polygon_to_bng_ostn15(
    polygon: &Polygon<f64>,
) -> Result<Polygon<f64>, N3gbError> {
    let exterior = wgs84_line_to_bng_ostn15(polygon.exterior())?;
    let interiors: Result<Vec<LineString>, N3gbError> = polygon
        .interiors()
        .iter()
        .map(wgs84_line_to_bng_ostn15)
        .collect();
    Ok(Polygon::new(exterior, interiors?))
}

pub(crate) fn wgs84_multipolygon_to_bng_ostn15(
    multipolygon: &MultiPolygon<f64>,
) -> Result<MultiPolygon<f64>, N3gbError> {
    let polygons: Result<Vec<Polygon<f64>>, N3gbError> = multipolygon
        .0
        .iter()
        .map(wgs84_polygon_to_bng_ostn15)
        .collect();
    Ok(MultiPolygon::new(polygons?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coord::Coordinate;

    #[test]
    fn test_wgs84_to_bng() -> Result<(), N3gbError> {
        let bng = wgs84_to_bng(&(-2.2479699500757597, 53.48082746395233))?;

        assert!(bng.x() > 380000.0 && bng.x() < 390000.0);
        assert!(bng.y() > 390000.0 && bng.y() < 400000.0);
        Ok(())
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

    #[test]
    fn test_wgs84_to_bng_ostn15() -> Result<(), N3gbError> {
        let bng = wgs84_to_bng_ostn15(&(-2.2479699500757597, 53.48082746395233))?;

        assert!(bng.x() > 380000.0 && bng.x() < 390000.0);
        assert!(bng.y() > 390000.0 && bng.y() < 400000.0);
        Ok(())
    }

    #[test]
    fn test_ostn15_same_result_tuple_and_point() -> Result<(), N3gbError> {
        let lon = -2.2479699500757597;
        let lat = 53.48082746395233;

        let from_tuple = wgs84_to_bng_ostn15(&(lon, lat))?;
        let from_point = wgs84_to_bng_ostn15(&Point::new(lon, lat))?;

        assert_eq!(from_tuple.x(), from_point.x());
        assert_eq!(from_tuple.y(), from_point.y());
        Ok(())
    }

    #[test]
    fn test_proj_and_ostn15_close_agreement() -> Result<(), N3gbError> {
        let coord = (-2.2479699500757597, 53.48082746395233);
        let proj_result = wgs84_to_bng(&coord)?;
        let ostn15_result = wgs84_to_bng_ostn15(&coord)?;

        // Both paths should agree within 10 metres
        // (PROJ falls back to a Helmert transform in CI where grid files are absent)
        assert!((proj_result.x() - ostn15_result.x()).abs() < 10.0);
        assert!((proj_result.y() - ostn15_result.y()).abs() < 10.0);
        Ok(())
    }

    #[test]
    fn test_wgs84_line_to_bng_ostn15() -> Result<(), N3gbError> {
        use geo_types::LineString;
        let line: LineString = vec![(-2.248, 53.481), (-1.5, 53.8)].into();
        let bng_line = wgs84_line_to_bng_ostn15(&line)?;

        assert_eq!(bng_line.0.len(), 2);
        assert!(bng_line.0[0].x > 300000.0);
        assert!(bng_line.0[0].y > 300000.0);
        Ok(())
    }

    #[test]
    fn test_wgs84_polygon_to_bng_ostn15() -> Result<(), N3gbError> {
        use geo_types::{LineString, Polygon};
        let exterior: LineString = vec![
            (-2.248, 53.481),
            (-2.240, 53.481),
            (-2.240, 53.488),
            (-2.248, 53.488),
            (-2.248, 53.481),
        ]
        .into();
        let polygon = Polygon::new(exterior, vec![]);
        let bng_polygon = wgs84_polygon_to_bng_ostn15(&polygon)?;

        assert_eq!(bng_polygon.exterior().0.len(), 5);
        Ok(())
    }
}
