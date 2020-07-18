# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

# [Unreleased]
### Fixed
- Fixed potential panic if a benchmark took zero time.

## [1.0.0-alpha3] - 2020.07-06
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
[Unreleased]: https://github.com/bheisler/cargo-criterion/compare/1.0.0-alpha3...HEAD