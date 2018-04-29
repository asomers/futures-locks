// vim: tw=80

use futures::{Async, Future, Poll};
use futures::sync::oneshot;
use std::cell::UnsafeCell;
use std::clone::Clone;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync;

/// An RAII guard, much like `std::sync::RwLockReadGuard`.  The wrapped data can
/// be accessed via its `Deref` implementation.
pub struct RwLockReadGuard<T: ?Sized> {
    rwlock: RwLock<T>
}

impl<T: ?Sized> Deref for RwLockReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {&*self.rwlock.inner.data.get()}
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<T> {
    fn drop(&mut self) {
        self.rwlock.unlock_reader();
    }
}

/// An RAII guard, much like `std::sync::RwLockWriteGuard`.  The wrapped data
/// can be accessed via its `Deref`  and `DerefMut` implementations.
pub struct RwLockWriteGuard<T: ?Sized> {
    rwlock: RwLock<T>
}

impl<T: ?Sized> Deref for RwLockWriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {&*self.rwlock.inner.data.get()}
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {&mut *self.rwlock.inner.data.get()}
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<T> {
    fn drop(&mut self) {
        self.rwlock.unlock_writer();
    }
}

/// A `Future` representation a pending `RwLock` shared acquisition.
pub struct RwLockReadFut<T: ?Sized> {
    receiver: Option<oneshot::Receiver<()>>,
    rwlock: RwLock<T>,
}

impl<T: ?Sized> RwLockReadFut<T> {
    fn new(rx: Option<oneshot::Receiver<()>>, rwlock: RwLock<T>) -> Self {
        RwLockReadFut{receiver: rx, rwlock}
    }
}

impl<T> Future for RwLockReadFut<T> {
    type Item = RwLockReadGuard<T>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.receiver.is_none() {
            Ok(Async::Ready(RwLockReadGuard{rwlock: self.rwlock.clone()}))
        } else {
            match self.receiver.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => panic!(
                    "receiver.poll returned unanticipated error {:?}", e),
                Ok(Async::Ready(_)) => {
                    Ok(Async::Ready(RwLockReadGuard{rwlock: self.rwlock.clone()}))
                }
            }
        }
    }
}

/// A `Future` representation a pending `RwLock` exclusive acquisition.
pub struct RwLockWriteFut<T: ?Sized> {
    receiver: Option<oneshot::Receiver<()>>,
    rwlock: RwLock<T>,
}

impl<T: ?Sized> RwLockWriteFut<T> {
    fn new(rx: Option<oneshot::Receiver<()>>, rwlock: RwLock<T>) -> Self {
        RwLockWriteFut{receiver: rx, rwlock}
    }
}

impl<T> Future for RwLockWriteFut<T> {
    type Item = RwLockWriteGuard<T>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.receiver.is_none() {
            Ok(Async::Ready(RwLockWriteGuard{rwlock: self.rwlock.clone()}))
        } else {
            match self.receiver.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => panic!(
                    "receiver.poll returned unanticipated error {:?}", e),
                Ok(Async::Ready(_)) => {
                    Ok(Async::Ready(RwLockWriteGuard{rwlock: self.rwlock.clone()}))
                }
            }
        }
    }
}

#[derive(Debug)]
struct RwLockData {
    /// True iff the `RwLock` is currently exclusively owned
    exclusive: bool,

    /// The number of tasks that currently have shared ownership of the RwLock
    num_readers: u32,

    // FIFO queue of waiting readers
    read_waiters: VecDeque<oneshot::Sender<()>>,

    // FIFO queue of waiting writers
    write_waiters: VecDeque<oneshot::Sender<()>>,
}

#[derive(Debug)]
struct Inner<T: ?Sized> {
    mutex: sync::Mutex<RwLockData>,
    data: UnsafeCell<T>,
}

/// A Futures-aware RwLock.
///
/// `std::sync::RwLock` cannot be used in an asynchronous environment like
/// Tokio, because an acquisition can block an entire reactor.  This class can
/// be used instead.  It functions much like `std::sync::RwLock`.  Unlike that
/// class, it also has a builtin `Arc`, making it accessible from multiple
/// threads.  It's also safe to `clone`.  Also unlike `std::sync::RwLock`, this
/// class does not detect lock poisoning.
#[derive(Debug)]
pub struct RwLock<T: ?Sized> {
    inner: sync::Arc<Inner<T>>,
}

impl<T: ?Sized> Clone for RwLock<T> {
    fn clone(&self) -> RwLock<T> {
        RwLock { inner: self.inner.clone()}
    }
}

impl<T> RwLock<T> {
    /// Create a new `RwLock` in the unlocked state.
    pub fn new(t: T) -> RwLock<T> {
        let lock_data = RwLockData {
            exclusive: false,
            num_readers: 0,
            read_waiters: VecDeque::new(),
            write_waiters: VecDeque::new(),
        };
        let inner = Inner {
            mutex: sync::Mutex::new(lock_data),
            data: UnsafeCell::new(t)
        };
        RwLock { inner: sync::Arc::new(inner)}
    }

