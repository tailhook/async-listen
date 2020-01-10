use std::io;
use std::time::Duration;

use async_std::stream::Stream;

use crate::log;
use crate::sleep;
use crate::backpressure;


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

    /// Apply a fixed backpressure to the the stream
    ///
    /// The output stream yields pairs of (token, stream). The token must
    /// be kept alive as long as connection is still alive.
    ///
    /// `stream.backpressure(10)` is equivalent of:
    /// ```ignore
    /// let (tx, rx) = backpressure::new(10);
    /// stream
    ///     .apply_backpressure(rx)
    ///     .map(|conn| (tx.token(), conn))
    /// ```
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use async_std::net::{TcpListener, TcpStream};
    /// # use async_std::prelude::*;
    /// # use async_std::task;
    /// # fn main() -> std::io::Result<()> { task::block_on(async {
    /// #
    /// use async_server::ListenExt;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// let mut incoming = listener.incoming()
    ///     .handle_errors(Duration::from_millis(100))
    ///     .backpressure(100);
    ///
    /// while let Some((token, stream)) = incoming.next().await {
    ///     task::spawn(async {
    ///         connection_loop(stream).await;
    ///         drop(token);
    ///     });
    /// }
    /// # async fn connection_loop(_stream: TcpStream) {
    /// # }
    /// #
    /// # Ok(()) }) }
    /// ```
    ///
    /// *Note:* the `drop` there is not needed you can use either:
    ///
    /// * `let _token = token;` inside `async` block, or
    /// * `connection_loop(&token, stream)`,
    ///
    /// To achieve the same result. But `drop(token)` makes it explicit that
    /// token is dropped only at that point, which is an important property to
    /// achieve.
    fn backpressure<I>(self, limit: usize)
        -> backpressure::BackpressureToken<Self>
        where Self: Stream<Item=I> + Sized,
    {
        let (_tx, rx) = backpressure::new(limit);
        return backpressure::BackpressureToken::new(self, rx);
    }

    /// Apply a backpressure object to a stream
    ///
    /// This method is different from [`backpressure`](#method.backpressure) in
    /// two ways:
    ///
    /// 1. It doesn't modify stream output
    /// 2. External backpressure object may be used to change limit at runtime
    ///
    /// With the greater power comes greater responsibility, though. Here are
    /// some things to remember when using the method:
    ///
    /// 1. You must create a token for each connection (see example).
    /// 2. Token *should* be created before yielding to a main loop, otherwise
    ///    limit can be exhausted at times.
    /// 2. Token should be kept alive as long as the connection is alive.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::time::Duration;
    /// # use async_std::net::{TcpListener, TcpStream};
    /// # use async_std::prelude::*;
    /// # use async_std::task;
    /// # fn main() -> std::io::Result<()> { task::block_on(async {
    /// #
    /// use async_server::ListenExt;
    /// use async_server::backpressure;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// let (tx, rx) = backpressure::new(10);
    /// let mut incoming = listener.incoming()
    ///     .handle_errors(Duration::from_millis(100))
    ///     .apply_backpressure(rx);
    ///
    /// while let Some(stream) = incoming.next().await {
    ///     let token = tx.token();  // should be created before spawn
    ///     task::spawn(async {
    ///         connection_loop(stream).await;
    ///         drop(token);  // should be dropped after
    ///     });
    /// }
    /// # async fn connection_loop(_stream: TcpStream) {
    /// # }
    /// #
    /// # Ok(()) }) }
    /// ```
    ///
    /// *Note:* the `drop` there is not needed you can use either:
    ///
    /// * `let _token = token;` inside `async` block, or
    /// * `connection_loop(&token, stream)`,
    ///
    /// To achieve the same result. But `drop(token)` makes it explicit that
    /// token is dropped only at that point, which is an important property to
    /// achieve. Also don't create token in async block as it makes
    /// backpressure enforcing unreliable.
    fn apply_backpressure<I>(self, backpressure: backpressure::Receiver)
        -> backpressure::Backpressure<Self>
        where Self: Stream<Item=I> + Sized,
    {
        return backpressure::Backpressure::new(self, backpressure);
    }
}

impl<T: Stream> ListenExt for T {}
