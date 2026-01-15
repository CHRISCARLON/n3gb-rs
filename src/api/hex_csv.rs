use crate::api::hex_cell::HexCell;
use crate::util::error::N3gbError;
use geo_types::Geometry;
use geojson::GeoJson;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::str::FromStr;
use wkt::Wkt;

/// Coordinate reference system for input geometry data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Crs {
    /// WGS84 (EPSG:4326) - longitude/latitude coordinates
    #[default]
    Wgs84,
    /// British National Grid (EPSG:27700) - easting/northing coordinates
    Bng,
}

/// Output format for hex polygon geometries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryFormat {
    /// Well-Known Text format (e.g., "POLYGON((...))")
    Wkt,
    /// GeoJSON format
    GeoJson,
}

/// Configuration for CSV to hex conversion.
#[derive(Debug, Clone)]
pub struct CsvHexConfig {
    pub geometry_column: String,
    pub exclude_columns: Vec<String>,
    pub zoom_level: u8,
    pub crs: Crs,
    pub include_hex_geometry: Option<GeometryFormat>,
}

impl CsvHexConfig {
    pub fn new(geometry_column: impl Into<String>, zoom_level: u8) -> Self {
        Self {
            geometry_column: geometry_column.into(),
            exclude_columns: Vec::new(),
            zoom_level,
            crs: Crs::default(),
            include_hex_geometry: None,
        }
    }

    pub fn exclude(mut self, columns: Vec<String>) -> Self {
        self.exclude_columns = columns;
        self
    }

    pub fn crs(mut self, crs: Crs) -> Self {
        self.crs = crs;
        self
    }

    /// Include hex polygon geometry in output.
    pub fn with_hex_geometry(mut self, format: GeometryFormat) -> Self {
        self.include_hex_geometry = Some(format);
        self
    }
}

pub trait CsvToHex {
    fn to_hex_csv(
        &self,
        output_path: impl AsRef<Path>,
        config: &CsvHexConfig,
    ) -> Result<(), N3gbError>;
}

impl<P: AsRef<Path>> CsvToHex for P {
    fn to_hex_csv(
        &self,
        output_path: impl AsRef<Path>,
        config: &CsvHexConfig,
    ) -> Result<(), N3gbError> {
        csv_to_hex_csv(self, output_path, config)
    }
}

// ============================================================================
// Geometry Parsing
// ============================================================================

fn parse_geometry(s: &str) -> Result<Geometry<f64>, N3gbError> {
    let trimmed = s.trim();
    if trimmed.starts_with('{') {
        parse_geojson(trimmed)
    } else {
        parse_wkt(trimmed)
    }
}

