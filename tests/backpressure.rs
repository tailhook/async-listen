use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rand::{Rng, thread_rng};

use async_std::stream::{from_iter, Stream, StreamExt};
use async_std::task;

use async_listen::{ListenExt, backpressure};

fn collect<S: Stream + Unpin>(mut stream: S) -> Vec<S::Item> {
    task::block_on(async {
        let mut result = Vec::new();
        while let Some(item) = stream.next().await {
            result.push(item);
        }
        result
    })
}

fn random_delay() -> Duration {
    Duration::from_millis(thread_rng().gen_range(1, 200))
}

#[test]
fn test_backpressure() {
    let current = Arc::new(AtomicUsize::new(0));
    let top = Arc::new(AtomicUsize::new(0));
    let tasks = collect(
        from_iter(0..100)
        .backpressure(10)
        .map(|(token, _index)| {
            let current = current.clone();
            let top = top.clone();
            task::spawn(async move {
                let size = current.fetch_add(1, Ordering::Acquire) + 1;
                assert!(size <= 10);
                if size == 10 {
                    top.fetch_add(1, Ordering::Relaxed);
                }
                task::sleep(random_delay()).await;
                current.fetch_sub(1, Ordering::Acquire);
                drop(token);
            })
        }));
    for item in tasks {
        task::block_on(item);
    };
    let top = top.load(Ordering::SeqCst);
    println!("Top capacity {}", top);
    assert!(50 < top && top <= 91);
}

#[test]
fn test_apply_backpressure() {
    let current = Arc::new(AtomicUsize::new(0));
    let top = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = backpressure::new(10);
    let tasks = collect(
        from_iter(0..100)
        .apply_backpressure(rx)
        .map(|_index| {
            let token = tx.token();
            let current = current.clone();
            let top = top.clone();
            task::spawn(async move {
                let size = current.fetch_add(1, Ordering::Acquire) + 1;
                assert!(size <= 10);
                if size == 10 {
                    top.fetch_add(1, Ordering::Relaxed);
                }
                task::sleep(random_delay()).await;
                current.fetch_sub(1, Ordering::Acquire);
                drop(token);
            })
        }));
    for item in tasks {
        task::block_on(item);
    };
    let top = top.load(Ordering::SeqCst);
    println!("Top capacity {}", top);
    assert!(50 < top && top <= 91);
}

#[test]
fn test_change_limit() {
    let current = Arc::new(AtomicUsize::new(0));
    let top = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = backpressure::new(10);
    let tasks = collect(
        from_iter(0..100)
        .apply_backpressure(rx)
        .map(|index| {
            let tx = tx.clone();
            let token = tx.token();
            let current = current.clone();
            let top = top.clone();
            task::spawn(async move {
                let size = current.fetch_add(1, Ordering::Acquire) + 1;
                assert!(size <= 20);
                if size == 20 {
                    top.fetch_add(1, Ordering::Relaxed);
                }
                task::sleep(random_delay()).await;
                if index == 20 {
                    tx.set_limit(20);
                } else if index == 60 {
                    tx.set_limit(5);
                }
                current.fetch_sub(1, Ordering::Acquire);
                drop(token);
            })
        }));
    for item in tasks {
        task::block_on(item);
    };
    let top = top.load(Ordering::SeqCst);
    println!("Top capacity {}", top);
    assert!(5 < top && top <= 61);
}
