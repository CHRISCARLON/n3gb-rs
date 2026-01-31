/// Error type for n3gb-rs operations.
#[derive(Debug, PartialEq)]
pub enum N3gbError {
    /// The hex identifier has an invalid length.
    InvalidIdentifierLength,
    /// The hex identifier checksum validation failed.
    InvalidChecksum,
    /// The identifier version is not supported.
    UnsupportedVersion(u8),
    /// The zoom level is outside the valid range (0-15).
    InvalidZoomLevel(u8),
    /// A hexagon dimension value is invalid (e.g., negative).
    InvalidDimension(String),
    /// Failed to decode Base64 identifier.
    Base64DecodeError,
    /// Coordinate projection failed (WGS84 to BNG).
    ProjectionError(String),
    /// File I/O or serialization error.
    IoError(String),
    /// CSV parsing or reading error.
    CsvError(String),
    /// Failed to parse geometry from string (GeoJSON or WKT).
    GeometryParseError(String),
}

impl std::fmt::Display for N3gbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            N3gbError::InvalidIdentifierLength => write!(f, "Invalid identifier length"),
            N3gbError::InvalidChecksum => write!(f, "Invalid checksum"),
            N3gbError::UnsupportedVersion(v) => write!(f, "Unsupported version: {}", v),
            N3gbError::InvalidZoomLevel(z) => write!(f, "Invalid zoom level: {}", z),
            N3gbError::InvalidDimension(msg) => write!(f, "Invalid dimension: {}", msg),
            N3gbError::Base64DecodeError => write!(f, "Base64 decode error"),
            N3gbError::ProjectionError(msg) => write!(f, "Projection error: {}", msg),
            N3gbError::IoError(msg) => write!(f, "IO error: {}", msg),
            N3gbError::CsvError(msg) => write!(f, "CSV error: {}", msg),
            N3gbError::GeometryParseError(msg) => write!(f, "Geometry parse error: {}", msg),
        }
    }
}

impl std::error::Error for N3gbError {}
