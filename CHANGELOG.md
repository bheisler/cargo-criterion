# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

# [Unreleased]
### Added
- New message output format for [OpenMetrics](https://openmetrics.io).

## [1.1.0] - 2021-07-28
### Fixed
- Fixed wrong exit code being returned when a panic occurs outside of the function being benchmarked. 
- MacOS/Windows: Fix connection issue that manifested itself in a few different ways.
- Use new version of plotters. No new features but it fixes a bug that caused criterion to
  hang indefinitely.

### Added
- Load configuration options 'criterion.toml' if 'Criterion.toml' isn't available.

## [1.0.1] - 2021-01-24
### Fixed
- Changed opacity of the violin plots to full.
- Fixed violin chart X axis not starting at zero in the plotters backend.
- Fixed panic in the history report code.

## [1.0.0] - 2020-07-18
### Fixed
- Fixed potential panic if a benchmark took zero time.
- cargo-criterion now calls `cargo metadata` to find the path to the target directory. This fixes
  the location of the target directory in workspaces.

## Added
- Added a report showing the historical performance of a benchmark.

## [1.0.0-alpha3] - 2020-07-06
### Added
- The criterion.toml file can now be used to configure the colors used for the generated plots.

## [1.0.0-alpha2] - 2020-07-05
### Added
- Initial version of cargo-criterion
### Fixed
- Fixed problem where benchmarks that relied on dynamically linked libraries would fail
  in cargo-criterion but not in cargo bench.
- Sort the benchmark targets before running them. This should ensure a stable execution order
  for all benchmarks.

### Added
- Added `--message-format=json` option, which prints JSON messages about the benchmarks to
  stdout, similar to other Cargo commands.

### Changed
- In order to accommodate the machine-readable output, all of cargo-criterion's other output
  is now printed to stderr. This matches Cargo's normal behavior. If benchmark targets print 
  anything to stdout, it will be redirected to stderr if `--message-format` is set, or will be 
  left on stderr if not.
- Heavy internal refactoring of plot generation code. There may be some bugs.

## [1.0.0-alpha1] - 2020-06-29
### Added
- Initial version of cargo-criterion


[1.0.0-alpha1]: https://github.com/bheisler/cargo-criterion/compare/e5fa23b...1.0.0-alpha1
[1.0.0-alpha2]: https://github.com/bheisler/cargo-criterion/compare/1.0.0-alpha1...1.0.0-alpha2
[1.0.0-alpha3]: https://github.com/bheisler/cargo-criterion/compare/1.0.0-alpha2...1.0.0-alpha3
[1.0.0]: https://github.com/bheisler/cargo-criterion/compare/1.0.0-alpha3...1.0.0
[1.0.1]: https://github.com/bheisler/cargo-criterion/compare/1.0.0-alpha3...1.0.1
[1.0.1]: https://github.com/bheisler/cargo-criterion/compare/1.0.1...1.1.0
[Unreleased]: https://github.com/bheisler/cargo-criterion/compare/1.1.0...HEAD
