# n3gb-rs

Rust implementation of hex-based spatial indexing for British National Grid.

Inspired by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb) and [h3o](https://github.com/HydroniumLabs/h3o).

## Core Structs

### HexCell

A single hexagonal cell with a unique ID, center point, and zoom level.

**Create one:**

```rust
use n3gb_rs::HexCell;
use geo_types::Point;

// From BNG coordinates
let point = Point::new(383640.0, 398260.0);
let cell = HexCell::from_bng(&point, 12)?;

// From an existing ID
let cell = HexCell::from_id("AAF3kQBBMZQM")?;
```

**What you can do:**

```rust
cell.id              // Unique Base64 identifier
cell.easting()       // Center X coordinate
cell.northing()      // Center Y coordinate
cell.to_polygon()    // Get the hex as a geo_types Polygon
```

### HexGrid

A collection of HexCells covering a bounding box.

**Create one:**

```rust
use n3gb_rs::HexGrid;

// Using builder
let grid = HexGrid::builder()
    .zoom_level(12)
    .bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0))
    .build();

// Direct construction
let grid = HexGrid::from_bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0), 12);
```

**What you can do:**

```rust
use geo_types::point;

// Look up a cell by point
let pt = point! { x: 325000.0, y: 325000.0 };
if let Some(cell) = grid.get_cell_at(&pt) {
    println!("{}", cell.id);
}

// Iterate over all cells
for cell in grid.cells() {
    println!("{}", cell.id);
}

// Export to GeoParquet
grid.to_geoparquet("output.parquet")?;

// Export to Arrow RecordBatch
let batch = grid.to_record_batch()?;
```

## API Reference

For people used to similar hexagonal indexing systems (like H3), here is the mapping to n3gb-rs.

### Indexing functions

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| Point to cell (BNG)      | `HexCell::from_bng`                      |
| Point to cell (WGS84)    | `HexCell::from_wgs84`                    |
| Cell ID to cell          | `HexCell::from_hex_id`                   |
| Generate cell ID         | `generate_hex_identifier`                |
| Decode cell ID           | `decode_hex_identifier`                  |
| Point to row/col         | `point_to_row_col`                       |
| Row/col to center        | `row_col_to_center`                      |

### Cell inspection functions

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| Get zoom level           | `HexCell::zoom_level`                    |
| Get cell ID              | `HexCell::id`                            |
| Get center point         | `HexCell::center`                        |
| Get easting              | `HexCell::easting`                       |
| Get northing             | `HexCell::northing`                      |
| Get row index            | `HexCell::row`                           |
| Get column index         | `HexCell::col`                           |
| Cell to polygon          | `HexCell::to_polygon`                    |

### Grid functions

| Concept                   | n3gb-rs                                 |
| :------------------------ | :-------------------------------------- |
| Grid from extent (BNG)    | `HexGrid::from_bng_extent`              |
| Grid from extent (WGS84)  | `HexGrid::from_wgs84_extent`            |
| Grid from rect            | `HexGrid::from_rect`                    |
| Grid from polygon (BNG)   | `HexGrid::from_bng_polygon`             |
| Grid from polygon (WGS84) | `HexGrid::from_wgs84_polygon`           |
| Grid from multipolygon    | `HexGrid::from_bng_multipolygon`        |
| Grid builder              | `HexGridBuilder`                        |
| Get cells                 | `HexGrid::cells`                        |
| Get cell count            | `HexGrid::len`                          |
| Find cell at point        | `HexGrid::get_cell_at`                  |
| Filter cells              | `HexGrid::filter`                       |
| Grid to polygons          | `HexGrid::to_polygons`                  |

### Line coverage functions

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| Line to cells (BNG)      | `HexCell::from_line_string_bng`          |
| Line to cells (WGS84)    | `HexCell::from_line_string_wgs84`        |

### Coordinate transformation functions

| Concept                   | n3gb-rs                                 |
| :------------------------ | :-------------------------------------- |
| WGS84 to BNG point        | `wgs84_to_bng`                          |
| WGS84 to BNG polygon      | `wgs84_polygon_to_bng`                  |
| WGS84 to BNG multipolygon | `wgs84_multipolygon_to_bng`             |
| WGS84 to BNG line         | `wgs84_line_to_bng`                     |

### Hexagon dimension functions

| Concept                    | n3gb-rs                                |
| :------------------------- | :------------------------------------- |
| Dims from side length      | `HexagonDims::from_side`               |
| Dims from circumradius     | `HexagonDims::from_circumradius`       |
| Dims from apothem          | `HexagonDims::from_apothem`            |
| Dims from flat-to-flat     | `HexagonDims::from_across_flats`       |
| Dims from corner-to-corner | `HexagonDims::from_across_corners`     |
| Dims from area             | `HexagonDims::from_area`               |
| Bounding box               | `bounding_box`                         |

### Geometry functions

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| Create hex cell polygon  | `create_hexagon` (used in to_polygon)    |

### Arrow/Parquet I/O functions

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| Cell to Arrow points     | `HexCell::to_arrow_points`               |
| Cell to Arrow polygons   | `HexCell::to_arrow_polygons`             |
| Cell to RecordBatch      | `HexCell::to_record_batch`               |
| Cell to GeoParquet       | `HexCell::to_geoparquet`                 |
| Grid to Arrow points     | `HexGrid::to_arrow_points`               |
| Grid to Arrow polygons   | `HexGrid::to_arrow_polygons`             |
| Grid to RecordBatch      | `HexGrid::to_record_batch`               |
| Grid to GeoParquet       | `HexGrid::to_geoparquet`                 |
| Write GeoParquet         | `write_geoparquet`                       |

### CSV I/O functions

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| CSV to hex-indexed CSV   | `csv_to_hex_csv`                         |
| CSV config (geometry)    | `CsvHexConfig::new`                      |
| CSV config (coords)      | `CsvHexConfig::from_coords`              |

### Constants

| Concept                  | n3gb-rs                                  |
| :----------------------- | :--------------------------------------- |
| Max zoom level           | `MAX_ZOOM_LEVEL`                         |
| Cell radii by zoom       | `CELL_RADIUS`                            |
| Cell widths by zoom      | `CELL_WIDTHS`                            |
| Grid extents (BNG)       | `GRID_EXTENTS`                           |
| Identifier version       | `IDENTIFIER_VERSION`                     |
