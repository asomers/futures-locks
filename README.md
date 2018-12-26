# futures-locks

A library of [Futures]-aware locking primitives.  These locks can safely be used
in asynchronous environments like [Tokio].  When they block, they'll only block
a single task, not the entire reactor.

[![Build Status](https://travis-ci.org/asomers/futures-locks.svg?branch=master)](https://travis-ci.org/asomers/futures-locks)
[![CodeCov.io](https://codecov.io/gh/asomers/futures-locks/branch/master/graph/badge.svg)](https://codecov.io/gh/asomers/futures-locks)

[Futures]: https://github.com/rust-lang-nursery/futures-rs
[Tokio]: https:/tokio.rs

```toml
# Cargo.toml
[dependencies]
futures = "0.1.20"
futures-locks = "0.2"
```

# Usage

Generally, the provided primitives work much like their counterparts from the
standard library.  But instead of blocking until ready, they return Futures
which will become ready when the lock is acquired.  See the doc comments for
individual examples.

`futures-locks` requires Rust 1.31.0 or higher.

# License

`futures-locks` is primarily distributed under the terms of both the MIT license
and the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details
