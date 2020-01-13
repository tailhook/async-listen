use std::error::Error;
use std::time::Duration;

use async_std::task;
use async_std::net::TcpListener;
use async_std::prelude::*;

use rand::{thread_rng, Rng};
use async_listen::{ListenExt, ByteStream, backpressure, error_hint};


fn main() -> Result<(), Box<dyn Error>> {
    let (metrics, bp) = backpressure::new(10);
    let metrics1 = metrics.clone();
    task::spawn(async move {
        loop {
            task::sleep(Duration::from_secs(10)).await;
            println!("Currently connected {}", metrics1.get_active_tokens());
        }
    });
    task::block_on(async move {
        let listener = TcpListener::bind("localhost:8080").await?;
        eprintln!("Accepting connections on localhost:8080");
        let mut incoming = listener.incoming()
            .log_warnings(|e| {
                eprintln!("Accept error: {}. Paused listener for 0.5s. {}",
                          e, error_hint(&e))
            })
            .handle_errors(Duration::from_millis(500))
            .backpressure_wrapper(bp);
        while let Some(stream) = incoming.next().await {
            task::spawn(connection_loop(stream, metrics.clone()));
        }
        Ok(())
    })
}
async fn connection_loop(mut stream: ByteStream, metrics: backpressure::Sender)
{
    let delay = Duration::from_millis(thread_rng().gen_range(100, 5000));
    task::sleep(delay).await;
    let text = format!("Connections {}\n", metrics.get_active_tokens());
    stream.write_all(text.as_bytes()).await
        .map_err(|e| eprintln!("Write error: {}", e)).ok();
}
