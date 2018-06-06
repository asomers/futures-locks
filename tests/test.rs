//vim: tw=80

extern crate futures;
extern crate tokio;
extern crate futures_locks;

use futures::{future, lazy, stream};
use futures::sync::oneshot;
use tokio::executor::current_thread;
use futures_locks::*;


mod mutex {
use super::*;
use futures::{Future, Stream};

// When a pending Mutex gets dropped, it should drain its channel and relinquish
// ownership if a message was found.  If not, deadlocks may result.
#[test]
fn drop_before_poll() {
    let mutex = Mutex::<u32>::new(0);

    let _ = current_thread::block_on_all(lazy(|| {
        let mut fut1 = mutex.lock();
        let guard1 = fut1.poll();    // fut1 immediately gets ownership
        assert!(guard1.as_ref().unwrap().is_ready());
        let mut fut2 = mutex.lock();
        assert!(!fut2.poll().unwrap().is_ready());
        drop(guard1);                // ownership transfers to fut2
        drop(fut1);
        drop(fut2);                  // relinquish ownership
        let mut fut3 = mutex.lock();
        let guard3 = fut3.poll();    // fut3 immediately gets ownership
        assert!(guard3.as_ref().unwrap().is_ready());
        future::ok::<(), ()>(())
    }));
}

// Mutably dereference a uniquely owned Mutex
#[test]
fn get_mut() {
    let mut mutex = Mutex::<u32>::new(42);
    *mutex.get_mut().unwrap() += 1;
    assert_eq!(*mutex.get_mut().unwrap(), 43);
}

// Cloned Mutexes cannot be deferenced
#[test]
fn get_mut_cloned() {
    let mut mutex = Mutex::<u32>::new(42);
    let _clone = mutex.clone();
    assert!(mutex.get_mut().is_none());
}

// Acquire an uncontested Mutex.  poll immediately returns Async::Ready
#[test]
fn lock_uncontested() {
    let mutex = Mutex::<u32>::new(0);

    let result = current_thread::block_on_all(lazy(|| {
        mutex.lock().map(|guard| {
            *guard + 5
        })
    })).unwrap();
    assert_eq!(result, 5);
}

// Pend on a Mutex held by another task in the same tokio Reactor.  poll returns
// Async::NotReady.  Later, it gets woken up without involving the OS.
#[test]
fn lock_contested() {
    let mutex = Mutex::<u32>::new(0);

    let result = current_thread::block_on_all(lazy(|| {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = mutex.lock()
            .and_then(move |mut guard| {
                *guard += 5;
                rx0.map_err(|_| {drop(guard);})
            });
        let task1 = mutex.lock().map(|guard| *guard);
        // Readying task2 before task1 causes Tokio to poll the latter even
        // though it's not ready
        let task2 = rx1.map_err(|_| ()).map(|_| tx0.send(()).unwrap());
        let task3 = lazy(move || {
            tx1.send(()).unwrap();
            future::ok::<(), ()>(())
        });
        task0.join4(task1, task2, task3)
    }));

    assert_eq!(result, Ok(((), 5, (), ())));
}

// A single Mutex is contested by tasks in multiple threads
#[test]
fn lock_multithreaded() {
    let mutex = Mutex::<u32>::new(0);
    let mtx_clone0 = mutex.clone();
    let mtx_clone1 = mutex.clone();
    let mtx_clone2 = mutex.clone();
    let mtx_clone3 = mutex.clone();

    let parent = lazy(move || {
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone0.lock().map(|mut guard| { *guard += 2 })
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone1.lock().map(|mut guard| { *guard += 3 })
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone2.lock().map(|mut guard| { *guard += 5 })
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone3.lock().map(|mut guard| { *guard += 7 })
        }));
        future::ok::<(), ()>(())
    });

    tokio::run(parent);
    assert_eq!(mutex.try_unwrap().expect("try_unwrap"), 17_000);
}

// Mutexes should be acquired in the order that their Futures are waited upon.
#[test]
fn lock_order() {
    let mutex = Mutex::<Vec<u32>>::new(vec![]);
    let fut2 = mutex.lock().map(|mut guard| guard.push(2));
    let fut1 = mutex.lock().map(|mut guard| guard.push(1));

    let r = current_thread::block_on_all(lazy(|| {
        fut1.and_then(|_| fut2)
    }));
    assert!(r.is_ok());
    assert_eq!(mutex.try_unwrap().unwrap(), vec![1, 2]);
}

// Acquire an uncontested Mutex with try_lock
#[test]
fn try_lock_uncontested() {
    let mutex = Mutex::<u32>::new(5);

    let guard = mutex.try_lock().unwrap();
    assert_eq!(5, *guard);
}

// Try and fail to acquire a contested Mutex with try_lock
#[test]
fn try_lock_contested() {
    let mutex = Mutex::<u32>::new(0);

    let _guard = mutex.try_lock().unwrap();
    assert!(mutex.try_lock().is_err());
}

#[test]
fn try_unwrap_multiply_referenced() {
    let mtx = Mutex::<u32>::new(0);
    let _mtx2 = mtx.clone();
    assert!(mtx.try_unwrap().is_err());
}

}

