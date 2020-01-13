use std::env::args;
use std::error::Error;
use std::fs::remove_file;
use std::io;
use std::time::Duration;

use async_std::task;
use async_std::net::TcpListener;
use async_std::prelude::*;

use async_listen::{ListenExt, ByteStream, backpressure, error_hint};

fn main() -> Result<(), Box<dyn Error>> {
    let (_, bp) = backpressure::new(10);
    #[cfg(unix)] {
        use async_std::os::unix::net::UnixListener;

        if args().any(|x| x == "--unix") {
            remove_file("./example.sock").ok();
            return task::block_on(async move {
                let listener = UnixListener::bind("./example.sock").await?;
                eprintln!("Accepting connections on ./example.sock");
                let mut incoming = listener.incoming()
                    .log_warnings(log_error)
                    .handle_errors(Duration::from_millis(500))
                    .backpressure_wrapper(bp);
                while let Some(stream) = incoming.next().await {
                    task::spawn(connection_loop(stream));
                }
                Ok(())
            });
        }
    }
    task::block_on(async move {
        let listener = TcpListener::bind("localhost:8080").await?;
        eprintln!("Accepting connections on localhost:8080");
        let mut incoming = listener.incoming()
            .log_warnings(log_error)
            .handle_errors(Duration::from_millis(500))
            .backpressure_wrapper(bp);
        while let Some(stream) = incoming.next().await {
             task::spawn(async {
                 if let Err(e) = connection_loop(stream).await {
                     eprintln!("Error: {}", e);
                 }
             });
        }
        Ok(())
    })
}

async fn connection_loop(mut stream: ByteStream) -> Result<(), io::Error> {
    println!("Connected from {}", stream.peer_addr()?);
    task::sleep(Duration::from_secs(5)).await;
    stream.write_all("hello\n".as_bytes()).await?;
    Ok(())
}

fn log_error(e: &io::Error) {
    eprintln!("Accept error: {}. Paused for 0.5s. {}", e, error_hint(&e));
}
