# n3gb-rs

Rust implemented of a hex-based spatial indexing for British National Grid.

Inspired by the work done by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb).

## Simple Overview

Creating a HexCell

1. point_to_hex — Takes your input coordinates and finds which grid cell they fall into. Returns (row, col).
2. hex_to_point — Takes the (row, col) and calculates the exact center of that cell. This is where the offset for odd rows gets applied. Returns (center_x, center_y).
3. generate_identifier — Takes the center coordinates of the HexCell and zoom level, packs it all into binary, and then encodes as Base64. The ID contains: version + center_x + center_y + zoom.

Reconstructing from ID

1. decode_hex_identifier — Decodes the Base64 string back into center_x, center_y, and zoom.
2. create_hexagon — Draws 6 vertices around the center at the radius for that zoom level.

How polygons maintain the offset structure?

1. They don't care nor need to know about it.
2. The offset is applied when hex_to_point calculates the centre.
3. The centre coordinates themselves contain the offset.

## Lib currently has two main entry points

**1. Single cells** - use `HexCell`

```rust
let cell = HexCell::from_bng(383640.0, 398260.0, 12)?; // BNG
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

**2. Cell collections** - use `HexGrid`

```rust
let grid = HexGrid::builder()
    .zoom_level(12)
    .extent(383500.0, 397800.0, 384000.0, 398500.0)
    .build();

if let Some(cell) = grid.get_cell_at(&point) {
    println!("{}", cell.id);
}
```
