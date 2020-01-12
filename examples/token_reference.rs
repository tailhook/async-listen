use std::error::Error;
use std::time::Duration;

use async_std::task;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;

use async_listen::{ListenExt, backpressure::{self, Token}};


fn main() -> Result<(), Box<dyn Error>> {
    let (tok_gen, throttle) = backpressure::new(10);
    task::block_on(async {
        let listener = TcpListener::bind("localhost:8080").await?;
        let mut incoming = listener
            .incoming()
            .log_warnings(|e| eprintln!("Error: {}. Pausing for 500ms.", e))
            .handle_errors(Duration::from_millis(500)) // 1
            .apply_backpressure(throttle);
        while let Some(stream) = incoming.next().await { // 2
            let token = tok_gen.token();
            task::spawn(async move {
                connection_loop(&token, stream).await
            });
        }
        Ok(())
    })
}

async fn connection_loop(_token: &Token, _stream: TcpStream) {
    task::sleep(Duration::from_secs(10)).await;
}
