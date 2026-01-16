# n3gb-rs

Rust implementation of a hex-based spatial indexing for British National Grid.

Inspired by the work done by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb).

## Simple Overview

Creating a HexCell

1. point_to_row_col — Takes your input coordinates and finds which grid cell they fall into. Returns (row, col).
2. row_col_to_center — Takes the (row, col) and calculates the exact center of that cell. This is where the offset for odd rows gets applied. Returns (center_x, center_y).
3. generate_identifier — Takes the center coordinates of the HexCell and zoom level, packs it all into binary, and then encodes as Base64. The ID contains: version + center_x + center_y + zoom.

Reconstructing from ID

1. decode_hex_identifier — Decodes the Base64 string back into version, center_x, center_y, and zoom.
2. create_hexagon — Draws 6 vertices around the center at the radius for that zoom level.

## Lib currently has three main entry points

**1. Single Cells** - use `HexCell`

```rust
let point = Point::new(383640.0, 398260.0);
let cell = HexCell::from_bng(&point, 12)?;
println!("{}", cell.id);
let polygon = cell.to_polygon();
```

Example output:

```bash
Hex ID: AQAAAAAW3cpQAAAAABe831IMHg
Center: (383634, 398253.9062859271)
Row: 25548, Col: 21313
Polygon: POLYGON((383643.0 398259.1024383498,383634.0 398264.2985907725,383625.0 398259.1024383498,383625.0 398248.7101335044,383634.0 398243.5139810817,383643.0 398248.7101335044,383643.0 398259.1024383498))
```

**2. Cell Collections** - use `HexGrid`

```rust
let grid = HexGrid::builder()
    .zoom_level(12)
    .bng_extent(&(300000.0, 300000.0), &(350000.0, 350000.0))
    .build();

if let Some(cell) = grid.get_cell_at(&point) {
    println!("{}", cell.id);
}
```

**3. CSV to Hex Conversion** - use `CsvToHex` trait or `csv_to_hex_csv` function

Convert CSV files containing geometry (WKT or GeoJSON) to hex-indexed CSVs:

```rust
use n3gb_rs::{CsvToHex, CsvHexConfig, Crs, GeometryFormat};

let config = CsvHexConfig::new("geometry", 12)
    .exclude(vec!["Geo Point".into()])
    .crs(Crs::Wgs84)
    .with_hex_geometry(GeometryFormat::Wkt);

// Using trait method
"input.csv".to_hex_csv("output.csv", &config)?;

// Or using function
csv_to_hex_csv("input.csv", "output.csv", &config)?;
```

Supported geometry types:
- Point, MultiPoint
- LineString, MultiLineString
- Polygon, MultiPolygon
- GeometryCollection

Input formats: WKT (`POINT(...)`) or GeoJSON (`{"type":"Point",...}`)
