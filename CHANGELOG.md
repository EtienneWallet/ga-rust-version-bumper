# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-03-06

### Added

- **Cargo.lock version syncing**: When `Cargo.lock` is tracked in git, local package versions are automatically updated alongside `Cargo.toml` during bumps, stable releases, and dev branch advancement
- `is_file_tracked()` utility in `git_ops` to check if a file exists in the HEAD tree
- New `lockfile_ops` module with `update_lockfile_version()` — identifies local packages by absence of `source` field and updates matching versions
- 6 new unit tests covering lockfile update logic and file tracking detection

### Fixed

- `Cargo.lock` no longer becomes out of sync after version bumps when tracked in the repository

## [0.1.0] - 2026-03-05

### Added

- **Initial release**: Rust-based GitHub Action for target-based SemVer bumping in Rust workspaces
  - Conventional Commits parsing (feat, fix, refactor, breaking changes)
  - Target-based versioning with `-devN` progress counters
  - Automatic stable release flow on `main` branch
  - Dev branch advancement after stable releases
  - Composite GitHub Action with 4 inputs and 5 outputs
  - Full test coverage: 43 unit tests + 25 integration tests
  - Example Cargo.toml formats and commit message reference

### Fixed

- Tag pushing now scoped to the specific new tag instead of all tags (`refs/tags/*`), reducing unnecessary network traffic
- Dev branch advancement now re-reads the Cargo.toml version location after checkout to handle structure differences between main and dev branches
- Fixed unsafe block in test environment variable handling (not needed on Rust 2021 edition)

### Documentation

- Comprehensive README with philosophy, quick start, inputs/outputs, bump rules, and troubleshooting
- Example Cargo.toml with both workspace and package formats
- Commit message reference table with expected version outcomes
- CI workflow that runs unit tests and validates action behavior

### Performance

- Cached Rust builds in CI to complete in under 5 seconds
- Release binary compiled with `--release` flag for optimal performance

### Quality

- Loop prevention: automatically skips processing on version-bump commits to avoid CI loops
- Dry-run mode for safe testing of version bump logic
- All error paths report clear, actionable error messages
- TOML formatting preserved on write operations

[0.2.0]: https://github.com/EtienneWallet/ga-rust-version-bumper/releases/tag/v0.2.0
[0.1.0]: https://github.com/EtienneWallet/ga-rust-version-bumper/releases/tag/v0.1.0
