# Architecture

THIS WAS CREATED BY CLAUDE AND REVIEWED BY CHRIS CARLON.

I will change this as and when things spring to mind.

A guided tour of how `n3gb-rs` works internally, and a cookbook for changing and
extending it. If you're new to the codebase, read this top to bottom once, then
keep section 9 ("How to extend it") nearby while you work.

## The one-paragraph mental model

A coordinate — either British National Grid (BNG / EPSG:27700) easting/northing,
or WGS84 lon/lat that gets **projected to BNG first** — is mapped to an integer
`(row, col)` on an offset hexagon lattice. The lattice spacing comes from per-zoom
constant tables. That `(row, col)` yields a hexagon **center**, and the center +
zoom level is encoded into a 19-byte, checksummed, URL-safe Base64 **id**. A
`HexCell` is one such cell; a `HexGrid` is a collected, spatially-indexed
`Vec<HexCell>`. Everything else in the crate is either (a) turning geometry into
cells, or (b) turning cells into output (Arrow / GeoParquet / CSV).

> **Important:** unlike H3, the n3gb index is **not hierarchical**. Each zoom level
> is an independent lattice — there is no built-in parent/child relationship
> between a cell at zoom 10 and one at zoom 11.

## 1. Module map

All modules are declared private in `lib.rs`; the public API is defined entirely
by the `pub use` re-export block (`src/lib.rs:206`). To change what users can see,
you edit that block.

```
lib.rs                  crate root — module declarations + public re-exports
├── cell.rs             HexCell (single hexagon)            → coord, error, geom, index, io
├── grid.rs             HexGrid + HexGridBuilder            → cell, coord, error, index, io
├── coord/              Crs, ConversionMethod, Coordinate trait, BNG transforms
│   ├── mod.rs
│   └── bng_transformations.rs
├── geom/               create_hexagon, parse_geometry      (leaf — no internal deps)
│   ├── mod.rs
│   ├── hexagon.rs
│   └── parse.rs
├── index/              row/col math, Base64 id encode/decode, constant tables
│   ├── mod.rs
│   ├── constants.rs
│   ├── indexing.rs
│   └── identifier.rs
├── dimensions.rs       HexagonDims — pure hexagon math      → error
├── error.rs            N3gbError enum + From conversions     (leaf)
└── io/                 Arrow / GeoParquet / CSV export
    ├── mod.rs
    ├── arrow.rs
    ├── parquet.rs
    └── csv.rs
```

**Dependency rule worth remembering:** `geom/` is a leaf module — it must not
depend on `cell` (that would be circular, since `cell` uses `geom`). Domain logic
that *creates* `HexCell`s lives in `cell.rs`, not in `geom/`.

## 2. The data model

### `HexCell` (`src/cell.rs:41`)

```rust
pub struct HexCell {
    pub id: String,          // URL-safe Base64 unique identifier
    pub center: Point<f64>,  // BNG coordinates of the hexagon center
    pub zoom_level: u8,      // 0–15
    pub row: i64,            // offset-lattice row
    pub col: i64,            // offset-lattice column
}
```

Key methods:

| Method | Where | What it does |
|--------|-------|--------------|
| `from_bng(coord, zoom)` | `cell.rs:173` | One cell from a BNG coordinate |
| `from_wgs84(coord, zoom, method)` | `cell.rs:203` | Projects WGS84 → BNG, then indexes |
| `from_hex_id(id)` | `cell.rs:76` | Reconstructs a cell from its id string |
| `from_geometry(geom, zoom, crs, method)` | `cell.rs:217` | Dispatches any `geo_types::Geometry` → `Vec<HexCell>` |
| `from_line_string_bng/_wgs84` | `cell.rs:92/148` | All unique cells a line passes through |
| `easting()` / `northing()` | `cell.rs:342/347` | Center X / Y |
| `to_polygon()` | `cell.rs:355` | Hexagon boundary as a `geo_types::Polygon` |
| `grid_distance(&other)` | `cell.rs:328` | Hex-step distance (same zoom required) |

### `HexGrid` (`src/grid.rs:51`)

