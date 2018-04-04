//vim: tw=80

extern crate futures;
extern crate tokio;
extern crate futures_locks;

use futures::{Future, Stream, future, lazy, stream};
use futures::sync::oneshot;
use tokio::executor::current_thread;
use futures_locks::*;


// Acquire an uncontested Mutex.  poll immediately returns Async::Ready
#[test]
fn mutex_lock_uncontested() {
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
fn mutex_lock_contested() {
    let mutex = Mutex::<u32>::new(0);

    let result = current_thread::block_on_all(lazy(|| {
        let (tx, rx) = oneshot::channel::<()>();
        let task0 = mutex.lock()
            .and_then(move |mut guard| {
                *guard += 5;
                rx
            }).map_err(|_| ());
        let task1 = mutex.lock().map(|guard| *guard).map_err(|_| ());
        let task2 = lazy(move || {
            tx.send(()).unwrap();
            future::ok::<(), ()>(())
        });
        task0.join3(task1, task2)
    }));

    assert_eq!(result, Ok(((), 5, ())));
}

// A single Mutex is contested by tasks in multiple threads
#[test]
fn mutex_lock_multithreaded() {
    let mutex = Mutex::<u32>::new(0);
    let mtx_clone0 = mutex.clone();
    let mtx_clone1 = mutex.clone();
    let mtx_clone2 = mutex.clone();
    let mtx_clone3 = mutex.clone();

    let parent = lazy(move || {
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone0.lock().map(|mut guard| { *guard += 2 }).map_err(|_| ())
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone1.lock().map(|mut guard| { *guard += 3 }).map_err(|_| ())
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone2.lock().map(|mut guard| { *guard += 5 }).map_err(|_| ())
        }));
        tokio::spawn(stream::iter_ok::<_, ()>(0..1000).for_each(move |_| {
            mtx_clone3.lock().map(|mut guard| { *guard += 7 }).map_err(|_| ())
        }));
        future::ok::<(), ()>(())
    });

    tokio::run(parent);
    assert_eq!(mutex.try_unwrap().expect("try_unwrap"), 17_000);
}
