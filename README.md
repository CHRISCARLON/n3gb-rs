# n3gb-rs

Rust implementation of hex-based spatial indexing for British National Grid.

Inspired by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb) and [h3o](https://github.com/HydroniumLabs/h3o).

## How It Works

**Creating a HexCell:**
1. `point_to_row_col` — Finds which grid cell coordinates fall into, returns `(row, col)`
2. `row_col_to_center` — Calculates the exact center of that cell (applying offset for odd rows)
3. `generate_hex_identifier` — Packs center coordinates + zoom level into a Base64 identifier

**Reconstructing from ID:**
1. `decode_hex_identifier` — Decodes Base64 back to version, center_x, center_y, zoom
2. `create_hexagon` — Draws 6 vertices around the center at the radius for that zoom level (creates the hex cell polygon)

## Usage

### Single HexCells

```rust
use n3gb_rs::HexCell;
use geo_types::Point;

let point = Point::new(383640.0, 398260.0);
let cell = HexCell::from_bng(&point, 12)?;

println!("ID: {}", cell.id);
println!("Center: ({}, {})", cell.easting(), cell.northing());

let polygon = cell.to_polygon();
```

### Cell Collections (A Grid of HexCells)

```rust
use n3gb_rs::HexGrid;
use geo_types::point;

let grid = HexGrid::builder()
    .zoom_level(12)
    .bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0))
    .build();

let pt = point! { x: 325000.0, y: 325000.0 };
if let Some(cell) = grid.get_cell_at(&pt) {
    println!("{}", cell.id);
}
```

### CSV Conversion

Convert CSV files with geometry columns (WKT or GeoJSON) to hex-indexed CSVs:

```rust
use n3gb_rs::{CsvHexConfig, Crs, csv_to_hex_csv};

let config = CsvHexConfig::new("geometry", 12)
    .crs(Crs::Wgs84)
    .exclude(vec!["Geo Point".into()]);

csv_to_hex_csv("input.csv", "output.csv", &config)?;
```

Or with coordinate columns:

```rust
let config = CsvHexConfig::from_coords("Easting", "Northing", 12)
    .crs(Crs::Bng);

csv_to_hex_csv("input.csv", "output.csv", &config)?;
```

Supported geometry types: Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, GeometryCollection

### Output Formats

Export to GeoParquet or Arrow:

```rust
use n3gb_rs::{HexGrid, HexCellsToArrow, HexCellsToGeoParquet};

let grid = HexGrid::from_bng_extent(&(457000.0, 339500.0), &(458000.0, 340500.0), 10);

// GeoParquet
grid.to_geoparquet("output.parquet")?;

// Arrow
let record_batch = grid.to_record_batch()?;
```
