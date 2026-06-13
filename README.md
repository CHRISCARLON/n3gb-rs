# n3gb-rs

[![CI](https://github.com/CHRISCARLON/n3gb-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/CHRISCARLON/n3gb-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/n3gb-rs.svg)](https://crates.io/crates/n3gb-rs)
[![docs.rs](https://docs.rs/n3gb-rs/badge.svg)](https://docs.rs/n3gb-rs)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

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