```rust
pub struct HexGrid {
    cells: Vec<HexCell>,
    index: HashMap<(i64, i64), usize>,  // (row, col) → position in `cells`
    zoom_level: u8,
}
```

The `HashMap` is what makes `get_cell_at(point)` (`grid.rs:375`) an O(1) lookup
instead of a linear scan. Constructors come in BNG and WGS84 pairs:
`from_bng_extent` / `from_wgs84_extent`, `from_rect`, `from_bng_polygon` /
`from_wgs84_polygon`, `from_bng_multipolygon` / `from_wgs84_multipolygon`. Query
helpers: `len`, `is_empty`, `cells`, `iter`, `filter`, `to_polygons`. Both
`&HexGrid` and `HexGrid` implement `IntoIterator`.

### `HexGridBuilder` (`src/grid.rs:456`)

The chainable front door, and the most ergonomic way to build a grid:

```rust
let grid = HexGrid::builder()
    .zoom_level(12)
    .bng_extent(&(300_000.0, 300_000.0), &(350_000.0, 350_000.0))
    .build()?;
```

Setters mirror the constructors (`rect`, `bng_extent`, `wgs84_extent`,
`bng_polygon`, `wgs84_polygon`, `bng_multipolygon`, `wgs84_multipolygon`,
`conversion_method`). `build()` validates and produces the `HexGrid`.

## 3. The spatial pipeline (step by step)

This is the heart of the crate. Follow one coordinate from input to id.

**Step 0 — (optional) WGS84 → BNG.** `coord/bng_transformations.rs::convert_to_bng`
dispatches on `ConversionMethod`:

- `Ostn15` (**default**) — uses the `lonlat_bng` crate. The OSTN15 grid-shift data
  is embedded at compile time, so it's ~1 mm accurate with **no system
  dependencies**. This is why it's the default: a self-contained, reliable path.
- `Proj` — uses the system PROJ library via the `proj` crate. A `Proj` object is
  cached thread-locally (lazy init, once per thread). Accuracy is ~1 mm *with* the
  OSTN15 grid file installed, ~5 m (Helmert fallback) without. PROJ is a heavier,
  riskier build dependency — hence it's opt-in.

**Step 1 — BNG point → (row, col).** `index/indexing.rs::point_to_row_col`:

```text
dx = CELL_WIDTHS[zoom]
dy = 1.5 * CELL_RADIUS[zoom]
qx = (x - GRID_EXTENTS[0]) / dx
ry = (y - GRID_EXTENTS[1]) / dy
row = round(ry)
col = round(qx - (row mod 2))      // odd rows shift half a cell — offset coords
```

**Step 2 — (row, col) → center.** `row_col_to_center` is the exact inverse:

```text
x = GRID_EXTENTS[0] + col*dx + (row mod 2)*(dx/2)
y = GRID_EXTENTS[1] + row*dy
```

**Step 3 — center + zoom → id.** `index/identifier.rs::generate_hex_identifier`
packs a 19-byte buffer, then URL-safe Base64 (no padding):

| Bytes | Field |
|-------|-------|
| 0 | version (`IDENTIFIER_VERSION = 1`) |
| 1–8 | `easting × 1000` as big-endian `u64` |
| 9–16 | `northing × 1000` as big-endian `u64` |
| 17 | zoom level |
| 18 | checksum (wrapping sum of bytes 0–17) |

`SCALE_FACTOR = 1000` preserves 3 decimal places (millimetre precision).

**Step 4 — id → components (round trip).** `decode_hex_identifier` Base64-decodes,
checks length is 19, verifies the checksum, checks the version, then divides the
integers back by 1000.

### Worked example (Manchester-ish, zoom 10)

```text
WGS84 (lon -2.248, lat 53.481)
  → OSTN15 → BNG (easting ≈ 385500.123, northing ≈ 339874.456)
  → dx = CELL_WIDTHS[10] = 130.0, dy = 1.5 * 75.06 = 112.59
  → row = round(339874.456 / 112.59) = 3019
  → col = round(385500.123/130 - (3019 mod 2)) = 2964
  → center ≈ (385385, 340213)
  → id bytes [1, <385500123 be>, <339874456 be>, 10, checksum]
  → Base64 string
```

