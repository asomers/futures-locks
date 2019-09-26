#![feature(test)]

extern crate test;
extern crate tokio_ as tokio;

use futures::{
    FutureExt,
    executor::block_on,
    future::join
};
use futures_locks::*;
use test::Bencher;

/// Benchmark the speed of acquiring an uncontested `Mutex`
#[bench]
fn bench_mutex_uncontested(bench: &mut Bencher) {
    let mutex = Mutex::<()>::new(());

    bench.iter(|| {
        block_on(mutex.lock().map(|_guard| ()));
    });
}

/// Benchmark the speed of acquiring a contested `Mutex`
#[bench]
fn bench_mutex_contested(bench: &mut Bencher) {
    let mutex = Mutex::<()>::new(());

    bench.iter(|| {
        let fut0 = mutex.lock().map(|_guard| ());
        let fut1 = mutex.lock().map(|_guard| ());
        block_on(join(fut0, fut1));
    });
}
