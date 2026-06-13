# Changelog

All notable changes to this project will be documented in this file.

I'll try keep it up to date.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),

## [Unreleased]

### Added
- `deny.toml` for dependency licence / advisory / source governance.
- `CONTRIBUTING.md` and this changelog.
- Basic lints (`unsafe_code = "forbid"`, clippy `all = "warn"`) in `Cargo.toml`.
- CI, crates.io, docs.rs and licence badges in the README.

### Changed
- Crate licence is now Apache-2.0 (was previously declared MIT, while the
  `LICENSE` file already contained the Apache-2.0 text).

## [0.2.1] - 2026-03-22

### Added
- Hex cell step calculations between hex cells.

[Unreleased]: https://github.com/CHRISCARLON/n3gb-rs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/CHRISCARLON/n3gb-rs/releases/tag/v0.2.1
