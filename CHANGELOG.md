# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

# [Unreleased]
### Fixed
- Fixed problem where benchmarks that relied on dynamically linked libraries would fail
  in cargo-criterion but not in cargo bench.

### Added
- Added `--message-format=json` option, which prints JSON messages about the benchmarks to
  stdout, similar to other Cargo commands.

### Changed
- In order to accommodate the machine-readable output, all of cargo-criterion's other output
  is now printed to stderr. This matches Cargo's normal behavior. If benchmark targets print 
  anything to stdout, it will be redirected to stderr if `--message-format` is set, or will be 
  left on stderr if not.

## [1.0.0-alpha1] - 2020-06-29
### Added
- Initial version of cargo-criterion


[1.0.0-alpha1]: https://github.com/bheisler/cargo-criterion/compare/e5fa23b...1.0.0-alpha1
[Unreleased]: https://github.com/bheisler/cargo-criterion/compare/1.0.0-alpha1...HEAD