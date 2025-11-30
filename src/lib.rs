//! # n3gb-rs - NUAR Hex-Based Spatial Indexing System
//!
//! ## Two Entry Points
//!
//! ### 1. `HexCell` - Single Cell Operations
//!
//! ```
//! use n3gb_rs::HexCell;
//!
//! # fn main() -> Result<(), n3gb_rs::N3gbError> {
//! let cell = HexCell::from_wgs84(-2.248, 53.481, 12)?;
//! let cell = HexCell::from_bng(383640.0, 398260.0, 12)?;
//! println!("{}", cell.id);
//! let polygon = cell.to_polygon();
//! # Ok(())
//! # }
//! ```
//!
//! ### 2. `HexGrid` - Collections of Cells
//!
//! ```
//! use n3gb_rs::HexGrid;
//! use geo_types::point;
//!
//! let grid = HexGrid::builder()
//!     .zoom_level(10)
//!     .extent(457000.0, 339500.0, 458000.0, 340500.0)
//!     .build();
//!
//! let pt = point! { x: 457500.0, y: 340000.0 };
//! if let Some(cell) = grid.get_cell_at(&pt) {
//!     println!("{}", cell.id);
//! }
//! ```
//!

pub mod api;
pub mod core;
pub mod util;

pub use api::{HexCell, HexGrid, HexGridBuilder};
pub use core::{
    CELL_RADIUS, CELL_WIDTHS, GRID_EXTENTS, IDENTIFIER_VERSION, MAX_ZOOM_LEVEL,
    bounding_box, from_across_corners, from_across_flats, from_apothem, from_area,
    from_circumradius, from_side, HexagonDims,
    create_hexagon, create_hexagon_from_point,
    hex_to_point, point_to_hex, point_to_hex_coord,
};
pub use util::{
    N3gbError,
    bng_to_wgs84, bng_to_wgs84_point, wgs84_to_bng, wgs84_to_bng_point,
    decode_hex_identifier, generate_identifier,
};

pub use geo_types;

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{coord, point, Rect};

    #[test]
    fn test_end_to_end_workflow() -> Result<(), N3gbError> {
        let grid = HexGrid::builder()
            .zoom_level(10)
            .extent(457000.0, 339500.0, 458000.0, 340500.0)
            .build();

        assert!(!grid.is_empty());
        assert_eq!(grid.zoom_level(), 10);

        let pt = point! { x: 457500.0, y: 340000.0 };
        let cell = grid.get_cell_at(&pt);
        assert!(cell.is_some());

        if let Some(cell) = cell {
            let (version, _easting, _northing, zoom) = decode_hex_identifier(&cell.id)?;
            assert_eq!(version, IDENTIFIER_VERSION);
            assert_eq!(zoom, 10);

            let polygon = cell.to_polygon();
            assert_eq!(polygon.exterior().coords().count(), 7);
        }
        Ok(())
    }

    #[test]
    fn test_using_geo_types_macros() -> Result<(), N3gbError> {
        let pt = point! { x: 457996.0, y: 339874.0 };
        let (row, col) = point_to_hex_coord(&pt, 10)?;
        assert!(row > 0);
        assert!(col > 0);

        let rect = Rect::new(
            coord! { x: 457000.0, y: 339500.0 },
            coord! { x: 458000.0, y: 340500.0 },
        );
        let grid = HexGrid::from_rect(&rect, 10);
        assert!(!grid.is_empty());
        Ok(())
    }

    #[test]
    fn test_dimensions_workflow() -> Result<(), N3gbError> {
        let dims = from_side(10.0)?;

        assert!((dims.a - 10.0).abs() < 0.001);
        assert!((dims.perimeter - 60.0).abs() < 0.001);

        let dims2 = from_area(dims.area)?;
        assert!((dims2.a - 10.0).abs() < 0.001);
        Ok(())
    }

    #[test]
    fn test_grid_iteration() {
        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);

        let mut count = 0;
        for cell in grid.iter() {
            assert_eq!(cell.zoom_level, 10);
            count += 1;
        }

        assert_eq!(count, grid.len());
    }

    #[test]
    fn test_grid_filtering() {
        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);

        let high_easting = grid.filter(|cell| cell.easting() > 457500.0);
        assert!(!high_easting.is_empty());
        assert!(high_easting.len() < grid.len());
    }

    #[test]
    fn test_hexcell_from_bng() -> Result<(), N3gbError> {
        let cell = HexCell::from_bng(383640.0, 398260.0, 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        assert!(cell.row > 0);
        assert!(cell.col > 0);

        let polygon = cell.to_polygon();
        assert_eq!(polygon.exterior().coords().count(), 7);
        Ok(())
    }

    #[test]
    fn test_hexcell_from_wgs84() -> Result<(), N3gbError> {
        let cell = HexCell::from_wgs84(-2.248, 53.481, 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        assert!(cell.easting() > 380000.0 && cell.easting() < 390000.0);
        assert!(cell.northing() > 390000.0 && cell.northing() < 400000.0);
        Ok(())
    }

    #[test]
    fn test_hexcell_consistency_with_hexgrid() -> Result<(), N3gbError> {
        let cell_direct = HexCell::from_bng(457500.0, 340000.0, 10)?;

        let grid = HexGrid::from_extent(457000.0, 339500.0, 458000.0, 340500.0, 10);
        let pt = point! { x: 457500.0, y: 340000.0 };
        let cell_from_grid = grid.get_cell_at(&pt);

        assert!(cell_from_grid.is_some());
        let cell_from_grid = cell_from_grid.unwrap();

        assert_eq!(cell_direct.id, cell_from_grid.id);
        assert_eq!(cell_direct.row, cell_from_grid.row);
        assert_eq!(cell_direct.col, cell_from_grid.col);
        Ok(())
    }
}