    /// Consumes the `RwLock` and returns the wrapped data.  If the `RwLock`
    /// still has multiple references (not necessarily locked), returns a copy
    /// of `self` instead.
    pub fn try_unwrap(self) -> Result<T, RwLock<T>> {
        match sync::Arc::try_unwrap(self.inner) {
            Ok(inner) => Ok({
                // `unsafe` is no longer needed as of somewhere around 1.25.0.
                // https://github.com/rust-lang/rust/issues/35067
                #[allow(unused_unsafe)]
                unsafe { inner.data.into_inner() }
            }),
            Err(arc) => Err(RwLock {inner: arc})
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Returns a reference to the underlying data, if there are no other
    /// clones of the `RwLock`.
    ///
    /// Since this call borrows the `RwLock` mutably, no actual locking takes
    /// place -- the mutable borrow statically guarantees no locks exist.
    /// However, if the `RwLock` has already been cloned, then `None` will be
    /// returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate futures_locks;
    /// # use futures_locks::*;
    /// # fn main() {
    /// let mut lock = RwLock::<u32>::new(0);
    /// *lock.get_mut().unwrap() += 5;
    /// assert_eq!(lock.try_unwrap().unwrap(), 5);
    /// # }
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if let Some(inner) = sync::Arc::get_mut(&mut self.inner) {
            let lock_data = inner.mutex.get_mut().unwrap();
            let data = unsafe { inner.data.get().as_mut() }.unwrap();
            debug_assert!(!lock_data.exclusive);
            debug_assert_eq!(lock_data.num_readers, 0);
            Some(data)
        } else {
            None
        }
    }

    /// Acquire the `RwLock` nonexclusively, read-only, blocking the task in the
    /// meantime.
    ///
    /// When the returned `Future` is ready, then this task will have read-only
    /// access to the protected data.
    ///
    /// # Examples
    /// ```
    /// # extern crate futures;
    /// # extern crate futures_locks;
    /// # use futures_locks::*;
    /// # use futures::executor::{Spawn, spawn};
    /// # use futures::Future;
    /// # fn main() {
    /// let rwlock = RwLock::<u32>::new(42);
    /// let fut = rwlock.read().map(|mut guard| { *guard });
    /// assert_eq!(spawn(fut).wait_future(), Ok(42));
    /// # }
    ///
    /// ```
    pub fn read(&self) -> RwLockReadFut<T> {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        if lock_data.exclusive {
            let (tx, rx) = oneshot::channel::<()>();
            lock_data.read_waiters.push_back(tx);
            return RwLockReadFut::new(Some(rx), self.clone());
        } else {
            lock_data.num_readers += 1;
            return RwLockReadFut::new(None, self.clone())
        }
    }

    /// Acquire the `RwLock` exclusively, read-write, blocking the task in the
    /// meantime.
    ///
    /// When the returned `Future` is ready, then this task will have read-write
    /// access to the protected data.
    ///
    /// # Examples
    /// ```
    /// # extern crate futures;
    /// # extern crate futures_locks;
    /// # use futures_locks::*;
    /// # use futures::executor::{Spawn, spawn};
    /// # use futures::Future;
    /// # fn main() {
    /// let rwlock = RwLock::<u32>::new(42);
    /// let fut = rwlock.write().map(|mut guard| { *guard = 5;});
    /// spawn(fut).wait_future().expect("spawn");
    /// assert_eq!(rwlock.try_unwrap().unwrap(), 5);
    /// # }
    ///
    /// ```
    pub fn write(&self) -> RwLockWriteFut<T> {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        if lock_data.exclusive || lock_data.num_readers > 0 {
            let (tx, rx) = oneshot::channel::<()>();
            lock_data.write_waiters.push_back(tx);
            return RwLockWriteFut::new(Some(rx), self.clone());
        } else {
            lock_data.exclusive = true;
            return RwLockWriteFut::new(None, self.clone())
        }
    }

    /// Release a shared lock of an `RwLock`.
    fn unlock_reader(&self) {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        assert!(lock_data.num_readers > 0);
        assert!(!lock_data.exclusive);
        if lock_data.num_readers == 1 {
            if let Some(tx) = lock_data.write_waiters.pop_front() {
                lock_data.num_readers -= 1;
                lock_data.exclusive = true;
                tx.send(()).expect("Sender::send");
                return;
            }
        }
        if let Some(tx) = lock_data.read_waiters.pop_front() {
            tx.send(()).expect("Sender::send");
        } else {
            lock_data.num_readers -= 1;
        }
    }

    /// Release an exclusive lock of an `RwLock`.
    fn unlock_writer(&self) {
        let mut lock_data = self.inner.mutex.lock().expect("sync::Mutex::lock");
        assert!(lock_data.num_readers == 0);
        assert!(lock_data.exclusive);
        if let Some(tx) = lock_data.write_waiters.pop_front() {
            tx.send(()).expect("Sender::send");
        } else if let Some(tx) = lock_data.read_waiters.pop_front() {
            lock_data.num_readers += 1;
            lock_data.exclusive = false;
            tx.send(()).expect("Sender::send");
        } else {
            lock_data.exclusive = false;
        }
    }
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for RwLock<T> {}
