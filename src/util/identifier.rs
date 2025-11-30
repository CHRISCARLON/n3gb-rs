use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use crate::core::constants::{IDENTIFIER_VERSION, SCALE_FACTOR};
use crate::util::error::N3gbError;

pub fn generate_identifier(easting: f64, northing: f64, zoom_level: u8) -> String {
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

        let id = generate_identifier(easting, northing, zoom);
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
}
