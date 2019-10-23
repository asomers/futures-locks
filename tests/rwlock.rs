//vim: tw=80

use std::pin::Pin;
use std::task::Poll;
use futures::{Future, FutureExt, TryFutureExt, StreamExt, future, stream};
use futures::channel::oneshot;
#[cfg(feature = "tokio")]
use std::rc::Rc;
use tokio;
use tokio::runtime::current_thread;
use futures_locks::*;


// When an exclusively owned but not yet polled RwLock future is dropped, it
// should relinquish ownership.  If not, deadlocks may result.
#[test]
fn drop_exclusive_before_poll() {
    let rwlock = RwLock::<u32>::new(42);
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(future::poll_fn(|cx| {
        let mut fut1 = rwlock.read();
        let guard1 = Pin::new(&mut fut1).poll(cx);       // fut1 immediately gets ownership
        assert!(guard1.is_ready());
        let mut fut2 = rwlock.write();
        assert!(Pin::new(&mut fut2).poll(cx).is_pending());
        drop(guard1);                   // ownership transfers to fut2
        //drop(fut1);
        drop(fut2);                     // relinquish ownership
        let mut fut3 = rwlock.read();
        let guard3 = Pin::new(&mut fut3).poll(cx);       // fut3 immediately gets ownership
        assert!(guard3.is_ready());
        Poll::Ready(())
    }));
}

