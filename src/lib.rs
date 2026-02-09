//! # n3gb-rs
//!
//! Rust implementation of hex-based spatial indexing for British National Grid.
//!
//! Inspired by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb) and [h3o](https://github.com/HydroniumLabs/h3o).
//!
//! ## Core Structs
//!
//! ### HexCell
//!
//! A single hexagonal cell with a unique ID, center point, and zoom level.
//!
//! **Create one:**
//!
//! ```no_run
//! use n3gb_rs::HexCell;
//! use geo_types::Point;
//!
//! // From BNG coordinates
//! let point = Point::new(383640.0, 398260.0);
//! let cell = HexCell::from_bng(&point, 12).unwrap();
//!
//! // From an existing ID
//! let cell = HexCell::from_hex_id("AAF3kQBBMZQM").unwrap();
//! ```
//!
//! **What you can do:**
//!
//! ```ignore
//! cell.id              // Unique Base64 identifier
//! cell.easting()       // Center X coordinate
//! cell.northing()      // Center Y coordinate
//! cell.to_polygon()    // Get the hex as a geo_types Polygon
//! ```
//!
//! ### HexGrid
//!
//! A collection of HexCells covering a bounding box.
//!
//! **Create one:**
//!
//! ```
//! use n3gb_rs::HexGrid;
//!
//! // Using builder
//! let grid = HexGrid::builder()
//!     .zoom_level(12)
//!     .bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0))
//!     .build();
//!
//! // Direct construction
//! let grid = HexGrid::from_bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0), 12);
//! ```
//!
//! **What you can do:**
//!
//! ```no_run
//! use n3gb_rs::HexGrid;
//! use geo_types::point;
//!
//! # let grid = HexGrid::from_bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0), 12);
//! // Look up a cell by point
//! let pt = point! { x: 325000.0, y: 325000.0 };
//! if let Some(cell) = grid.get_cell_at(&pt) {
//!     println!("{}", cell.id);
//! }
//!
//! // Iterate over all cells
//! for cell in grid.cells() {
//!     println!("{}", cell.id);
//! }
//!
//! // Export to GeoParquet
//! grid.to_geoparquet("output.parquet").unwrap();
//!
//! // Export to Arrow RecordBatch
//! let batch = grid.to_record_batch().unwrap();
//! ```
//!
//! ## API Reference
//!
//! For people used to similar hexagonal indexing systems (like H3), here is the mapping to n3gb-rs.
//!
//! ### Indexing functions
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | Point to cell (BNG)      | `HexCell::from_bng`                      |
//! | Point to cell (WGS84)    | `HexCell::from_wgs84`                    |
//! | Geometry to cells        | `HexCell::from_geometry`                 |
//! | Cell ID to cell          | `HexCell::from_hex_id`                   |
//! | Generate cell ID         | `generate_hex_identifier`                |
//! | Decode cell ID           | `decode_hex_identifier`                  |
//! | Point to row/col         | `point_to_row_col`                       |
//! | Row/col to center        | `row_col_to_center`                      |
//!
//! ### Cell inspection functions
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | Get zoom level           | `HexCell::zoom_level`                    |
//! | Get cell ID              | `HexCell::id`                            |
//! | Get center point         | `HexCell::center`                        |
//! | Get easting              | `HexCell::easting`                       |
//! | Get northing             | `HexCell::northing`                      |
//! | Get row index            | `HexCell::row`                           |
//! | Get column index         | `HexCell::col`                           |
//! | Cell to polygon          | `HexCell::to_polygon`                    |
//!
//! ### Grid functions
//!
//! | Concept                   | n3gb-rs                                 |
//! | :------------------------ | :-------------------------------------- |
//! | Grid from extent (BNG)    | `HexGrid::from_bng_extent`              |
//! | Grid from extent (WGS84)  | `HexGrid::from_wgs84_extent`            |
//! | Grid from rect            | `HexGrid::from_rect`                    |
//! | Grid from polygon (BNG)   | `HexGrid::from_bng_polygon`             |
//! | Grid from polygon (WGS84) | `HexGrid::from_wgs84_polygon`           |
//! | Grid from multipolygon    | `HexGrid::from_bng_multipolygon`        |
//! | Grid builder              | `HexGridBuilder`                        |
//! | Get cells                 | `HexGrid::cells`                        |
//! | Get cell count            | `HexGrid::len`                          |
//! | Find cell at point        | `HexGrid::get_cell_at`                  |
//! | Filter cells              | `HexGrid::filter`                       |
//! | Grid to polygons          | `HexGrid::to_polygons`                  |
//!
//! ### Line coverage functions
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | Line to cells (BNG)      | `HexCell::from_line_string_bng`          |
//! | Line to cells (WGS84)    | `HexCell::from_line_string_wgs84`        |
//!
//! ### Coordinate transformation functions
//!
//! | Concept                   | n3gb-rs                                 |
//! | :------------------------ | :-------------------------------------- |
//! | WGS84 to BNG point        | `wgs84_to_bng`                          |
//! | WGS84 to BNG polygon      | `wgs84_polygon_to_bng`                  |
//! | WGS84 to BNG multipolygon | `wgs84_multipolygon_to_bng`             |
//! | WGS84 to BNG line         | `wgs84_line_to_bng`                     |
//!
//! ### Hexagon dimension functions
//!
//! | Concept                    | n3gb-rs                                |
//! | :------------------------- | :------------------------------------- |
//! | Dims from side length      | `HexagonDims::from_side`               |
//! | Dims from circumradius     | `HexagonDims::from_circumradius`       |
//! | Dims from apothem          | `HexagonDims::from_apothem`            |
//! | Dims from flat-to-flat     | `HexagonDims::from_across_flats`       |
//! | Dims from corner-to-corner | `HexagonDims::from_across_corners`     |
//! | Dims from area             | `HexagonDims::from_area`               |
//! | Bounding box               | `bounding_box`                         |
//!
//! ### Geometry functions
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | Create hex cell polygon  | `create_hexagon` (used in to_polygon)    |
//! | Parse WKT/GeoJSON        | `parse_geometry`                         |
//!
//! ### Arrow/Parquet I/O functions
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | Cell to Arrow points     | `HexCell::to_arrow_points`               |
//! | Cell to Arrow polygons   | `HexCell::to_arrow_polygons`             |
//! | Cell to RecordBatch      | `HexCell::to_record_batch`               |
//! | Cell to GeoParquet       | `HexCell::to_geoparquet`                 |
//! | Grid to Arrow points     | `HexGrid::to_arrow_points`               |
//! | Grid to Arrow polygons   | `HexGrid::to_arrow_polygons`             |
//! | Grid to RecordBatch      | `HexGrid::to_record_batch`               |
//! | Grid to GeoParquet       | `HexGrid::to_geoparquet`                 |
//! | Write GeoParquet         | `write_geoparquet`                       |
//!
//! ### CSV I/O functions
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | CSV to hex-indexed CSV   | `csv_to_hex_csv`                         |
//! | CSV config (geometry)    | `CsvHexConfig::new`                      |
//! | CSV config (coords)      | `CsvHexConfig::from_coords`              |
//!
//! ### Constants
//!
//! | Concept                  | n3gb-rs                                  |
//! | :----------------------- | :--------------------------------------- |
//! | Max zoom level           | `MAX_ZOOM_LEVEL`                         |
//! | Cell radii by zoom       | `CELL_RADIUS`                            |
//! | Cell widths by zoom      | `CELL_WIDTHS`                            |
//! | Grid extents (BNG)       | `GRID_EXTENTS`                           |
//! | Identifier version       | `IDENTIFIER_VERSION`                     |

