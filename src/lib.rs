//! # Async Server
//!
//! The crate contains various helpers for writing production-ready servers in
//! rust using [async-std](https://async.rs/).
//!
//! [Docs](https://docs.rs/async-server/) |
//! [Github](https://github.com/tailhook/async-server/) |
//! [Crate](https://crates.io/crates/async-server)
//!
//! ## Utilities
//! * [ListenExt](trait.ListenExt.html) -- extension trait for stream of
//!   accepted sockets, provides useful conbinators for a stream
//!
//! ## Low-Level Utilities
//!
//! * [is_transient_error](fn.is_transient_error.html) -- determines if the
//!   error returned from `accept()` can be ignored
//!
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

mod error;
mod listen_ext;
mod log;
mod sleep;
pub mod wrapper_types;

pub use error::is_transient_error;
pub use listen_ext::ListenExt;
