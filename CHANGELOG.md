# Changelog

All notable changes to this project will be documented in this file.

I'll try keep it up to date.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),

## [Unreleased]

### Changed
- `HexCell::from_geometry` is now documented as the general-purpose dispatcher
  for runtime/parsed geometry, with guidance to prefer the typed constructors
  (`from_bng`, `from_wgs84`, `from_line_string_*`) when the input type is known,
  plus a note that it always returns a `Vec`. Added cross-references back to
  `from_geometry` on the typed point constructors.
- `cell_basics` example expanded to demonstrate every constructor (`from_bng`,
  `from_wgs84`, `from_hex_id`, `from_line_string_bng`, `from_geometry`).
- `line_coverage` example now defines the route once in BNG and derives the
  WGS84 line by reprojection, then reports how closely the two entry points
  agree on the covered cells.
- Clarified the cell dedup in `from_line_string_bng` by binding the
  `HashSet::insert` result to a named `is_new_cell` (internal readability only;
  no behaviour change).
- Renamed the `zoom` parameter to `zoom_level` on the `HexCell` constructors
  (`from_bng`, `from_wgs84`, `from_geometry`, `from_line_string_bng`,
  `from_line_string_wgs84`) so it matches the `HexCell::zoom_level` field and the
  `HexGrid` / `CsvHexConfig` constructors. Parameters are positional, so this is
  not a breaking change for callers.
- Crate-level "Cell inspection functions" table now distinguishes public fields
  (`cell.id`, `cell.center`, `cell.zoom_level`, `cell.row`, `cell.col`) from
  methods (`cell.easting()`, `cell.northing()`, `cell.to_polygon()`), instead of
  showing both with method-call notation.

### Fixed
- Removed the crate-level "Coordinate transformation functions" documentation
  table, which listed `wgs84_to_bng`, `wgs84_line_to_bng`, `wgs84_polygon_to_bng`
  and `wgs84_multipolygon_to_bng` as public functions. They are internal
  (`pub(crate)`) and never re-exported, so the documented API did not exist.
  WGS84 reprojection is reached via the `from_wgs84*` constructors or
  `Crs::Wgs84`.
- Corrected the documented default for `conversion_method` from
  `ConversionMethod::Proj` to `ConversionMethod::Ostn15` (the actual default) on
  both `CsvHexConfig::conversion_method` and `HexGridBuilder::conversion_method`.
- Fixed a typo ("Remeber" → "Remember") in the `HexGridBuilder` docs.

## [0.2.2] - 2026-06-13

### Added
- Comprehensive rustdoc sections (`# Arguments`, `# Returns`, `# Errors`) across
  the public API.
- `arrow_export` example demonstrating the `HexCellsToArrow` trait across
  `Vec<HexCell>`, slices, a single cell, and `HexGrid`.
- Optional `ostn15` feature (enabled by default) gating the `lonlat_bng` OSTN15
  conversion backend.
- `deny.toml` for dependency licence / advisory / source governance.
- `CONTRIBUTING.md` and this changelog.
- Basic lints (`unsafe_code = "forbid"`, clippy `all = "warn"`) in `Cargo.toml`.
- CI, crates.io, docs.rs and licence badges in the README.

### Changed
- Crate licence is now Apache-2.0 (was previously declared MIT, while the
  `LICENSE` file already contained the Apache-2.0 text).

### Fixed
- docs.rs build failure: `lonlat_bng`'s build script runs `cbindgen`, which needs
  network access the docs.rs sandbox forbids. `lonlat_bng` is now an optional
  dependency behind the default `ostn15` feature, and docs.rs is configured with
  `no-default-features` so the documentation builds.

[Unreleased]: https://github.com/CHRISCARLON/n3gb-rs/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/CHRISCARLON/n3gb-rs/compare/v0.2.1...v0.2.2
