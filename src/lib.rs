//! # Async Listen
//!
//! The crate contains various helpers for writing production-ready servers in
//! rust using [async-std](https://async.rs/).
//!
//! [Docs](https://docs.rs/async-listen/) |
//! [Github](https://github.com/tailhook/async-listen/) |
//! [Crate](https://crates.io/crates/async-listen)
//!
//! # Utilities
//! * [ListenExt](trait.ListenExt.html) -- extension trait for stream of
//!   accepted sockets, provides useful conbinators for a stream
//! * [error_hint](fn.error_hint.html) -- shows end-user hints no how to fix
//!   [the most imporant errors](errors/index.html)
//!
//! # Low-Level Utilities
//!
//! * [is_transient_error](fn.is_transient_error.html) -- determines if the
//!   error returned from `accept()` can be ignored
//!
//! # Example
//!
//! Here is a quite elaborate example that demonstrates:
//! * Backpressure (limit on the number of simultaneous connections)
//! * Error handling
//! * Unification of Tcp and Unix sockets
//!
//! ```no_run
//! use std::env::args;
//! use std::error::Error;
//! use std::fs::remove_file;
//! use std::io;
//! use std::time::Duration;
//!
//! use async_std::task;
//! use async_std::net::TcpListener;
//! use async_std::prelude::*;
//!
//! use async_listen::{ListenExt, ByteStream, backpressure, error_hint};
//!
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     let (_, bp) = backpressure::new(10);
//!     #[cfg(unix)] {
//!         use async_std::os::unix::net::UnixListener;
//!
//!         if args().any(|x| x == "--unix") {
//!             remove_file("./example.sock").ok();
//!             return task::block_on(async {
//!                 let listener = UnixListener::bind("./example.sock").await?;
//!                 eprintln!("Accepting connections on ./example.sock");
//!                 let mut incoming = listener.incoming()
//!                     .log_warnings(log_accept_error)
//!                     .handle_errors(Duration::from_millis(500))
//!                     .backpressure_wrapper(bp);
//!                 while let Some(stream) = incoming.next().await {
//!                     task::spawn(connection_loop(stream));
//!                 }
//!                 Ok(())
//!             });
//!         }
//!     }
//!     task::block_on(async {
//!         let listener = TcpListener::bind("localhost:8080").await?;
//!         eprintln!("Accepting connections on localhost:8080");
//!         let mut incoming = listener.incoming()
//!             .log_warnings(log_accept_error)
//!             .handle_errors(Duration::from_millis(500))
//!             .backpressure_wrapper(bp);
//!         while let Some(stream) = incoming.next().await {
//!             task::spawn(async {
//!                 if let Err(e) = connection_loop(stream).await {
//!                     eprintln!("Error: {}", e);
//!                 }
//!             });
//!         }
//!         Ok(())
//!     })
//! }
//!
//! async fn connection_loop(mut stream: ByteStream) -> Result<(), io::Error> {
//!     println!("Connected from {}", stream.peer_addr()?);
//!     task::sleep(Duration::from_secs(5)).await;
//!     stream.write_all("hello\n".as_bytes()).await?;
//!     Ok(())
//! }
//!
//! fn log_accept_error(e: &io::Error) {
//!     eprintln!("Accept error: {}. Sleeping 0.5s. {}", e, error_hint(&e));
//! }
//! ```
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod error;
mod listen_ext;
mod log;
mod sleep;
mod byte_stream;
pub mod backpressure;
pub mod wrapper_types;
pub mod errors;

pub use byte_stream::{ByteStream, PeerAddr};
pub use error::{is_transient_error, error_hint};
pub use listen_ext::ListenExt;
