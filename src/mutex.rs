// vim: tw=80

use futures::{Async, Future, Poll};
use futures::sync::oneshot;
use std::cell::UnsafeCell;
use std::clone::Clone;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync;

/// An RAII mutex guard, much like `std::sync::MutexGuard`.  The wrapped data
/// can be accessed via its `Deref` and `DerefMut` implementations.
pub struct MutexGuard<T: ?Sized> {
    mutex: Mutex<T>
}

impl<T: ?Sized> Drop for MutexGuard<T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

/// A `Future` representation a pending `Mutex` acquisition.
pub struct MutexFut<T: ?Sized> {
    receiver: Option<oneshot::Receiver<()>>,
    mutex: Mutex<T>,
}

impl<T: ?Sized> MutexFut<T> {
    fn new(rx: Option<oneshot::Receiver<()>>, mutex: Mutex<T>) -> Self {
        MutexFut{receiver: rx, mutex}
    }
}

impl<T: ?Sized> Deref for MutexGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {&*self.mutex.inner.data.get()}
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {&mut *self.mutex.inner.data.get()}
    }
}

impl<T> Future for MutexFut<T> {
    type Item = MutexGuard<T>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.receiver.is_none() {
            Ok(Async::Ready(MutexGuard{mutex: self.mutex.clone()}))
        } else {
            match self.receiver.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => panic!(
                    "receiver.poll returned unanticipated error {:?}", e),
                Ok(Async::Ready(_)) => {
                    Ok(Async::Ready(MutexGuard{mutex: self.mutex.clone()}))
                }
            }
        }
    }
}

#[derive(Debug)]
struct MutexData {
    owned: bool,
    // FIFO queue of waiting tasks.
    waiters: VecDeque<oneshot::Sender<()>>,
}

#[derive(Debug)]
struct Inner<T: ?Sized> {
    mutex: sync::Mutex<MutexData>,
    data: UnsafeCell<T>,
}

/// A Futures-aware Mutex.
///
/// `std::sync::Mutex` cannot be used in an asynchronous environment like Tokio,
/// because a mutex acquisition can block an entire reactor.  This class can be
/// used instead.  It functions much like `std::sync::Mutex`.  Unlike that
/// class, it also has a builtin `Arc`, making it accessible from multiple
/// threads.  It's also safe to `clone`.  Also unlike `std::sync::Mutex`, this
/// class does not detect lock poisoning.
///
/// # Examples
///
/// ```
/// # extern crate futures;
/// # extern crate futures_locks;
/// # use futures_locks::*;
/// # use futures::executor::{Spawn, spawn};
/// # use futures::Future;
/// # fn main() {
/// let mtx = Mutex::<u32>::new(0);
/// let fut = mtx.lock().map(|mut guard| { *guard += 5; });
/// spawn(fut).wait_future();
/// assert_eq!(mtx.try_unwrap().unwrap(), 5);
/// # }
/// ```
#[derive(Debug)]
pub struct Mutex<T: ?Sized> {
    inner: sync::Arc<Inner<T>>,
}

impl<T: ?Sized> Clone for Mutex<T> {
    fn clone(&self) -> Mutex<T> {
        Mutex { inner: self.inner.clone()}
    }
}

impl<T> Mutex<T> {
    /// Create a new `Mutex` in the unlocked state.
    pub fn new(t: T) -> Mutex<T> {
        let mutex_data = MutexData {
            owned: false,
            waiters: VecDeque::new(),
        };
        let inner = Inner {
            mutex: sync::Mutex::new(mutex_data),
            data: UnsafeCell::new(t)
        };
        Mutex { inner: sync::Arc::new(inner)}
    }

    /// Consumes the `Mutex` and returns the wrapped data.  If the `Mutex` still
    /// has multiple references (not necessarily locked), returns a copy of
    /// `self` instead.
    pub fn try_unwrap(self) -> Result<T, Mutex<T>> {
        match sync::Arc::try_unwrap(self.inner) {
            Ok(inner) => Ok({
                // `unsafe` is no longer needed as of somewhere around 1.25.0.
                // https://github.com/rust-lang/rust/issues/35067
                #[allow(unused_unsafe)]
                unsafe { inner.data.into_inner() }
            }),
            Err(arc) => Err(Mutex {inner: arc})
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires a `Mutex`, blocking the task in the meantime.  When the
    /// returned `Future` is ready, this task will have sole access to the
    /// protected data.
    pub fn lock(&self) -> MutexFut<T> {
        let mut mtx_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        if mtx_data.owned {
            let (tx, rx) = oneshot::channel::<()>();
            mtx_data.waiters.push_back(tx);
            return MutexFut::new(Some(rx), self.clone());
        } else {
            mtx_data.owned = true;
            return MutexFut::new(None, self.clone())
        }
    }

    /// Release the `Mutex`
    fn unlock(&self) {
        let mut mtx_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        assert!(mtx_data.owned);
        if let Some(tx) = mtx_data.waiters.pop_back() {
            // Send ownership to the waiter
            tx.send(()).expect("Sender::send");
        } else {
            // Relinquish ownership
            mtx_data.owned = false;
        }
    }
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
