use std::io;
use std::time::Duration;

use async_std::stream::Stream;

use crate::log;
use crate::sleep;


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

    /// Handle errors and return infallible stream
    ///
    /// There are two types of errors:
    ///
    /// * [`transient`](fn.is_transient_error.html) which may be ignored
    /// * warnings which keep socket in accept queue after the error
    ///
    /// We ignore transient errors entirely, and timeout for a `sleep_amount`
    /// on stick ones.
    ///
    /// One example of warning is `EMFILE: too many open files`. In this
    /// case, if we sleep for some amount, so there is a chance that other
    /// connection or some file descriptor is closed in the meantime and we
    /// can accept another connection.
    ///
    /// Also in the case of warnings, it's usually a good idea to log them
    /// (i.e. so file descritor limit or max connection is adjusted by user).
    /// Use [`log_warnings`](#method.log_warnings) to do this.
    ///
    /// `while let Some(s) = stream.handle_errors(d).next().await {...}`
    /// is equivalent of:
    ///
    /// ```ignore
    /// while let Some(res) = stream.next().await? {
    ///     let s = match res {
    ///         Ok(s) => s,
    ///         Err(e) => {
    ///             if !is_traisient_error(e) {
    ///                 task::sleep(d);
    ///             }
    ///             continue;
    ///         }
    ///     };
    ///     # ...
    /// }
    /// ```
    ///
    /// # Example
    ///
    fn handle_errors<I>(self, sleep_on_warning: Duration)
        -> sleep::HandleErrors<Self>
        where Self: Stream<Item=Result<I, io::Error>> + Sized,
    {
        sleep::HandleErrors::new(self, sleep_on_warning)
    }
}

impl<T: Stream> ListenExt for T {}
