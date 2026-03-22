# n3gb-rs

Rust implementation of hex-based spatial indexing for British National Grid.

Inspired by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb) and [h3o](https://github.com/HydroniumLabs/h3o).

## Documentation

Full API reference and guides: **[chriscarlon.github.io/n3gb-rs-docs](https://chriscarlon.github.io/n3gb-rs-docs/)**

## Quick Start

```rust
use n3gb_rs::{HexCell, HexGrid};
use geo_types::Point;

// Single cell from BNG coordinates
let point = Point::new(383640.0, 398260.0);
let cell = HexCell::from_bng(&point, 12)?;

// Grid over a bounding box
let grid = HexGrid::from_bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0), 12)?;

// Export to GeoParquet
grid.to_geoparquet("output.parquet")?;
```
