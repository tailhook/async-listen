use std::error::Error;
use std::time::Duration;

use async_std::task;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;

use async_listen::ListenExt;


fn main() -> Result<(), Box<dyn Error>> {
    task::block_on(async {
        let listener = TcpListener::bind("localhost:8080").await?;
        eprintln!("Accepting connections on localhost:8080");
        let mut incoming = listener.incoming()
            .log_warnings(|e| eprintln!("Error: {}. Sleeping 0.5s...", e))
            .handle_errors(Duration::from_millis(500));
        while let Some(stream) = incoming.next().await {
            task::spawn(connection_loop(stream));
        }
        Ok(())
    })
}

async fn connection_loop(_stream: TcpStream) {
    task::sleep(Duration::from_secs(10)).await;
}