mod cell;
mod coord;
mod dimensions;
mod error;
mod geom;
mod grid;
mod index;
mod io;

pub use cell::HexCell;
pub use coord::{
    Coordinate, Crs, wgs84_line_to_bng, wgs84_multipolygon_to_bng, wgs84_polygon_to_bng,
    wgs84_to_bng,
};
pub use dimensions::{
    HexagonDims, bounding_box, from_across_corners, from_across_flats, from_apothem, from_area,
    from_circumradius, from_side,
};
pub use error::N3gbError;
pub use grid::{HexGrid, HexGridBuilder};
pub use index::{
    CELL_RADIUS, CELL_WIDTHS, GRID_EXTENTS, IDENTIFIER_VERSION, MAX_ZOOM_LEVEL,
    decode_hex_identifier, generate_hex_identifier, point_to_row_col, row_col_to_center,
};
pub use io::{
    CoordinateSource, CsvHexConfig, CsvToHex, GeometryFormat, HexCellsToArrow,
    HexCellsToGeoParquet, csv_to_hex_csv, write_geoparquet,
};

pub use geom::{create_hexagon, parse_geometry};

pub use geo_types;
pub use geoarrow_array;
pub use geoarrow_schema;
pub use geoparquet;

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::{Rect, coord, point};

    #[test]
    fn test_end_to_end_workflow() -> Result<(), N3gbError> {
        let grid = HexGrid::builder()
            .zoom_level(10)
            .bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0))
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
        let (row, col) = point_to_row_col(&pt, 10)?;
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
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);

        let mut count = 0;
        for cell in grid.iter() {
            assert_eq!(cell.zoom_level, 10);
            count += 1;
        }

        assert_eq!(count, grid.len());
    }

    #[test]
    fn test_grid_filtering() {
        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);

        let high_easting = grid.filter(|cell| cell.easting() > 457500.0);
        assert!(!high_easting.is_empty());
        assert!(high_easting.len() < grid.len());
    }

    #[test]
    fn test_hexcell_from_bng() -> Result<(), N3gbError> {
        let cell = HexCell::from_bng(&(383640.0, 398260.0), 12)?;

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
        let cell = HexCell::from_wgs84(&(-2.248, 53.481), 12)?;

        assert_eq!(cell.zoom_level, 12);
        assert!(!cell.id.is_empty());
        assert!(cell.easting() > 380000.0 && cell.easting() < 390000.0);
        assert!(cell.northing() > 390000.0 && cell.northing() < 400000.0);
        Ok(())
    }

    #[test]
    fn test_hexcell_consistency_with_hexgrid() -> Result<(), N3gbError> {
        let cell_direct = HexCell::from_bng(&(457500.0, 340000.0), 10)?;

        let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);
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
