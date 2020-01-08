use std::io;

use async_std::stream::{from_iter, Stream, StreamExt};
use async_std::task;

use async_server::ListenExt;

fn collect<S: Stream + Unpin>(mut stream: S) -> Vec<S::Item> {
    task::block_on(async {
        let mut result = Vec::new();
        while let Some(item) = stream.next().await {
            result.push(item);
        }
        result
    })
}

#[test]
fn test_log() {
    let s = from_iter(vec![
        Ok(1u32),
        Err(io::ErrorKind::ConnectionReset.into()),
        Ok(2),
        Err(io::ErrorKind::Other.into()),
        Ok(3),
    ]);
    let mut visited = false;
    let stream = s.log_warnings(|e| {
        assert_eq!(e.kind(), io::ErrorKind::Other);
        visited = true;
    });
    let result = collect(stream);
    assert_eq!(result.len(), 5);
    assert!(visited);
}
