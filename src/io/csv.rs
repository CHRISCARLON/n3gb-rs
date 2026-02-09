use crate::cell::HexCell;
use crate::coord::Crs;
use crate::error::N3gbError;
use crate::geom::parse_geometry;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;

enum SourceIndices {
    Geometry(usize),
    Coordinates { x_idx: usize, y_idx: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryFormat {
    /// Well-Known Text format (e.g., "POLYGON((...))")
    Wkt,
    /// GeoJSON format
    GeoJson,
}

#[derive(Debug, Clone)]
pub enum CoordinateSource {
    /// A single column containing WKT or GeoJSON geometry
    GeometryColumn(String),
    /// Separate X and Y coordinate columns (e.g., Easting/Northing or Lon/Lat)
    CoordinateColumns { x_column: String, y_column: String },
}

#[derive(Debug, Clone)]
pub struct CsvHexConfig {
    pub source: CoordinateSource,
    pub exclude_columns: Vec<String>,
    pub zoom_level: u8,
    pub crs: Crs,
    pub include_hex_geometry: Option<GeometryFormat>,
}

impl CsvHexConfig {
    /// Create config for a CSV with a geometry column (WKT or GeoJSON).
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::CsvHexConfig;
    ///
    /// let config = CsvHexConfig::new("geometry", 12);
    /// ```
    pub fn new(geometry_column: impl Into<String>, zoom_level: u8) -> Self {
        Self {
            source: CoordinateSource::GeometryColumn(geometry_column.into()),
            exclude_columns: Vec::new(),
            zoom_level,
            crs: Crs::default(),
            include_hex_geometry: None,
        }
    }

    /// Create config for a CSV with separate X/Y coordinate columns.
    ///
    /// # Example
    /// ```
    /// use n3gb_rs::{CsvHexConfig, Crs};
    ///
    /// // For BNG coordinates (Easting/Northing)
    /// let config = CsvHexConfig::from_coords("Easting", "Northing", 12)
    ///     .crs(Crs::Bng);
    ///
    /// // For WGS84 coordinates (Longitude/Latitude)
    /// let config = CsvHexConfig::from_coords("Longitude", "Latitude", 12)
    ///     .crs(Crs::Wgs84);
    /// ```
    pub fn from_coords(
        x_column: impl Into<String>,
        y_column: impl Into<String>,
        zoom_level: u8,
    ) -> Self {
        Self {
            source: CoordinateSource::CoordinateColumns {
                x_column: x_column.into(),
                y_column: y_column.into(),
            },
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

    // Include hex polygon geometry in output.
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

fn polygon_to_wkt(polygon: &geo_types::Polygon<f64>) -> String {
    use wkt::ToWkt;
    polygon.wkt_string()
}

fn polygon_to_geojson(polygon: &geo_types::Polygon<f64>) -> String {
    let geom = geojson::Geometry::from(polygon);
    geom.to_string()
}

/// Converts a CSV file with geometry or coordinate columns to a CSV file with hex IDs.
///
/// Streams output to minimize memory usage for large files.
///
/// # Example with geometry column (WKT or GeoJSON)
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
///
/// # Example with coordinate columns
///
/// ```no_run
/// use n3gb_rs::{csv_to_hex_csv, CsvHexConfig, Crs};
///
/// // For BNG (Easting/Northing)
/// let config = CsvHexConfig::from_coords("Easting", "Northing", 12)
///     .crs(Crs::Bng);
///
/// csv_to_hex_csv("bus_stops.csv", "output.csv", &config).unwrap();
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

    // Determine which columns to exclude based on source type
    // Best practice is to always exclude ANY geometry column
    let (source_indices, mut exclude_indices) =
        match &config.source {
            CoordinateSource::GeometryColumn(col) => {
                if col.is_empty() {
                    return Err(N3gbError::CsvError(
                        "Geometry column name cannot be empty".to_string(),
                    ));
                }
                let idx = headers.iter().position(|h| h == col).ok_or_else(|| {
                    N3gbError::CsvError(format!("Geometry column '{}' not found", col))
                })?;
                let mut exclude = HashSet::new();
                exclude.insert(idx);
                (SourceIndices::Geometry(idx), exclude)
            }
            CoordinateSource::CoordinateColumns { x_column, y_column } => {
                if x_column.is_empty() {
                    return Err(N3gbError::CsvError(
                        "X column name cannot be empty".to_string(),
                    ));
                }
                if y_column.is_empty() {
                    return Err(N3gbError::CsvError(
                        "Y column name cannot be empty".to_string(),
                    ));
                }
                let x_idx = headers.iter().position(|h| h == x_column).ok_or_else(|| {
                    N3gbError::CsvError(format!("X column '{}' not found", x_column))
                })?;
                let y_idx = headers.iter().position(|h| h == y_column).ok_or_else(|| {
                    N3gbError::CsvError(format!("Y column '{}' not found", y_column))
                })?;
                let mut exclude = HashSet::new();
                exclude.insert(x_idx);
                exclude.insert(y_idx);
                (SourceIndices::Coordinates { x_idx, y_idx }, exclude)
            }
        };

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

        let cells = match &source_indices {
            SourceIndices::Geometry(idx) => {
                let geom_str = record.get(*idx).ok_or_else(|| {
                    N3gbError::CsvError(format!("Missing geometry column at index {}", idx))
                })?;
                let geom = parse_geometry(geom_str)?;
                HexCell::from_geometry(geom, config.zoom_level, config.crs)?
            }
            SourceIndices::Coordinates { x_idx, y_idx } => {
                let x_str = record
                    .get(*x_idx)
                    .ok_or_else(|| {
                        N3gbError::CsvError(format!("Missing X column at index {}", x_idx))
                    })?
                    .trim();
                let y_str = record
                    .get(*y_idx)
                    .ok_or_else(|| {
                        N3gbError::CsvError(format!("Missing Y column at index {}", y_idx))
                    })?
                    .trim();

                let x: f64 = x_str.parse().map_err(|_| {
                    N3gbError::CsvError(format!("Invalid X coordinate: '{}'", x_str))
                })?;
                let y: f64 = y_str.parse().map_err(|_| {
                    N3gbError::CsvError(format!("Invalid Y coordinate: '{}'", y_str))
                })?;

                // TODO: Could replace with from geometry call?
                let cell = match config.crs {
                    Crs::Wgs84 => HexCell::from_wgs84(&(x, y), config.zoom_level)?,
                    Crs::Bng => HexCell::from_bng(&(x, y), config.zoom_level)?,
                };
                vec![cell]
            }
        };

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
    fn test_csv_from_coords_bng() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "StopCode,Name,Easting,Northing")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ABC123,Temple Meads,359581,172304")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "DEF456,Castle Park,358500,173100")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::from_coords("Easting", "Northing", 12).crs(Crs::Bng);
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        assert!(output_path.exists());

        let output =
            std::fs::read_to_string(&output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        assert!(output.contains("hex_id"));
        assert!(output.contains("StopCode"));
        assert!(output.contains("Name"));
        assert!(!output.contains(",Easting,"));
        assert!(!output.contains(",Northing"));

        Ok(())
    }

    #[test]
    fn test_csv_from_coords_wgs84() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ID,Longitude,Latitude,Description")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "1,-2.58302,51.44827,Bristol Temple Meads")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::from_coords("Longitude", "Latitude", 12).crs(Crs::Wgs84);
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        assert!(output_path.exists());
        Ok(())
    }
}
