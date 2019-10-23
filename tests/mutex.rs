//vim: tw=80

use std::future::Future;
use std::task::Poll;
use std::pin::Pin;
use futures::channel::oneshot;
use futures::{future, stream, StreamExt, FutureExt, TryFutureExt};
#[cfg(feature = "tokio")]
use std::rc::Rc;
use tokio;
#[cfg(feature = "tokio")]
use tokio::runtime::current_thread;
use futures_locks::*;

// Create a MutexWeak and then upgrade it to Mutex
#[test]
fn mutex_weak_some() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_weak = Mutex::downgrade(&mutex);

    assert!(mutex_weak.upgrade().is_some())
}

// Create a MutexWeak and drop the mutex so that MutexWeak::upgrade return None
#[test]
fn mutex_weak_none() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_weak = Mutex::downgrade(&mutex);

    drop(mutex);

    assert!(mutex_weak.upgrade().is_none())
}

// Compare Mutexes if it point to the same value
#[test]
fn mutex_eq_ptr_true() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_other = mutex.clone();

    assert!(Mutex::ptr_eq(&mutex, &mutex_other));
}

// Compare Mutexes if it point to the same value
#[test]
fn mutex_eq_ptr_false() {
    let mutex = Mutex::<u32>::new(0);
    let mutex_other = Mutex::<u32>::new(0);

    assert!(!Mutex::ptr_eq(&mutex, &mutex_other));
}

// When a pending Mutex gets dropped, it should drain its channel and relinquish
// ownership if a message was found.  If not, deadlocks may result.
#[tokio::test]
async fn drop_before_poll() {

    future::poll_fn(|cx| {
        let mutex = Mutex::<u32>::new(0);
        let mut fut1 = mutex.lock();
        let guard1 = Pin::new(&mut fut1).poll(cx);    // fut1 immediately gets ownership
        assert!(guard1.is_ready());
        let mut fut2 = mutex.lock();
        assert!(Pin::new(&mut fut2).poll(cx).is_pending());
        drop(guard1);                // ownership transfers to fut2
        drop(fut1);
        drop(fut2);                  // relinquish ownership
        let mut fut3 = mutex.lock();
        let guard3 = Pin::new(&mut fut3).poll(cx);    // fut3 immediately gets ownership
        assert!(guard3.is_ready());
        Poll::Ready(())
    }).await
}

// // Mutably dereference a uniquely owned Mutex
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
#[tokio::test]
async fn lock_uncontested() {
    let mutex = Mutex::<u32>::new(0);

    let guard = mutex.lock().await;
    let result = *guard + 5;

    assert_eq!(result, 5);
}

// Pend on a Mutex held by another task in the same tokio Reactor.  poll returns
// Async::NotReady.  Later, it gets woken up without involving the OS.
#[tokio::test]
async fn lock_contested() {
    let mutex = Mutex::<u32>::new(0);

    let (tx0, rx0) = oneshot::channel::<()>();
    let (tx1, rx1) = oneshot::channel::<()>();
    let task0 = mutex.lock()
        .then(move |mut guard| {
            *guard += 5;
            rx0.map_err(|_| {drop(guard);})
        });
    let task1 = mutex.lock().map(|guard| *guard);
    // Readying task2 before task1 causes Tokio to poll the latter even
    // though it's not ready
    let task2 = rx1.map(|_| tx0.send(()).unwrap());
    let task3 = async move {
        tx1.send(()).unwrap();
    };

    let result = future::join4(task0, task1, task2, task3).await;
    assert_eq!(result, (Ok(()), 5, (), ()));
}

// A single Mutex is contested by tasks in multiple threads
#[tokio::test]
async fn lock_multithreaded() {
    let mutex = Mutex::<u32>::new(0);
    let mtx_clone0 = mutex.clone();
    let mtx_clone1 = mutex.clone();
    let mtx_clone2 = mutex.clone();
    let mtx_clone3 = mutex.clone();

    let fut1 = stream::iter(0..1000).for_each(move |_| {
        mtx_clone0.lock().map(|mut guard| { *guard += 2 })
    });

    let fut2 = stream::iter(0..1000).for_each(move |_| {
        mtx_clone1.lock().map(|mut guard| { *guard += 3 })
    });

    let fut3 = stream::iter(0..1000).for_each(move |_| {
        mtx_clone2.lock().map(|mut guard| { *guard += 5 })
    });

    let fut4 = stream::iter(0..1000).for_each(move |_| {
        mtx_clone3.lock().map(|mut guard| { *guard += 7 })
    });

    future::join4(fut1, fut2, fut3, fut4).await;

    assert_eq!(mutex.try_unwrap().expect("try_unwrap"), 17_000);
}

// Mutexes should be acquired in the order that their Futures are waited upon.
#[tokio::test]
async fn lock_order() {
    let mutex = Mutex::<Vec<u32>>::new(vec![]);
    let fut2 = mutex.lock().map(|mut guard| guard.push(2));
    let fut1 = mutex.lock().map(|mut guard| guard.push(1));

    fut1.then(|_| fut2).await;

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

#[cfg(feature = "tokio")]
#[test]
fn with_err() {
    let mtx = Mutex::<i32>::new(-5);
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(async {
        mtx.with(|guard| {
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
fn with_ok() {
    let mtx = Mutex::<i32>::new(5);
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(async move {
        mtx.with(|guard| {
            futures::future::ok::<i32, ()>(*guard)
        }).unwrap().await
    });
    assert_eq!(r, Ok(5));
}

// Mutex::with should work with multithreaded Runtimes as well as
// single-threaded Runtimes.
// https://github.com/asomers/futures-locks/issues/5
#[cfg(feature = "tokio")]
#[tokio::test]
async fn with_threadpool() {
    let mtx = Mutex::<i32>::new(5);
    let r = mtx.with(|guard| {
        futures::future::ok::<i32, ()>(*guard)
    }).unwrap().await;

    assert!(r.is_ok());
    assert_eq!(r, Ok(5));
}

#[cfg(feature = "tokio")]
#[test]
fn with_local_ok() {
    // Note: Rc is not Send
    let mtx = Mutex::<Rc<i32>>::new(Rc::new(5));
    let mut rt = current_thread::Runtime::new().unwrap();
    let r = rt.block_on(async move {
        mtx.with_local(|guard| {
            futures::future::ok::<i32, ()>(**guard)
        }).await
    });
    assert_eq!(r, Ok(5));
}
