use std::io;

use async_std::stream::{from_iter, Stream, StreamExt};
use async_std::task;

use async_listen::{ListenExt, error_hint};

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

#[test]
#[cfg(target_os="linux")]  // other OSs may have different error code or text
fn test_hint() {
    let e = io::Error::from_raw_os_error(24);
    assert_eq!(
        format!("Error: {}. {}", e, error_hint(&e)),
        "Error: Too many open files (os error 24). \
         Increase per-process open file limit \
         https://bit.ly/async-err#EMFILE");
    let e = io::Error::from_raw_os_error(23);
    assert_eq!(
        format!("Error: {}. {}", e, error_hint(&e)),
        "Error: Too many open files in system (os error 23). \
         Increase system open file limit \
         https://bit.ly/async-err#ENFILE");
    let e = io::ErrorKind::Other.into();
    assert_eq!(
        format!("Error: {}. {}", e, error_hint(&e)),
        "Error: other os error. ");
}
