pub mod coord;
pub mod error;
pub mod identifier;

pub use coord::{Coordinate, bng_to_wgs84, wgs84_to_bng};
pub use error::N3gbError;
pub use identifier::{decode_hex_identifier, generate_identifier};
