//! # Async Server
//!
//! The crate contains various helpers for writing production-ready servers in
//! rust using [async-std](https://async.rs/).
//!
//! [Docs](https://docs.rs/async-server/) |
//! [Github](https://github.com/tailhook/async-server/) |
//! [Crate](https://crates.io/crates/async-server)
//!
//! ## Low-Level Utilities
//!
//! * [is_trasient_error](#fn.is_transient_error) -- determines if the
//!   error returned from `accept()` can be ignored
//!
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]

mod error;

pub use error::is_transient_error;
