## [0.1.1] - 2018-06-07
### Added
- Implemented `Borrow` and `BorrowMut` for `DivBuf` and `DivBufMut`

### Changed
- Futures should do nothing until polled.
  ([#2](https://github.com/asomers/futures-locks/issues/2))
- Fixed potential deadlocks when dropping Futures without first polling them.
  ([#1](https://github.com/asomers/futures-locks/issues/1))

### Fixed

### Removed