### Cube coordinates and distance

`offset_to_cube(row, col)` converts to cube coords (`q = col - row/2`, `r = row`,
`s = -q - r`, with `q + r + s = 0`). This is the bridge used by
`grid_distance`, which is Manhattan distance in cube space. Cells must share a
zoom level or you get `N3gbError::ZoomLevelMismatch`.

## 4. The constant tables (the tuning knobs)

`index/constants.rs` *is* the grid definition. Three length-16 arrays, indexed by
zoom:

- `GRID_EXTENTS = [0.0, 0.0, 750000.0, 1350000.0]` — `[min_x, min_y, max_x, max_y]`,
  the BNG bounds of Great Britain.
- `CELL_RADIUS[16]` — center-to-vertex distance per zoom (zoom 0 ≈ country scale,
  zoom 15 ≈ 0.58 m).
- `CELL_WIDTHS[16]` — hexagon width per zoom (zoom 15 ≈ 1 m).
- `MAX_ZOOM_LEVEL = 15`, `IDENTIFIER_VERSION = 1`.

The relationship the pipeline relies on: `dy = 1.5 * radius`, `dx = width`. Editing
a row in these arrays changes cell sizes at that zoom — and therefore changes
every id produced at that zoom (see the versioning note in §9).

## 5. Geometry helpers (`geom/`)

- `hexagon.rs::create_hexagon(center, size)` — builds a **pointy-top** hexagon:
  vertices at angles `30° + 60°·i`, 7 coordinates total (the last repeats the
  first to close the ring). This backs `HexCell::to_polygon()`.
- `parse.rs::parse_geometry(s)` — auto-detects format: a leading `{` means
  **GeoJSON**, otherwise **WKT**. Both funnel through `geo_types::Geometry`. A
  GeoJSON `FeatureCollection` is rejected (single geometries/features only).

## 6. Dimensions (`dimensions.rs`)

A standalone hexagon-math utility, *not* part of the indexing pipeline.
`HexagonDims` holds side, circumradius, apothem, across-corners, across-flats,
perimeter, area. Construct it from any one measurement: `from_side`,
`from_circumradius`, `from_apothem`, `from_across_flats`, `from_across_corners`,
`from_area`, plus `bounding_box`. All validate positive inputs and return
`N3gbError::InvalidDimension` otherwise.

## 7. IO layer (`io/`)

The export API is built from two **blanket traits** implemented for any
`T: AsRef<[HexCell]>`. That single bound means they work uniformly on `&HexCell`,
`Vec<HexCell>`, and `&[HexCell]` — a lone cell is handled via
`std::slice::from_ref(self)`.

- `HexCellsToArrow` (`io/arrow.rs:20`): `to_arrow_points()`, `to_arrow_polygons()`
  (parallelised with rayon), `to_record_batch()`. The record batch has 7 columns:
  `id`, `zoom_level`, `row`, `col`, `easting`, `northing`, `geometry`. CRS metadata
  is EPSG:27700.
- `HexCellsToGeoParquet` (`io/parquet.rs:47`): `to_geoparquet(path)` builds the
  record batch then calls `write_geoparquet`, which uses WKB geometry encoding and
  appends GeoParquet key-value metadata.

CSV (`io/csv.rs`) is configuration-driven via `CsvHexConfig` (builders
`new` / `from_coords`, plus `.exclude()`, `.crs()`, `.with_hex_geometry()`,
`.conversion_method()`, `.hex_density()`). Input can be a single geometry column
(`CoordinateSource::GeometryColumn`) or separate X/Y columns
(`CoordinateColumns`); optional hex geometry output is WKT or GeoJSON
(`GeometryFormat`). `csv_to_hex_csv` streams input→output and can aggregate to a
per-hex density count.

## 8. Error model (`error.rs`)