// When an nonexclusively owned but not yet polled RwLock future is dropped, it
// should relinquish ownership.  If not, deadlocks may result.
#[test]
fn drop_shared_before_poll() {
    let rwlock = RwLock::<u32>::new(42);
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(future::poll_fn(|cx| {
        let mut fut1 = rwlock.write();
        let guard1 = Pin::new(&mut fut1).poll(cx);       // fut1 immediately gets ownership
        assert!(guard1.is_ready());
        let mut fut2 = rwlock.read();
        assert!(Pin::new(&mut fut2).poll(cx).is_pending());
        drop(guard1);                   // ownership transfers to fut2
        //drop(fut1);
        drop(fut2);                     // relinquish ownership
        let mut fut3 = rwlock.write();
        let guard3 = Pin::new(&mut fut3).poll(cx);       // fut3 immediately gets ownership
        assert!(guard3.is_ready());
        Poll::Ready(())
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
    let mut rt = current_thread::Runtime::new().unwrap();

    let result = rt.block_on(async {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.read()
            .then(move |guard| {
                tx1.send(()).unwrap();
                rx0.map(move |_| *guard)
            });
        let task1 = rwlock.read()
            .then(move |guard| {
                tx0.send(()).unwrap();
                rx1.map(move |_| *guard)
            });
        future::join(task0, task1).await
    });

    assert_eq!(result, (42, 42));
}

// Acquire an RwLock nonexclusively by a single task
#[test]
fn read_uncontested() {
    let rwlock = RwLock::<u32>::new(42);
    let mut rt = current_thread::Runtime::new().unwrap();

    let result = rt.block_on(async {
        rwlock.read().map(|guard| {
            *guard
        }).await
    });

    assert_eq!(result, 42);
}

// Attempt to acquire an RwLock for reading that already has a writer
#[test]
fn read_contested() {
    let rwlock = RwLock::<u32>::new(0);
    let mut rt = current_thread::Runtime::new().unwrap();

    let result = rt.block_on(async {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.write()
            .then(move |mut guard| {
                *guard += 5;
                rx0.map_err(|_| {drop(guard);})
            });
        let task1 = rwlock.read().map(|guard| *guard);
        let task2 = rwlock.read().map(|guard| *guard);
        // Readying task3 before task1 and task2 causes Tokio to poll the latter
        // even though they're not ready
        let task3 = rx1.map_err(|_| ()).map(|_| tx0.send(()).unwrap());
        let task4 = async { tx1.send(()) };

        future::join5(task0, task1, task2, task3, task4).await
   });

   assert_eq!(result, (Ok(()), 5, 5, (), Ok(())));
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
    let mut rt = current_thread::Runtime::new().unwrap();

    let result = rt.block_on(async {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.read()
            .then(move |guard| {
                rx0.map(move |_| { *guard })
            });
        let task1 = rwlock.write().map(|mut guard| *guard += 1);
        let task2 = rwlock.read().map(|guard| *guard);
        // Readying task3 before task1 and task2 causes Tokio to poll the latter
        // even though they're not ready
        let task3 = rx1.map_err(|_| ()).map(|_| tx0.send(()).unwrap());
        let task4 = async move {
            tx1.send(()).unwrap();
        };
        future::join5(task0, task1, task2, task3, task4).await
    });

    assert_eq!(result, (42, (), 42, (), ()));
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
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(async {
        rwlock.write().map(|mut guard| {
            *guard += 5;
        }).await
    });
    assert_eq!(rwlock.try_unwrap().expect("try_unwrap"), 5);
}

// Pend on an RwLock held exclusively by another task in the same tokio Reactor.
// poll returns Async::NotReady.  Later, it gets woken up without involving the
// OS.
#[test]
fn write_contested() {
    let rwlock = RwLock::<u32>::new(0);
    let mut rt = current_thread::Runtime::new().unwrap();

    let result = rt.block_on(async {
        let (tx0, rx0) = oneshot::channel::<()>();
        let (tx1, rx1) = oneshot::channel::<()>();
        let task0 = rwlock.write()
            .then(move |mut guard| {
                *guard += 5;
                rx0.map_err(|_| {drop(guard);})
            });
        let task1 = rwlock.write().map(|guard| *guard);
        // Readying task2 before task1 causes Tokio to poll the latter
        // even though it's not ready
        let task2 = rx1.map(|_| tx0.send(()).unwrap());
        let task3 = async move {
            tx1.send(()).unwrap();
        };
        future::join4(task0, task1, task2, task3).await
    });

    assert_eq!(result, (Ok(()), 5, (), ()));
}

// RwLocks should be acquired in the order that their Futures are waited upon.
#[test]
fn write_order() {
    let rwlock = RwLock::<Vec<u32>>::new(vec![]);
    let fut2 = rwlock.write().map(|mut guard| guard.push(2));
    let fut1 = rwlock.write().map(|mut guard| guard.push(1));
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(async {
        fut1.then(|_| fut2).await
    });
    assert_eq!(rwlock.try_unwrap().unwrap(), vec![1, 2]);
}

// A single RwLock is contested by tasks in multiple threads
#[tokio::test]
async fn multithreaded() {
    let rwlock = RwLock::<u32>::new(0);
    let rwlock_clone0 = rwlock.clone();
    let rwlock_clone1 = rwlock.clone();
    let rwlock_clone2 = rwlock.clone();
    let rwlock_clone3 = rwlock.clone();

    let fut1 = stream::iter(0..1000).for_each(move |_| {
        let rwlock_clone4 = rwlock_clone0.clone();
        rwlock_clone0.write().map(|mut guard| { *guard += 2 })
            .then(move |_| rwlock_clone4.read().map(|_| ()))
    });
    let fut2 = stream::iter(0..1000).for_each(move |_| {
        let rwlock_clone5 = rwlock_clone1.clone();
        rwlock_clone1.write().map(|mut guard| { *guard += 3 })
            .then(move |_| rwlock_clone5.read().map(|_| ()))
    });
    let fut3 = stream::iter(0..1000).for_each(move |_| {
        let rwlock_clone6 = rwlock_clone2.clone();
        rwlock_clone2.write().map(|mut guard| { *guard += 5 })
            .then(move |_| rwlock_clone6.read().map(|_| ()))
    });
    let fut4 = stream::iter(0..1000).for_each(move |_| {
        let rwlock_clone7 = rwlock_clone3.clone();
        rwlock_clone3.write().map(|mut guard| { *guard += 7 })
            .then(move |_| rwlock_clone7.read().map(|_| ()))
    });

    future::join4(fut1, fut2, fut3, fut4).await;
    assert_eq!(rwlock.try_unwrap().expect("try_unwrap"), 17_000);
}

#[cfg(feature = "tokio")]
#[test]
fn with_read_err() {
    let mtx = RwLock::<i32>::new(-5);
    let mut rt = current_thread::Runtime::new().unwrap();

    let r = rt.block_on(async {
        mtx.with_read(|guard| {
            if *guard > 0 {
                future::ok(*guard)
            } else {
                future::err("Whoops!")
            }
        }).unwrap().await
    });
    assert_eq!(r, Err("Whoops!"));
}

#[cfg(feature = "tokio")]
#[test]
fn with_read_ok() {
    let mtx = RwLock::<i32>::new(5);
    let mut rt = current_thread::Runtime::new().unwrap();

    let r = rt.block_on(async {
        mtx.with_read(|guard| {
            futures::future::ok::<i32, ()>(*guard)
        }).unwrap().await
    });
    assert_eq!(r, Ok(5));
}

// RwLock::with_read should work with multithreaded Runtimes as well as
// single-threaded Runtimes.
// https://github.com/asomers/futures-locks/issues/5
#[cfg(feature = "tokio")]
#[tokio::test]
async fn with_read_threadpool() {
    let mtx = RwLock::<i32>::new(5);

    let r = mtx.with_read(|guard| {
        futures::future::ok::<i32, ()>(*guard)
    }).unwrap().await;

    assert_eq!(r, Ok(5));
}

#[cfg(feature = "tokio")]
#[test]
fn with_read_local_ok() {
    // Note: Rc is not Send
    let rwlock = RwLock::<Rc<i32>>::new(Rc::new(5));
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(async move {
        rwlock.with_read_local(|guard| {
            futures::future::ok::<i32, ()>(**guard)
        }).await
    });
    assert_eq!(r, Ok(5));
}

#[cfg(feature = "tokio")]
#[test]
fn with_write_err() {
    let mtx = RwLock::<i32>::new(-5);
    let mut rt = current_thread::Runtime::new().unwrap();

    let r = rt.block_on(async move {
        mtx.with_write(|mut guard| {
            if *guard > 0 {
                *guard -= 1;
                future::ok(())
            } else {
                future::err("Whoops!")
            }
        }).unwrap().await
    });
    assert_eq!(r, Err("Whoops!"));
}

#[cfg(feature = "tokio")]
#[test]
fn with_write_ok() {
    let mtx = RwLock::<i32>::new(5);
    let mut rt = current_thread::Runtime::new().unwrap();

    rt.block_on(async {
        mtx.with_write(|mut guard| {
            *guard += 1;
            future::ok::<(), ()>(())
        }).unwrap().await
    }).unwrap();
    assert_eq!(mtx.try_unwrap().unwrap(), 6);
}

// RwLock::with_write should work with multithreaded Runtimes as well as
// single-threaded Runtimes.
// https://github.com/asomers/futures-locks/issues/5
#[cfg(feature = "tokio")]
#[tokio::test]
async fn with_write_threadpool() {
    let mtx = RwLock::<i32>::new(5);
    let test_mtx = mtx.clone();

    let r = async move {
        mtx.with_write(|mut guard| {
            *guard += 1;
            future::ok::<(), ()>(())
        }).unwrap().await
    }.await;

    assert!(r.is_ok());
    assert_eq!(test_mtx.try_unwrap().unwrap(), 6);
}

#[cfg(feature = "tokio")]
#[test]
fn with_write_local_ok() {
    // Note: Rc is not Send
    let rwlock = RwLock::<Rc<i32>>::new(Rc::new(5));
    let mut rt = current_thread::Runtime::new().unwrap();
    rt.block_on(async {
        rwlock.with_write_local(|mut guard| {
            *Rc::get_mut(&mut *guard).unwrap() += 1;
            future::ok::<(), ()>(())
        }).await.unwrap()
    });

    assert_eq!(*rwlock.try_unwrap().unwrap(), 6);
}