fn parse_geojson(s: &str) -> Result<Geometry<f64>, N3gbError> {
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

fn parse_wkt(s: &str) -> Result<Geometry<f64>, N3gbError> {
    let wkt: Wkt<f64> =
        Wkt::from_str(s).map_err(|e| N3gbError::GeometryParseError(e.to_string()))?;

    wkt.try_into()
        .map_err(|_| N3gbError::GeometryParseError("Failed to convert WKT to geometry".to_string()))
}

// ============================================================================
// Geometry Output Conversion
// ============================================================================

fn polygon_to_wkt(polygon: &geo_types::Polygon<f64>) -> String {
    use wkt::ToWkt;
    polygon.wkt_string()
}

fn polygon_to_geojson(polygon: &geo_types::Polygon<f64>) -> String {
    let geom = geojson::Geometry::from(polygon);
    geom.to_string()
}

// ============================================================================
// Geometry to Hex Cells Conversion
// ============================================================================

fn geometry_to_hex_cells(
    geom: Geometry<f64>,
    zoom: u8,
    crs: Crs,
) -> Result<Vec<HexCell>, N3gbError> {
    use geo::Centroid;

    match geom {
        Geometry::Point(pt) => {
            let cell = match crs {
                Crs::Wgs84 => HexCell::from_wgs84(&pt, zoom)?,
                Crs::Bng => HexCell::from_bng(&pt, zoom)?,
            };
            Ok(vec![cell])
        }
        Geometry::LineString(line) => match crs {
            Crs::Wgs84 => HexCell::from_line_string_wgs84(&line, zoom),
            Crs::Bng => HexCell::from_line_string_bng(&line, zoom),
        },
        Geometry::MultiLineString(mls) => {
            let mut all_cells = Vec::new();
            for line in mls.0 {
                let cells = match crs {
                    Crs::Wgs84 => HexCell::from_line_string_wgs84(&line, zoom)?,
                    Crs::Bng => HexCell::from_line_string_bng(&line, zoom)?,
                };
                all_cells.extend(cells);
            }
            Ok(all_cells)
        }
        Geometry::Polygon(poly) => {
            if let Some(centroid) = poly.centroid() {
                let cell = match crs {
                    Crs::Wgs84 => HexCell::from_wgs84(&centroid, zoom)?,
                    Crs::Bng => HexCell::from_bng(&centroid, zoom)?,
                };
                Ok(vec![cell])
            } else {
                Ok(vec![])
            }
        }
        Geometry::MultiPolygon(mp) => {
            let mut cells = Vec::new();
            for poly in mp.0 {
                if let Some(centroid) = poly.centroid() {
                    let cell = match crs {
                        Crs::Wgs84 => HexCell::from_wgs84(&centroid, zoom)?,
                        Crs::Bng => HexCell::from_bng(&centroid, zoom)?,
                    };
                    cells.push(cell);
                }
            }
            Ok(cells)
        }
        Geometry::MultiPoint(mp) => {
            let mut cells = Vec::new();
            for pt in mp.0 {
                let cell = match crs {
                    Crs::Wgs84 => HexCell::from_wgs84(&pt, zoom)?,
                    Crs::Bng => HexCell::from_bng(&pt, zoom)?,
                };
                cells.push(cell);
            }
            Ok(cells)
        }
        Geometry::GeometryCollection(gc) => {
            let mut all_cells = Vec::new();
            for g in gc.0 {
                all_cells.extend(geometry_to_hex_cells(g, zoom, crs)?);
            }
            Ok(all_cells)
        }
        _ => Err(N3gbError::GeometryParseError(
            "Unsupported geometry type".to_string(),
        )),
    }
}

// ============================================================================
// CSV Conversion
// ============================================================================

/// Converts a CSV file with geometry column to a CSV file with hex IDs.
///
/// Streams output to minimize memory usage for large files.
///
/// # Example (can also use trait method)
///
/// ```no_run
/// use n3gb_rs::{csv_to_hex_csv, CsvHexConfig, Crs};
///
/// let config = CsvHexConfig::new("Geo Shape", 12)
///     .exclude(vec!["Geo Point".into()])
///     .crs(Crs::Wgs84);
///
/// csv_to_hex_csv("input.csv", "output.csv", &config).unwrap();
/// ```
pub fn csv_to_hex_csv(
    csv_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    config: &CsvHexConfig,
) -> Result<(), N3gbError> {
    let file = File::open(csv_path).map_err(|e| N3gbError::CsvError(e.to_string()))?;
    let mut reader = csv::Reader::from_reader(file);

    let headers = reader
        .headers()
        .map_err(|e| N3gbError::CsvError(e.to_string()))?
        .clone();

    let geom_idx = headers
        .iter()
        .position(|h| h == config.geometry_column)
        .ok_or_else(|| {
            N3gbError::CsvError(format!(
                "Geometry column '{}' not found",
                config.geometry_column
            ))
        })?;

    let mut exclude_indices: HashSet<usize> = HashSet::new();
    exclude_indices.insert(geom_idx);
    for col_name in &config.exclude_columns {
        if let Some(idx) = headers.iter().position(|h| h == col_name) {
            exclude_indices.insert(idx);
        }
    }

    let out_file = File::create(output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
    let mut writer = csv::Writer::from_writer(out_file);

    let mut header_row: Vec<&str> = vec!["hex_id"];
    if config.include_hex_geometry.is_some() {
        header_row.push("hex_geometry");
    }
    for (i, h) in headers.iter().enumerate() {
        if !exclude_indices.contains(&i) {
            header_row.push(h);
        }
    }
    writer
        .write_record(&header_row)
        .map_err(|e| N3gbError::CsvError(e.to_string()))?;

    for result in reader.records() {
        let record = result.map_err(|e| N3gbError::CsvError(e.to_string()))?;

        let geom_str = record.get(geom_idx).ok_or_else(|| {
            N3gbError::CsvError(format!("Missing geometry column at index {}", geom_idx))
        })?;

        let geom = parse_geometry(geom_str)?;
        let cells = geometry_to_hex_cells(geom, config.zoom_level, config.crs)?;

        for cell in cells {
            let mut row: Vec<String> = vec![cell.id.clone()];

            if let Some(format) = config.include_hex_geometry {
                let polygon = cell.to_polygon();
                let geom_str = match format {
                    GeometryFormat::Wkt => polygon_to_wkt(&polygon),
                    GeometryFormat::GeoJson => polygon_to_geojson(&polygon),
                };
                row.push(geom_str);
            }

            for (i, field) in record.iter().enumerate() {
                if !exclude_indices.contains(&i) {
                    row.push(field.to_string());
                }
            }
            writer
                .write_record(&row)
                .map_err(|e| N3gbError::CsvError(e.to_string()))?;
        }
    }

    writer
        .flush()
        .map_err(|e| N3gbError::CsvError(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

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

    #[test]
    fn test_csv_to_hex_csv_wgs84() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ASSET_ID,TYPE,geometry").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(
            file,
            "CDT123,Pipe,\"{{\"\"type\"\":\"\"Point\"\",\"\"coordinates\"\":[-0.1,51.5]}}\""
        )
        .map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::new("geometry", 12).crs(Crs::Wgs84);
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    fn test_csv_to_hex_csv_bng() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        // BNG coordinates for somewhere in London
        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ASSET_ID,TYPE,geometry").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "CDT123,Pipe,\"POINT(530000 180000)\"")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::new("geometry", 12).crs(Crs::Bng);
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    fn test_crs_enum_default() {
        assert_eq!(Crs::default(), Crs::Wgs84);
    }
}
