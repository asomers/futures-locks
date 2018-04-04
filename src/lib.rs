// vim: tw=80

//!  A library of [`Futures`]-aware locking primitives.  These locks can safely be
//!  used in asynchronous environments like [`Tokio`].  When they block, they'll
//!  only block a single task, not the entire reactor.
//!
//! [`Futures`]: https://github.com/rust-lang-nursery/futures-rs
//! [`Tokio`]: https:/tokio.rs

extern crate futures;

mod mutex;
mod rwlock;

pub use mutex::{Mutex, MutexFut, MutexGuard};
pub use rwlock::{RwLock, RwLockReadFut, RwLockWriteFut,
                 RwLockReadGuard, RwLockWriteGuard};
