use crate::error::N3gbError;
use crate::index::constants::{IDENTIFIER_VERSION, SCALE_FACTOR};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

/// Generates a unique hex cell identifier from BNG coordinates and zoom level.
///
/// The identifier is a URL-safe Base64 string encoding a 19-byte binary structure.
///
/// # Binary Format
///
/// The identifier encodes the following data in big-endian byte order:
///
/// | Offset | Size | Field       | Description                                      |
/// |--------|------|-------------|--------------------------------------------------|
/// | 0      | 1    | Version     | Identifier format version (currently 1)          |
/// | 1      | 8    | Easting     | BNG easting scaled by `SCALE_FACTOR` as `u64`    |
/// | 9      | 8    | Northing    | BNG northing scaled by `SCALE_FACTOR` as `u64`   |
/// | 17     | 1    | Zoom Level  | Grid zoom level (0-15)                           |
/// | 18     | 1    | Checksum    | Wrapping sum of bytes 0-17 for validation        |
///
/// # Process
///
/// 1. Multiplies coordinates by `SCALE_FACTOR` and rounds to preserve precision
/// 2. Packs version, scaled coordinates, and zoom level into 18 bytes
/// 3. Computes a checksum by summing all bytes (with wrapping)
/// 4. Appends the checksum byte
/// 5. Encodes the 19 bytes as URL-safe Base64 (no padding)
///
/// # Example
/// ```
/// use n3gb_rs::generate_hex_identifier;
///
/// let id = generate_hex_identifier(457500.0, 340000.0, 10);
/// assert!(!id.is_empty());
/// println!("{}", id);
/// ```
pub fn generate_hex_identifier(easting: f64, northing: f64, zoom_level: u8) -> String {
    let easting_int = (easting * SCALE_FACTOR as f64).round() as u64;
    let northing_int = (northing * SCALE_FACTOR as f64).round() as u64;

    let mut binary_data = Vec::with_capacity(18);
    binary_data.push(IDENTIFIER_VERSION);
    binary_data.extend_from_slice(&easting_int.to_be_bytes());
    binary_data.extend_from_slice(&northing_int.to_be_bytes());
    binary_data.push(zoom_level);

    let checksum: u8 = binary_data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    binary_data.push(checksum);

    URL_SAFE_NO_PAD.encode(&binary_data)
}

/// Decodes a hex cell identifier back to its component parts.
///
/// Parses the URL-safe Base64 identifier and extracts the original BNG coordinates
/// and zoom level.
///
/// # Process
///
/// 1. Decodes the Base64 string to 19 bytes
/// 2. Validates the length is exactly 19 bytes
/// 3. Extracts and verifies the checksum (last byte) against bytes 0-17
/// 4. Extracts the version byte and validates it matches the current version
/// 5. Reads the 8-byte easting and northing values (big-endian `u64`)
/// 6. Divides by `SCALE_FACTOR` to restore the original `f64` coordinates
/// 7. Extracts the zoom level byte
///
/// # Returns
///
/// A tuple of `(version, easting, northing, zoom_level)` on success.
///
/// # Example
/// ```
/// use n3gb_rs::{generate_hex_identifier, decode_hex_identifier};
///
/// let id = generate_hex_identifier(457500.0, 340000.0, 10);
/// let (version, easting, northing, zoom) = decode_hex_identifier(&id).unwrap();
///
/// assert_eq!(version, 1);
/// assert!((easting - 457500.0).abs() < 0.001);
/// assert!((northing - 340000.0).abs() < 0.001);
/// assert_eq!(zoom, 10);
/// ```
///
/// # Errors
///
/// - [`N3gbError::Base64DecodeError`] - Invalid Base64 encoding
/// - [`N3gbError::InvalidIdentifierLength`] - Decoded data is not 19 bytes
/// - [`N3gbError::InvalidChecksum`] - Checksum validation failed
/// - [`N3gbError::UnsupportedVersion`] - Version byte doesn't match current version
pub fn decode_hex_identifier(identifier: &str) -> Result<(u8, f64, f64, u8), N3gbError> {
    let binary_data = URL_SAFE_NO_PAD
        .decode(identifier)
        .map_err(|_| N3gbError::Base64DecodeError)?;

    if binary_data.len() != 19 {
        return Err(N3gbError::InvalidIdentifierLength);
    }

    let (data, checksum_bytes) = binary_data.split_at(18);
    let checksum = checksum_bytes[0];

    let calculated_checksum: u8 = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    if calculated_checksum != checksum {
        return Err(N3gbError::InvalidChecksum);
    }

    let version = data[0];
    let easting_bytes: [u8; 8] = data[1..9]
        .try_into()
        .map_err(|_| N3gbError::InvalidIdentifierLength)?;
    let northing_bytes: [u8; 8] = data[9..17]
        .try_into()
        .map_err(|_| N3gbError::InvalidIdentifierLength)?;
    let easting_int = u64::from_be_bytes(easting_bytes);
    let northing_int = u64::from_be_bytes(northing_bytes);
    let zoom = data[17];

    if version != IDENTIFIER_VERSION {
        return Err(N3gbError::UnsupportedVersion(version));
    }

    let easting = easting_int as f64 / SCALE_FACTOR as f64;
    let northing = northing_int as f64 / SCALE_FACTOR as f64;

    Ok((version, easting, northing, zoom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_decode_identifier() -> Result<(), N3gbError> {
        let easting = 252086.123;
        let northing = 847702.123;
        let zoom = 10;

        let id = generate_hex_identifier(easting, northing, zoom);
        assert!(!id.is_empty());

        let (version, decoded_e, decoded_n, decoded_z) = decode_hex_identifier(&id)?;

        assert_eq!(version, IDENTIFIER_VERSION);
        assert!((decoded_e - easting).abs() < 0.001);
        assert!((decoded_n - northing).abs() < 0.001);
        assert_eq!(decoded_z, zoom);
        Ok(())
    }

    #[test]
    fn test_invalid_identifier() {
        let result = decode_hex_identifier("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_identifier_output() {
        let id = generate_hex_identifier(457500.0, 340000.0, 10);
        println!("Generated identifier: {}", id);
        println!("Length: {} chars", id.len());

        let (version, easting, northing, zoom) = decode_hex_identifier(&id).unwrap();
        println!(
            "Decoded: version={}, easting={}, northing={}, zoom={}",
            version, easting, northing, zoom
        );
    }
}
