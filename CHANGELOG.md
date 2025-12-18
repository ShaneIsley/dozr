# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2025-12-18

### Added
- GitHub Actions CI workflow for automated build and test
- MIT LICENSE file in repository root
- Runnable examples in `examples/` directory (`cargo run --example <name>`)
- Criterion benchmarks in `benches/` directory (`cargo bench`)

### Changed
- Reorganized repository: moved helper scripts to `scripts/` directory
- Made main `dozr` binary explicit in Cargo.toml for clarity

## [0.4.0] - 2025-01-15

### Added
- Uniform distribution support (`uniform` / `u` command)
- Triangular distribution support (`triangular` / `t` command)
- Subcommand-based CLI architecture with short aliases
- Testing documentation (`TEST.md`)

### Changed
- Refactored CLI to use subcommands instead of flags
- Encapsulated WaitCondition creation in CLI module
- Improved code organization and reduced code smells

### Removed
- Weibull distribution (replaced by more commonly used distributions)

## [0.3.0] - 2025-01-10

### Added
- Normal distribution support (`normal` / `n` command)
- Exponential distribution support (`exponential` / `e` command)
- Log-normal distribution support (`log-normal` / `ln` command)
- Pareto distribution support (`pareto` / `par` command)
- Adaptive verbose mode that adjusts update frequency based on remaining time
- Comprehensive test suite with unit and integration tests

### Fixed
- Verbose output timing accuracy improvements
- Adaptive verbose output alignment to time markers

## [0.2.0] - 2025-01-05

### Added
- `--until` / `at` command to wait until a specific time of day
- `--probability` flag for probabilistic execution
- Time alignment with `align` command (snap to intervals)
- Customizable verbose update period
- Jitter support with `--jitter` flag

### Changed
- Refactored to library-first architecture
- Decoupled verbose output from wait logic
- Improved CLI argument validation using clap groups

## [0.1.0] - 2025-01-01

### Added
- Initial release
- Fixed duration waits with human-readable time parsing
- `--verbose` flag with ETA countdown
- Adaptive ETA display with human-readable formatting
- Support for duration units: seconds, minutes, hours, days, milliseconds

[Unreleased]: https://github.com/ShaneIsley/dozr/compare/v0.4.1...HEAD
[0.4.1]: https://github.com/ShaneIsley/dozr/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/ShaneIsley/dozr/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ShaneIsley/dozr/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ShaneIsley/dozr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ShaneIsley/dozr/releases/tag/v0.1.0
