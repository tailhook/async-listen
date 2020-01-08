use std::io;

use async_std::stream::Stream;

use crate::log;


/// An extension trait that provides necessary combinators for turning
/// a stream of `accept()` events into a full-featured connection listener
///
pub trait ListenExt: Stream {
    /// Log errors which aren't transient using user-specified function
    ///
    /// The the warning in this context is any error which isn't transient.
    /// There are no fatal errors (ones which don't allow listener to
    /// procceed in the future) on any known platform so any error is
    /// considered a warning. See
    /// [`is_transient_error`](fn.is_transient_error.html) for more info.
    ///
    /// `stream.log_warnings(user_func)` is equivalent of:
    ///
    /// ```ignore
    /// stream.inspect(|res| res.map_err(|e| {
    ///     if !is_transient_error(e) {
    ///         user_func(e);
    ///     }
    /// })
    /// ```
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use async_std::net::TcpListener;
    /// # use async_std::prelude::*;
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_server::ListenExt;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// let mut incoming = listener.incoming()
    ///     .log_warnings(|e| eprintln!("Listening error: {}", e));
    ///
    /// while let Some(stream) = incoming.next().await {
    ///     // ...
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    fn log_warnings<I, F>(self, f: F)
        -> log::LogWarnings<Self, F>
        where Self: Stream<Item=Result<I, io::Error>> + Sized,
              F: FnMut(&io::Error),
    {
        log::LogWarnings::new(self, f)
    }
}

impl<T: Stream> ListenExt for T {}
