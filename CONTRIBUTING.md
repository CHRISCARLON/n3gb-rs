# Contributing to n3gb-rs

Read this document if you want to contribute.

## Prerequisites

n3gb-rs depends on [PROJ](https://proj.org/) (via the `proj` crate) for
coordinate transformations. You'll need it installed system-wide:

- **macOS:** `brew install proj`
- **Debian/Ubuntu:** `sudo apt-get install libproj-dev`

A recent stable Rust toolchain is required (this crate uses the 2024 edition).
Install via [rustup](https://rustup.rs/).

## Getting started

```bash
git clone https://github.com/CHRISCARLON/n3gb-rs
cd n3gb-rs
cargo build
cargo test
```

## Before opening a pull request

Please make sure the following pass locally — these are the same checks CI runs:

```bash
cargo fmt --all -- --check    # formatting
cargo clippy --all-targets    # lints (warnings are denied in CI)
cargo test                    # tests
cargo doc --no-deps           # docs build cleanly
```

If you've changed dependencies, also run:

```bash
cargo deny check           
```

## Pull request guidelines

- Keep PRs focused on a single change where possible.
- Add or update tests for any behavioural change.

## Reporting bugs

Open an issue describing what you expected, what happened, and a minimal
reproduction (coordinates, zoom level, and the call that misbehaves are
especially helpful for spatial bugs).

## Licence

By contributing, you agree that your contributions will be licensed under the
Apache-2.0 licence that covers this project.
