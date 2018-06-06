## [Unreleased] - ReleaseDate
### Added
- Implemented `Borrow` and `BorrowMut` for `DivBuf` and `DivBufMut`

### Changed
- Fixed potential deadlocks when dropping Futures without first polling them.
  ([#1](https://github.com/asomers/futures-locks/issues/1))

### Fixed

### Removed
