use crate::error::N3gbError;
use geo_types::Geometry;
use geojson::GeoJson;
use std::str::FromStr;
use wkt::Wkt;

/// Parses a geometry string, auto-detecting WKT or GeoJSON format.
///
/// GeoJSON is detected by a leading `{`, everything else is tried as WKT.
pub fn parse_geometry(s: &str) -> Result<Geometry<f64>, N3gbError> {
    let trimmed = s.trim();
    if trimmed.starts_with('{') {
        parse_geojson(trimmed)
    } else {
        parse_wkt(trimmed)
    }
}

/// Parses a GeoJSON string into a `geo_types::Geometry`.
pub fn parse_geojson(s: &str) -> Result<Geometry<f64>, N3gbError> {
    let geojson: GeoJson = s
        .parse()
        .map_err(|e: geojson::Error| N3gbError::GeometryParseError(e.to_string()))?;

    match geojson {
        GeoJson::Geometry(geom) => {
            Geometry::try_from(geom).map_err(|e| N3gbError::GeometryParseError(e.to_string()))
        }
        GeoJson::Feature(feat) => feat
            .geometry
            .ok_or_else(|| N3gbError::GeometryParseError("Feature has no geometry".to_string()))
            .and_then(|g| {
                Geometry::try_from(g).map_err(|e| N3gbError::GeometryParseError(e.to_string()))
            }),
        GeoJson::FeatureCollection(_) => Err(N3gbError::GeometryParseError(
            "FeatureCollection not supported, use individual geometries".to_string(),
        )),
    }
}

/// Parses a WKT string into a `geo_types::Geometry`.
pub fn parse_wkt(s: &str) -> Result<Geometry<f64>, N3gbError> {
    let wkt: Wkt<f64> =
        Wkt::from_str(s).map_err(|e| N3gbError::GeometryParseError(e.to_string()))?;

    wkt.try_into()
        .map_err(|_| N3gbError::GeometryParseError("Failed to convert WKT to geometry".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_geojson_point() -> Result<(), N3gbError> {
        let json = r#"{"type":"Point","coordinates":[-0.1,51.5]}"#;
        let geom = parse_geometry(json)?;
        match geom {
            Geometry::Point(pt) => {
                assert!((pt.x() - (-0.1)).abs() < 0.001);
                assert!((pt.y() - 51.5).abs() < 0.001);
            }
            _ => panic!("Expected Point"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_geojson_linestring() -> Result<(), N3gbError> {
        let json = r#"{"type":"LineString","coordinates":[[-0.1,51.5],[-0.2,51.6]]}"#;
        let geom = parse_geometry(json)?;
        match geom {
            Geometry::LineString(line) => {
                assert_eq!(line.0.len(), 2);
            }
            _ => panic!("Expected LineString"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_geojson_multilinestring() -> Result<(), N3gbError> {
        let json = r#"{"type":"MultiLineString","coordinates":[[[-0.1,51.5],[-0.2,51.6]],[[-0.3,51.7],[-0.4,51.8]]]}"#;
        let geom = parse_geometry(json)?;
        match geom {
            Geometry::MultiLineString(mls) => {
                assert_eq!(mls.0.len(), 2);
            }
            _ => panic!("Expected MultiLineString"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_wkt_point() -> Result<(), N3gbError> {
        let wkt = "POINT(-0.1 51.5)";
        let geom = parse_geometry(wkt)?;
        match geom {
            Geometry::Point(pt) => {
                assert!((pt.x() - (-0.1)).abs() < 0.001);
                assert!((pt.y() - 51.5).abs() < 0.001);
            }
            _ => panic!("Expected Point"),
        }
        Ok(())
    }

    #[test]
    fn test_parse_wkt_linestring() -> Result<(), N3gbError> {
        let wkt = "LINESTRING(-0.1 51.5, -0.2 51.6)";
        let geom = parse_geometry(wkt)?;
        match geom {
            Geometry::LineString(line) => {
                assert_eq!(line.0.len(), 2);
            }
            _ => panic!("Expected LineString"),
        }
        Ok(())
    }
}