mod rwlock {
use super::*;
use futures::{Future, Stream};

// When an exclusively owned but not yet polled RwLock future is dropped, it
// should relinquish ownership.  If not, deadlocks may result.
#[test]
fn drop_exclusive_before_poll() {
    let rwlock = RwLock::<u32>::new(42);

    let _ = current_thread::block_on_all(lazy(|| {
        let mut fut1 = rwlock.read();
        let guard1 = fut1.poll();       // fut1 immediately gets ownership
        assert!(guard1.as_ref().unwrap().is_ready());
        let mut fut2 = rwlock.write();
        assert!(!fut2.poll().unwrap().is_ready());
        drop(guard1);                   // ownership transfers to fut2
        //drop(fut1);
        drop(fut2);                     // relinquish ownership
        let mut fut3 = rwlock.read();
        let guard3 = fut3.poll();       // fut3 immediately gets ownership
        assert!(guard3.as_ref().unwrap().is_ready());
        future::ok::<(), ()>(())
    }));
}

// When an nonexclusively owned but not yet polled RwLock future is dropped, it
// should relinquish ownership.  If not, deadlocks may result.
#[test]
fn drop_shared_before_poll() {
    let rwlock = RwLock::<u32>::new(42);

    let _ = current_thread::block_on_all(lazy(|| {
        let mut fut1 = rwlock.write();
        let guard1 = fut1.poll();       // fut1 immediately gets ownership
        assert!(guard1.as_ref().unwrap().is_ready());
        let mut fut2 = rwlock.read();
        assert!(!fut2.poll().unwrap().is_ready());
        drop(guard1);                   // ownership transfers to fut2
        //drop(fut1);
        drop(fut2);                     // relinquish ownership
        let mut fut3 = rwlock.write();
        let guard3 = fut3.poll();       // fut3 immediately gets ownership
        assert!(guard3.as_ref().unwrap().is_ready());
        future::ok::<(), ()>(())
    }));
}

// Mutably dereference a uniquely owned RwLock
#[test]
fn get_mut() {
    let mut rwlock = RwLock::<u32>::new(42);
    *rwlock.get_mut().unwrap() += 1;
    assert_eq!(*rwlock.get_mut().unwrap(), 43);
}

// Cloned RwLocks cannot be deferenced
#[test]
fn get_mut_cloned() {
    let mut rwlock = RwLock::<u32>::new(42);
    let _clone = rwlock.clone();
    assert!(rwlock.get_mut().is_none());
}

// Acquire an RwLock nonexclusively by two different tasks simultaneously .
#[test]
fn read_shared() {
    let rwlock = RwLock::<u32>::new(42);

    let result = current_thread::block_on_all(lazy(|| {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.read()
            .and_then(move |guard| {
                tx1.send(()).unwrap();
                rx0.map(move |_| *guard).map_err(|_| ())
            });
        let task1 = rwlock.read()
            .and_then(move |guard| {
                tx0.send(()).unwrap();
                rx1.map(move |_| *guard).map_err(|_| ())
            });
        task0.join(task1)
    }));

    assert_eq!(result, Ok((42, 42)));
}

// Acquire an RwLock nonexclusively by a single task
#[test]
fn read_uncontested() {
    let rwlock = RwLock::<u32>::new(42);

    let result = current_thread::block_on_all(lazy(|| {
        rwlock.read().map(|guard| {
            *guard
        })
    })).unwrap();

    assert_eq!(result, 42);
}

// Attempt to acquire an RwLock for reading that already has a writer
#[test]
fn read_contested() {
    let rwlock = RwLock::<u32>::new(0);

    let result = current_thread::block_on_all(lazy(|| {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.write()
            .and_then(move |mut guard| {
                *guard += 5;
                rx0.map_err(|_| {drop(guard);})
            });
        let task1 = rwlock.read().map(|guard| *guard);
        let task2 = rwlock.read().map(|guard| *guard);
        // Readying task3 before task1 and task2 causes Tokio to poll the latter
        // even though they're not ready
        let task3 = rx1.map_err(|_| ()).map(|_| tx0.send(()).unwrap());
        let task4 = lazy(move || {
            tx1.send(()).unwrap();
            future::ok::<(), ()>(())
        });
        task0.join5(task1, task2, task3, task4)
    }));

    assert_eq!(result, Ok(((), 5, 5, (), ())));
}

// Attempt to acquire an rwlock exclusively when it already has a reader.
// 1) task0 will run first, reading the rwlock's original value and blocking on
//    rx.
// 2) task1 will run next, but block on acquiring rwlock.
// 3) task2 will run next, reading the rwlock's value and returning immediately.
// 4) task3 will run next, waking up task0 with the oneshot
// 5) finally task1 will acquire the rwlock and increment it.
//
// If RwLock::write is allowed to acquire an RwLock with readers, then task1
// would erroneously run before task2, and task2 would return the wrong value.
#[test]
fn read_write_contested() {
    let rwlock = RwLock::<u32>::new(42);

    let result = current_thread::block_on_all(lazy(|| {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.read()
            .and_then(move |guard| {
                rx0.map(move |_| { *guard }).map_err(|_| ())
            });
        let task1 = rwlock.write().map(|mut guard| *guard += 1);
        let task2 = rwlock.read().map(|guard| *guard);
        // Readying task3 before task1 and task2 causes Tokio to poll the latter
        // even though they're not ready
        let task3 = rx1.map_err(|_| ()).map(|_| tx0.send(()).unwrap());
        let task4 = lazy(move || {
            tx1.send(()).unwrap();
            future::ok::<(), ()>(())
        });
        task0.join5(task1, task2, task3, task4)
    }));

    assert_eq!(result, Ok((42, (), 42, (), ())));
    assert_eq!(rwlock.try_unwrap().expect("try_unwrap"), 43);
}

#[test]
fn try_read_uncontested() {
    let rwlock = RwLock::<u32>::new(42);
    assert_eq!(42, *rwlock.try_read().unwrap());
}

#[test]
fn try_read_contested() {
    let rwlock = RwLock::<u32>::new(42);
    let _guard = rwlock.try_write();
    assert!(rwlock.try_read().is_err());
}

#[test]
fn try_unwrap_multiply_referenced() {
    let rwlock = RwLock::<u32>::new(0);
    let _rwlock2 = rwlock.clone();
    assert!(rwlock.try_unwrap().is_err());
}

#[test]
fn try_write_uncontested() {
    let rwlock = RwLock::<u32>::new(0);
    *rwlock.try_write().unwrap() += 5;
    assert_eq!(5, rwlock.try_unwrap().unwrap());
}

#[test]
fn try_write_contested() {
    let rwlock = RwLock::<u32>::new(42);
    let _guard = rwlock.try_read();
    assert!(rwlock.try_write().is_err());
}

// Acquire an uncontested RwLock in exclusive mode.  poll immediately returns
// Async::Ready
#[test]
fn write_uncontested() {
    let rwlock = RwLock::<u32>::new(0);

    current_thread::block_on_all(lazy(|| {
        rwlock.write().map(|mut guard| {
            *guard += 5;
        })
    })).unwrap();
    assert_eq!(rwlock.try_unwrap().expect("try_unwrap"), 5);
}

// Pend on an RwLock held exclusively by another task in the same tokio Reactor.
// poll returns Async::NotReady.  Later, it gets woken up without involving the
// OS.
#[test]
fn write_contested() {
    let rwlock = RwLock::<u32>::new(0);

    let result = current_thread::block_on_all(lazy(|| {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.write()
            .and_then(move |mut guard| {
                *guard += 5;
                rx0.map_err(|_| {drop(guard);})
            });
        let task1 = rwlock.write().map(|guard| *guard);
        // Readying task2 before task1 causes Tokio to poll the latter
        // even though it's not ready
        let task2 = rx1.map_err(|_| ()).map(|_| tx0.send(()).unwrap());
        let task3 = lazy(move || {
            tx1.send(()).unwrap();
            future::ok::<(), ()>(())
        });
        task0.join4(task1, task2, task3)
    }));

    assert_eq!(result, Ok(((), 5, (), ())));
}

// RwLocks should be acquired in the order that their Futures are waited upon.
#[test]
fn write_order() {
    let rwlock = RwLock::<Vec<u32>>::new(vec![]);
    let fut2 = rwlock.write().map(|mut guard| guard.push(2));
    let fut1 = rwlock.write().map(|mut guard| guard.push(1));

    let r = current_thread::block_on_all(lazy(|| {
        fut1.and_then(|_| fut2)
    }));
    assert!(r.is_ok());
    assert_eq!(rwlock.try_unwrap().unwrap(), vec![1, 2]);
}

// A single RwLock is contested by tasks in multiple threads
#[test]
fn multithreaded() {
    let rwlock = RwLock::<u32>::new(0);
    let rwlock_clone0 = rwlock.clone();
    let rwlock_clone1 = rwlock.clone();
    let rwlock_clone2 = rwlock.clone();
    let rwlock_clone3 = rwlock.clone();

    let parent = lazy(move || {
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            let rwlock_clone4 = rwlock_clone0.clone();
            rwlock_clone0.write().map(|mut guard| { *guard += 2 })
                .and_then(move |_| rwlock_clone4.read().map(|_| ()))
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            let rwlock_clone5 = rwlock_clone1.clone();
            rwlock_clone1.write().map(|mut guard| { *guard += 3 })
                .and_then(move |_| rwlock_clone5.read().map(|_| ()))
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            let rwlock_clone6 = rwlock_clone2.clone();
            rwlock_clone2.write().map(|mut guard| { *guard += 5 })
                .and_then(move |_| rwlock_clone6.read().map(|_| ()))
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            let rwlock_clone7 = rwlock_clone3.clone();
            rwlock_clone3.write().map(|mut guard| { *guard += 7 })
                .and_then(move |_| rwlock_clone7.read().map(|_| ()))
        }));
        future::ok::<(), ()>(())
    });

    tokio::run(parent);
    assert_eq!(rwlock.try_unwrap().expect("try_unwrap"), 17_000);
}

}