`N3gbError` is the single error type (11 variants: invalid id length, bad
checksum, unsupported version, invalid zoom/dimension, base64 decode, projection,
io, csv, geometry parse, zoom mismatch). It implements `Display` + `std::error::
Error`, and has `From` conversions for `std::io::Error`, `csv::Error`,
`arrow_schema::ArrowError`, and `parquet::errors::ParquetError` — which is what
lets `?` propagate cleanly across the IO boundary. Convention: every fallible
public function returns `Result<T, N3gbError>`.

## 9. How to extend it (cookbook)

Each recipe names the file(s) to touch and the existing pattern to copy.

**Add a HexCell query method** (e.g. `neighbors()`)
→ Add an `impl HexCell` method in `cell.rs`. Use the cube-coordinate helpers from
`index` (`offset_to_cube`) for neighbour math. Inherent methods are public
automatically — no re-export needed.

**Add a constructor from a new geometry type**
→ Follow `from_line_string_bng` in `cell.rs`. If it accepts WGS84 input, add the
matching `convert_*_to_bng` helper in `coord/bng_transformations.rs` and provide a
`_wgs84` variant alongside the `_bng` one.

**Add a new grid source**
→ Add a `from_*` constructor on `HexGrid` (`grid.rs`) that produces a
`Vec<HexCell>` and feeds the internal index-building path. Then add the matching
setter on `HexGridBuilder` and wire it into `build()`.

**Add or retune a zoom level**
→ Edit the arrays in `index/constants.rs`. Keep all three length-16 arrays in sync;
bump `MAX_ZOOM_LEVEL` if you add entries. ⚠️ This changes the ids produced at the
affected zooms — coordinate with the versioning note below.

**Add a new output format** (e.g. GeoJSON export)
→ Add a trait in `io/` over `T: AsRef<[HexCell]>`, mirroring `HexCellsToArrow`,
then re-export it from `lib.rs`.

**Add a new error case**
→ Add a variant to `N3gbError`, a matching arm in its `Display` impl, and a `From`
impl if it wraps a foreign error type.

**Change the id binary format**
→ Bump `IDENTIFIER_VERSION` in `constants.rs` and branch on the version byte in
`decode_hex_identifier` so existing ids still decode.

**Every change:**
- Re-export any new public type/function from the `pub use` block in `lib.rs`
  (`src/lib.rs:206`) — that block defines the entire public surface.
- Add tests as `#[cfg(test)] mod tests` **in the same file** (this repo keeps
  tests inline; there is no `tests/` directory).
- If it's a user-facing entry point, add an example under `examples/`.

## 10. Build, test, run

```bash
cargo test                       # inline unit tests
cargo run --example cell_basics  # 5 examples — see below
cargo clippy --all-targets       # lints (Cargo.toml sets clippy all = warn)
cargo fmt                        # formatting
```

PROJ system library is needed **only** for `ConversionMethod::Proj` and the
`check_proj` example (`brew install proj` / `apt-get install libproj-dev`). The
default `Ostn15` path needs nothing extra.

**CI today:** `.github/workflows/ci.yml` runs `cargo audit` + `cargo test`;
`.github/workflows/security.yml` runs cargo-deny (push/PR + daily). There are no
`fmt` / `clippy` / `doc` CI jobs yet — a known gap.

## 11. Suggested reading order

Read each example with its underlying module open alongside:

1. `examples/cell_basics.rs` — `HexCell` creation, ids, round-trips → `cell.rs`, `index/`
2. `examples/grid_distance.rs` — spatial relationships, `grid_distance` → `index/indexing.rs`
3. `examples/grid_from_polygon.rs` — the builder + polygon clipping → `grid.rs`
4. `examples/line_coverage.rs` — line/geometry coverage → `cell.rs`
5. `examples/check_proj.rs` — WGS84 conversion backends → `coord/`

## 12. Known future work (from `TODO.md`)

- Grid traversal (path/cells-between) beyond the single-cell `grid_distance`.
- Geometry validation / bounds checking on input.
- Hierarchical parent/child navigation — non-trivial because the index is not
  hierarchical; would mean re-indexing the same center at another zoom.
