## [Unreleased] - ReleaseDate
### Added
- Added methods for running critical code in its own task.  These methods
  require Tokio.
  ([#3](https://github.com/asomers/futures-locks/issues/3))
- Implemented `Future` for all future types, even when the wrapped type is
  unsized.

## [0.1.1] - 2018-06-07
### Added

### Changed
- Futures should do nothing until polled.
  ([#2](https://github.com/asomers/futures-locks/issues/2))
- Fixed potential deadlocks when dropping Futures without first polling them.
  ([#1](https://github.com/asomers/futures-locks/issues/1))

### Fixed

### Removed
