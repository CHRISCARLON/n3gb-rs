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

## CSV Conversion

Convert CSV files with geometry columns to hex-indexed CSVs.

I am playing with this as a way to "anonymise" senstive, real world data you want to share without revealing the exact coordinates.

```rust
use n3gb_rs::{CsvHexConfig, Crs, csv_to_hex_csv};

// From WKT/GeoJSON geometry column
let config = CsvHexConfig::new("geometry", 12)
    .crs(Crs::Wgs84);

// From coordinate columns
let config = CsvHexConfig::from_coords("Easting", "Northing", 12)
    .crs(Crs::Bng);

csv_to_hex_csv("input.csv", "output.csv", &config)?;
```
