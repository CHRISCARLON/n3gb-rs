mod bng_transformations;

pub(crate) use bng_transformations::{
    convert_line_to_bng, convert_multipolygon_to_bng, convert_polygon_to_bng, convert_to_bng,
};

use geo_types::Point;

/// Coordinate reference system for input geometry data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Crs {
    /// WGS84 (EPSG:4326) - longitude/latitude coordinates
    #[default]
    Wgs84,
    /// British National Grid (EPSG:27700) - easting/northing coordinates
    Bng,
}

/// Which backend to use when converting WGS84 coordinates to BNG.
///
/// When PROJ grid files are installed both methods agree to within ~1mm.
/// The difference matters when grid files are absent:
///
/// | Scenario | `Proj` | `Ostn15` |
/// |---|---|---|
/// | PROJ + grid files installed | ~1mm | ~1mm |
/// | PROJ installed, no grid files | ~5m (silent!) | ~1mm |
/// | PROJ not installed | build fails | ~1mm |
///
/// `Ostn15` is the safer default for libraries — accuracy is guaranteed
/// regardless of what the user has installed on their system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConversionMethod {
    /// Use the `lonlat_bng` crate with OSTN15 data embedded at compile time.
    ///
    /// Always ~1mm accurate. No system dependencies.
    #[default]
    Ostn15,
    /// Use the `proj` system library (equivalent to pyproj).
    ///
    /// Requires `libproj` and the `uk_os_OSTN15_NTv2_OSGBtoETRS.tif` grid file.
    /// Without the grid file PROJ silently falls back to a ~5m Helmert transform.
    Proj,
}

/// Trait for types that can provide x/y coordinates.
///
/// Implemented for `(f64, f64)` tuples and `geo_types::Point<f64>`.
/// This allows functions to accept either type.
pub trait Coordinate {
    /// Returns the x-coordinate (easting or longitude).
    fn x(&self) -> f64;
    /// Returns the y-coordinate (northing or latitude).
    fn y(&self) -> f64;
}

impl Coordinate for (f64, f64) {
    fn x(&self) -> f64 {
        self.0
    }
    fn y(&self) -> f64 {
        self.1
    }
}

impl Coordinate for Point<f64> {
    fn x(&self) -> f64 {
        Point::x(*self)
    }
    fn y(&self) -> f64 {
        Point::y(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_trait_tuple() {
        let tuple = (100.0, 200.0);
        assert_eq!(tuple.x(), 100.0);
        assert_eq!(tuple.y(), 200.0);
    }

    #[test]
    fn test_coordinate_trait_point() {
        let point = Point::new(100.0, 200.0);
        assert_eq!(point.x(), 100.0);
        assert_eq!(point.y(), 200.0);
    }

    #[test]
    fn test_crs_enum_default() {
        assert_eq!(Crs::default(), Crs::Wgs84);
    }
}
