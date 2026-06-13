use crate::cell::HexCell;
use crate::coord::{ConversionMethod, Crs};
use crate::error::N3gbError;
use crate::geom::parse_geometry;
use std::collections::{HashMap, HashSet};
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

/// Configuration controlling how a CSV file is converted into hex IDs.
#[derive(Debug, Clone)]
pub struct CsvHexConfig {
    pub source: CoordinateSource,
    pub exclude_columns: Vec<String>,
    pub zoom_level: u8,
    pub crs: Crs,
    pub include_hex_geometry: Option<GeometryFormat>,
    pub hex_density: bool,
    pub conversion_method: ConversionMethod,
}

impl CsvHexConfig {
    /// Create config for a CSV with a geometry column (WKT or GeoJSON).
    ///
    /// # Arguments
    /// * `geometry_column` - Name of the CSV column containing WKT or GeoJSON geometry.
    /// * `zoom_level` - Hex zoom level to encode cells at.
    ///
    /// # Returns
    /// A new [`CsvHexConfig`] using the given geometry column as its coordinate source.
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
            hex_density: false,
            conversion_method: ConversionMethod::default(),
        }
    }

    /// Create config for a CSV with separate X/Y coordinate columns.
    ///
    /// # Arguments
    /// * `x_column` - Name of the CSV column holding the X coordinate (Easting or Longitude).
    /// * `y_column` - Name of the CSV column holding the Y coordinate (Northing or Latitude).
    /// * `zoom_level` - Hex zoom level to encode cells at.
    ///
    /// # Returns
    /// A new [`CsvHexConfig`] using the given coordinate columns as its source.
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
            hex_density: false,
            conversion_method: ConversionMethod::default(),
        }
    }

    /// Set the columns to drop from the output.
    ///
    /// # Arguments
    /// * `columns` - Names of input columns to exclude from the output rows.
    ///
    /// # Returns
    /// The updated config for chaining.
    pub fn exclude(mut self, columns: Vec<String>) -> Self {
        self.exclude_columns = columns;
        self
    }

    /// Set the coordinate reference system of the input data.
    ///
    /// # Arguments
    /// * `crs` - The [`Crs`] of the input coordinates or geometry.
    ///
    /// # Returns
    /// The updated config for chaining.
    pub fn crs(mut self, crs: Crs) -> Self {
        self.crs = crs;
        self
    }

    /// Include hex polygon geometry in output.
    ///
    /// # Arguments
    /// * `format` - The [`GeometryFormat`] (WKT or GeoJSON) for the emitted hex geometry.
    ///
    /// # Returns
    /// The updated config for chaining.
    pub fn with_hex_geometry(mut self, format: GeometryFormat) -> Self {
        self.include_hex_geometry = Some(format);
        self
    }

    /// Sets the WGS84→BNG conversion backend.
    ///
    /// Only relevant when `crs` is [`Crs::Wgs84`]. Defaults to [`ConversionMethod::Proj`].
    ///
    /// # Arguments
    /// * `method` - The [`ConversionMethod`] backend used to convert WGS84 to BNG.
    ///
    /// # Returns
    /// The updated config for chaining.
    pub fn conversion_method(mut self, method: ConversionMethod) -> Self {
        self.conversion_method = method;
        self
    }

    /// Aggregate output to one row per hex cell with a count of input rows.
    ///
    /// Output columns: `hex_id`, `count` (and optionally `hex_geometry`).
    /// Input attribute columns are dropped since rows are aggregated.
    ///
    /// # Returns
    /// The updated config for chaining.
    pub fn hex_density(mut self) -> Self {
        self.hex_density = true;
        self
    }
}

