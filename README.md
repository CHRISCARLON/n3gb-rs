# n3gb-rs

Rust implemented of a hex-based spatial indexing for British National Grid.

Inspired by the work done by [GDS NUAR n3gb](https://github.com/national-underground-asset-register/n3gb).

## Lib currently has two main entry points

**1. Single cells** - use `HexCell`

```rust
let cell = HexCell::from_wgs84(-2.248, 53.481, 12)?;  // lon/lat
let cell = HexCell::from_bng(383640.0, 398260.0, 12)?; // BNG
println!("{}", cell.id);
let polygon = cell.to_polygon();
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