/// Convert a single CSV record into the hex cells it covers.
///
/// # Arguments
/// * `record` - The CSV record to read coordinate or geometry values from.
/// * `source_indices` - Resolved column indices identifying the geometry or X/Y columns.
/// * `config` - Conversion configuration (zoom level, CRS, conversion method).
///
/// # Returns
/// The hex cells covered by the record, or an empty vector if the coordinates fall
/// outside the projectable area.
///
/// # Errors
/// Returns [`N3gbError::CsvError`] if a referenced column is missing or a coordinate
/// fails to parse, [`N3gbError::GeometryParseError`] if a geometry value cannot be
/// parsed, and [`N3gbError::InvalidZoomLevel`] if the configured zoom level is invalid.
fn read_cells_from_record(
    record: &csv::StringRecord,
    source_indices: &SourceIndices,
    config: &CsvHexConfig,
) -> Result<Vec<HexCell>, N3gbError> {
    match source_indices {
        SourceIndices::Geometry(idx) => {
            let geom_str = record.get(*idx).ok_or_else(|| {
                N3gbError::CsvError(format!("Missing geometry column at index {}", idx))
            })?;
            let geom = parse_geometry(geom_str)?;
            match HexCell::from_geometry(
                geom,
                config.zoom_level,
                config.crs,
                config.conversion_method,
            ) {
                Ok(cells) => Ok(cells),
                Err(N3gbError::ProjectionError(_)) => Ok(vec![]),
                Err(e) => Err(e),
            }
        }
        SourceIndices::Coordinates { x_idx, y_idx } => {
            let x_str = record
                .get(*x_idx)
                .ok_or_else(|| N3gbError::CsvError(format!("Missing X column at index {}", x_idx)))?
                .trim();
            let y_str = record
                .get(*y_idx)
                .ok_or_else(|| N3gbError::CsvError(format!("Missing Y column at index {}", y_idx)))?
                .trim();

            let x: f64 = x_str
                .parse()
                .map_err(|_| N3gbError::CsvError(format!("Invalid X coordinate: '{}'", x_str)))?;
            let y: f64 = y_str
                .parse()
                .map_err(|_| N3gbError::CsvError(format!("Invalid Y coordinate: '{}'", y_str)))?;

            use crate::coord::convert_to_bng;
            let cell = match config.crs {
                Crs::Wgs84 => match convert_to_bng(&(x, y), config.conversion_method) {
                    Ok(bng) => HexCell::from_bng(&bng, config.zoom_level)?,
                    Err(N3gbError::ProjectionError(_)) => return Ok(vec![]),
                    Err(e) => return Err(e),
                },
                Crs::Bng => HexCell::from_bng(&(x, y), config.zoom_level)?,
            };
            Ok(vec![cell])
        }
    }
}

/// Aggregate records into one output row per hex cell with a count of input rows.
///
/// # Arguments
/// * `reader` - The CSV reader positioned after the header row.
/// * `source_indices` - Resolved column indices identifying the geometry or X/Y columns.
/// * `output_path` - Path of the CSV file to write the aggregated counts to.
/// * `config` - Conversion configuration controlling zoom, CRS, and optional hex geometry.
///
/// # Returns
/// `()` on success, after the aggregated CSV has been written and flushed.
///
/// # Errors
/// Returns [`N3gbError::CsvError`] if reading or writing records fails, or for a missing
/// or invalid coordinate column; [`N3gbError::GeometryParseError`] if a geometry value
/// cannot be parsed; [`N3gbError::InvalidZoomLevel`] if the configured zoom level is
/// invalid; and [`N3gbError::IoError`] if the output file cannot be created.
fn csv_to_hex_density(
    mut reader: csv::Reader<File>,
    source_indices: SourceIndices,
    output_path: impl AsRef<Path>,
    config: &CsvHexConfig,
) -> Result<(), N3gbError> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for result in reader.records() {
        let record = result?;
        let cells = read_cells_from_record(&record, &source_indices, config)?;

        for cell in cells {
            *counts.entry(cell.id).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let out_file = File::create(output_path)?;
    let mut writer = csv::Writer::from_writer(out_file);

    let mut header_row: Vec<&str> = vec!["hex_id", "count"];
    if config.include_hex_geometry.is_some() {
        header_row.push("hex_geometry");
    }
    writer.write_record(&header_row)?;

    for (hex_id, count) in &sorted {
        let mut row: Vec<String> = vec![hex_id.clone(), count.to_string()];

        if let Some(format) = config.include_hex_geometry {
            let cell = HexCell::from_hex_id(hex_id)?;
            let polygon = cell.to_polygon();
            let geom_str = match format {
                GeometryFormat::Wkt => polygon_to_wkt(&polygon),
                GeometryFormat::GeoJson => polygon_to_geojson(&polygon),
            };
            row.push(geom_str);
        }

        writer.write_record(&row)?;
    }

    writer.flush()?;

    Ok(())
}

/// Render a polygon as a Well-Known Text (WKT) string.
///
/// # Arguments
/// * `polygon` - The polygon to serialize.
///
/// # Returns
/// The WKT representation of the polygon.
fn polygon_to_wkt(polygon: &geo_types::Polygon<f64>) -> String {
    use wkt::ToWkt;
    polygon.wkt_string()
}

/// Render a polygon as a GeoJSON geometry string.
///
/// # Arguments
/// * `polygon` - The polygon to serialize.
///
/// # Returns
/// The GeoJSON representation of the polygon.
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
///
/// # Arguments
/// * `csv_path` - Path of the input CSV file to read.
/// * `output_path` - Path of the CSV file to write hex IDs (and optional geometry) to.
/// * `config` - Conversion configuration describing the source columns, zoom, and CRS.
///
/// # Returns
/// `()` on success, after the output CSV has been written and flushed.
///
/// # Errors
/// Returns [`N3gbError::CsvError`] if the input cannot be read, a configured column name
/// is empty or not found, or a record cannot be read or written;
/// [`N3gbError::GeometryParseError`] if a geometry value cannot be parsed;
/// [`N3gbError::InvalidZoomLevel`] if the configured zoom level is invalid; and
/// [`N3gbError::IoError`] if the input file cannot be opened or the output file cannot
/// be created.
pub fn csv_to_hex_csv(
    csv_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    config: &CsvHexConfig,
) -> Result<(), N3gbError> {
    let file = File::open(csv_path)?;
    let mut reader = csv::Reader::from_reader(file);

    let headers = reader.headers()?.clone();

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

    if config.hex_density {
        return csv_to_hex_density(reader, source_indices, output_path, config);
    }

    let out_file = File::create(output_path)?;
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
    writer.write_record(&header_row)?;

    for result in reader.records() {
        let record = result?;

        let cells = read_cells_from_record(&record, &source_indices, config)?;

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
            writer.write_record(&row)?;
        }
    }

    writer.flush()?;

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
    fn test_csv_hex_density() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "StopCode,Name,Easting,Northing")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;

        writeln!(file, "ABC123,Stop A,359581,172304")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "DEF456,Stop B,359582,172305")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;

        writeln!(file, "GHI789,Stop C,350000,170000")
            .map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::from_coords("Easting", "Northing", 12)
            .crs(Crs::Bng)
            .hex_density();
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        let output =
            std::fs::read_to_string(&output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines[0], "hex_id,count");
        assert_eq!(lines.len(), 3);

        assert!(lines[1].ends_with(",2"));
        assert!(lines[2].ends_with(",1"));

        assert!(!output.contains("StopCode"));
        assert!(!output.contains("Name"));

        Ok(())
    }

    #[test]
    fn test_csv_hex_density_with_geometry() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ID,Easting,Northing").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "1,359581,172304").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "2,359582,172305").map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::from_coords("Easting", "Northing", 12)
            .crs(Crs::Bng)
            .hex_density()
            .with_hex_geometry(GeometryFormat::Wkt);
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        let output =
            std::fs::read_to_string(&output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines[0], "hex_id,count,hex_geometry");
        assert!(lines[1].contains("POLYGON"));
        Ok(())
    }

    #[test]
    fn test_csv_hex_density_wgs84() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ID,Lon,Lat").map_err(|e| N3gbError::IoError(e.to_string()))?;
        // Three points near Bristol — two very close (same hex), one further away
        writeln!(file, "1,-2.583,51.448").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "2,-2.583,51.448").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "3,-1.500,53.800").map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::from_coords("Lon", "Lat", 8)
            .crs(Crs::Wgs84)
            .hex_density();
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        let output =
            std::fs::read_to_string(&output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        let lines: Vec<&str> = output.lines().collect();

        // header + two distinct hex cells
        assert_eq!(lines[0], "hex_id,count");
        assert_eq!(lines.len(), 3);
        // highest count first
        assert!(lines[1].ends_with(",2"));
        assert!(lines[2].ends_with(",1"));
        Ok(())
    }

    #[test]
    fn test_csv_hex_density_geometry_column() -> Result<(), N3gbError> {
        let dir = tempdir().map_err(|e| N3gbError::IoError(e.to_string()))?;
        let csv_path = dir.path().join("test.csv");
        let output_path = dir.path().join("output.csv");

        let mut file = File::create(&csv_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "ID,geometry").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "1,POINT(530000 180000)").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "2,POINT(530001 180001)").map_err(|e| N3gbError::IoError(e.to_string()))?;
        writeln!(file, "3,POINT(400000 300000)").map_err(|e| N3gbError::IoError(e.to_string()))?;

        let config = CsvHexConfig::new("geometry", 10)
            .crs(Crs::Bng)
            .hex_density();
        csv_to_hex_csv(&csv_path, &output_path, &config)?;

        let output =
            std::fs::read_to_string(&output_path).map_err(|e| N3gbError::IoError(e.to_string()))?;
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines[0], "hex_id,count");
        // first two points should be in the same hex at zoom 10, third in a different one
        assert_eq!(lines.len(), 3);
        assert!(lines[1].ends_with(",2"));
        assert!(lines[2].ends_with(",1"));
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
