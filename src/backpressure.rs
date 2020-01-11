//! Backpressure handling structures
//!
//! The usual way to apply backpressure to a stream is using one of the
//! [`ListenExt`](../trait.ListenExt.html) trait methods:
//! * [`backpressure`](../trait.ListenExt.html#method.backpressure)
//! * [`apply_backpressure`](../trait.ListenExt.html#method.apply_backpressure)
//! * [`backpressure_wrapper`](../trait.ListenExt.html#method.backpressure_wrapper)
//!
//! Also take a look at [`backpressure::new`](fn.new.html) for the low-level
//! interface.
//!
use std::fmt;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, TryLockError};

use async_std::stream::Stream;
use async_std::future::Future;
use async_std::task::{Poll, Context, Waker};

use crate::byte_stream::ByteStream;


struct Inner {
    active: AtomicUsize,
    limit: AtomicUsize,
    task: Mutex<Option<Waker>>,
}

/// A stream combinator that applies backpressure
///
/// See
/// [`ListenExt::backpressure`](../trait.ListenExt.html#method.backpressure)
/// for more info.
pub struct BackpressureToken<S>(Backpressure<S>);

/// A stream combinator that applies backpressure and yields ByteStream
///
/// See
/// [`ListenExt::backpressure_wrapper`](../trait.ListenExt.html#method.backpressure_wrapper)
/// for more info.
pub struct BackpressureWrapper<S>(Backpressure<S>);

/// A stream combinator that applies backpressure and yields a token
///
/// See
/// [`ListenExt::apply_backpressure`](../trait.ListenExt.html#method.apply_backpressure)
/// for more info.
pub struct Backpressure<S> {
    stream: S,
    backpressure: Receiver,
}

/// The throttler of a stream
///
/// See [`new`](fn.new.html) for more details
pub struct Receiver {
    inner: Arc<Inner>,
}

/// Future that resolves when there is less that limit tokens alive
pub struct HasCapacity<'a> {
    recv: &'a mut Receiver,
}

/// The handle that controls backpressure
///
/// It can be used to create tokens, changing limit and getting metrics.
///
/// See [`new`](fn.new.html) for more details
#[derive(Clone)]
pub struct Sender {
    inner: Arc<Inner>,
}

/// The token which holds onto a single resource item
pub struct Token {
    inner: Arc<Inner>,
}

impl<S: Unpin> Unpin for Backpressure<S> {}
impl<S: Unpin> Unpin for BackpressureToken<S> {}
impl<S: Unpin> Unpin for BackpressureWrapper<S> {}

impl Sender {
    /// Acquire a backpressure token
    ///
    /// The token holds one unit of resource
    ///
    /// *Note:* You can always acquire a token, even if capacity limit reached.
    pub fn token(&self) -> Token {
        self.inner.active.fetch_add(1, Ordering::SeqCst);
        Token {
            inner: self.inner.clone(),
        }
    }
    /// Change the limit for the number of connections
    ///
    /// If limit is increased it's applied immediately. If limit is lowered,
    /// we can't drop connections. So listening stream is paused until
    /// there are less then new limit tokens alive (i.e. first dropped
    /// tokens may not unblock the stream).
    pub fn set_limit(&self, new_limit: usize) {
        let old_limit = self.inner.limit.swap(new_limit, Ordering::SeqCst);
        if old_limit < new_limit {
            match self.inner.task.try_lock() {
                Ok(mut guard) => {
                    guard.take().map(|w| w.wake());
                }
                Err(TryLockError::WouldBlock) => {
                    // This means either another token is currently waking
                    // up a Receiver. Or Receiver is currently running.
                    // Receiver will recheck values after releasing the Mutex.
                }
                Err(TryLockError::Poisoned(_)) => {
                    unreachable!("backpressure lock should never be poisoned");
                }
            }
        }
    }

    /// Returns the number of currently active tokens
    ///
    /// Can return a value larger than limit if tokens are created manually.
    ///
    /// This can be used for metrics or debugging. You should not rely on
    /// this value being in sync. There is also no way to wake-up when this
    /// value is lower than limit, also see
    /// [`has_capacity`](struct.Receiver.html#method.has_capacity).
    pub fn get_active_tokens(&self) -> usize {
        self.inner.active.load(Ordering::Relaxed)
    }
}

impl Receiver {
    /// Handy to create token in Backpressure wrapper
    fn token(&self) -> Token {
        self.inner.active.fetch_add(1, Ordering::SeqCst);
        Token {
            inner: self.inner.clone(),
        }
    }

    /// Return future which resolves when the current number active of tokens
    /// is less than a limit
    ///
    /// If you create tokens in different task than the task that waits
    /// on `HasCapacity` there is a race condition.
    pub fn has_capacity(&mut self) -> HasCapacity {
        HasCapacity { recv: self }
    }

    fn poll(&mut self, cx: &mut Context) -> Poll<()> {
        loop {
            let active = self.inner.active.load(Ordering::Acquire);
            let limit = self.inner.limit.load(Ordering::Acquire);
            if active < limit {
                break;
            }
            match self.inner.task.try_lock() {
                Ok(mut guard) => {
                    *guard = Some(cx.waker().clone());
                    return Poll::Pending;
                }
                Err(TryLockError::WouldBlock) => {
                    // This means either another token is currently waking
                    // up this receiver, retry
                    //
                    // Note: this looks like a busyloop, but we don't have
                    // anything long/slow behind the mutex. And it's only
                    // executed when limit is reached.
                    continue;
                }
                Err(TryLockError::Poisoned(_)) => {
                    unreachable!("backpressure lock should never be poisoned");
                }
            }
        }
        return Poll::Ready(());
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        // TODO(tailhook) we could use Acquire for old_ref,
        // but not sure how safe is it to compare it with a limit
        let old_ref = self.inner.active.fetch_sub(1, Ordering::SeqCst);
        let limit = self.inner.limit.load(Ordering::SeqCst);
        if old_ref == limit {
            match self.inner.task.try_lock() {
                Ok(mut guard) => {
                    guard.take().map(|w| w.wake());
                }
                Err(TryLockError::WouldBlock) => {
                    // This means either another token is currently waking
                    // up a Receiver. Or Receiver is currently running.
                    // Receiver will recheck values after releasing the Mutex.
                }
                Err(TryLockError::Poisoned(_)) => {
                    unreachable!("backpressure lock should never be poisoned");
                }
            }
        }
    }
}

impl<S> BackpressureToken<S> {
    pub(crate) fn new(stream: S, backpressure: Receiver)
        -> BackpressureToken<S>
    {
        BackpressureToken(Backpressure::new(stream, backpressure))
    }

    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &S {
        self.0.get_ref()
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    pub fn get_mut(&mut self) -> &mut S {
        self.0.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying stream that this
    /// combinator is pulling from.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe {
            self.map_unchecked_mut(|x| &mut x.0.stream)
        }
    }

    /// Consumes this combinator, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.0.into_inner()
    }
}

impl<S> BackpressureWrapper<S> {
    pub(crate) fn new(stream: S, backpressure: Receiver)
        -> BackpressureWrapper<S>
    {
        BackpressureWrapper(Backpressure::new(stream, backpressure))
    }

    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &S {
        self.0.get_ref()
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    pub fn get_mut(&mut self) -> &mut S {
        self.0.get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying stream that this
    /// combinator is pulling from.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe {
            self.map_unchecked_mut(|x| &mut x.0.stream)
        }
    }

    /// Consumes this combinator, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.0.into_inner()
    }
}

impl<S> Backpressure<S> {
    pub(crate) fn new(stream: S, backpressure: Receiver) -> Backpressure<S> {
        Backpressure { stream, backpressure }
    }

    /// Acquires a reference to the underlying stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// combinator is pulling from.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Acquires a pinned mutable reference to the underlying stream that this
    /// combinator is pulling from.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe {
            self.map_unchecked_mut(|x| &mut x.stream)
        }
    }

    /// Consumes this combinator, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

/// Create a new pair of backpressure structures
///
/// These structures are called [`Sender`](struct.Sender.html)
/// and [`Receiver`](struct.Receiver.html) similar to channels.
/// The `Receiver` should be used to throttle, either by applying
/// it to a stream or using it directly. The `Sender` is a way to create
/// throtting tokens (the stream is paused when there are tokens >= limit),
/// and to change the limit.
///
/// See [`ListenExt`](../trait.ListenExt.html) for example usage
///
/// # Direct Use Example
///
/// ```no_run
/// # use std::time::Duration;
/// # use async_std::net::{TcpListener, TcpStream};
/// # use async_std::prelude::*;
/// # use async_std::task;
/// # fn main() -> std::io::Result<()> { task::block_on(async {
/// #
/// use async_listen::ListenExt;
/// use async_listen::backpressure;
///
/// let listener = TcpListener::bind("127.0.0.1:0").await?;
/// let (tx, mut rx) = backpressure::new(10);
/// let mut incoming = listener.incoming()
///     .handle_errors(Duration::from_millis(100));
///
/// loop {
///     rx.has_capacity().await;
///     let conn = match incoming.next().await {
///         Some(conn) => conn,
///         None => break,
///     };
///     let token = tx.token();  // should be created before spawn
///     task::spawn(async {
///         connection_loop(conn).await;
///         drop(token);  // should be dropped after
///     });
/// }
/// # async fn connection_loop(_stream: TcpStream) {
/// # }
/// #
/// # Ok(()) }) }
/// ```
///
pub fn new(initial_limit: usize) -> (Sender, Receiver) {
    let inner = Arc::new(Inner {
        limit: AtomicUsize::new(initial_limit),
        active: AtomicUsize::new(0),
        task: Mutex::new(None),
    });
    return (
        Sender {
            inner: inner.clone(),
        },
        Receiver {
            inner: inner.clone(),
        },
    )
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug("Token", &self.inner, f)
    }
}

impl fmt::Debug for Sender {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug("Sender", &self.inner, f)
    }
}

impl fmt::Debug for Receiver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug("Receiver", &self.inner, f)
    }
}

impl<'a> fmt::Debug for HasCapacity<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        debug("HasCapacity", &self.recv.inner, f)
    }
}

fn debug(name: &str, inner: &Arc<Inner>, f: &mut fmt::Formatter)
    -> fmt::Result
{
    let active = inner.active.load(Ordering::Relaxed);
    let limit = inner.limit.load(Ordering::Relaxed);
    write!(f, "<{} {}/{}>", name, active, limit)
}

impl<S: fmt::Debug> fmt::Debug for Backpressure<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Backpressure")
            .field("stream", &self.stream)
            .field("backpressure", &self.backpressure)
            .finish()
    }
}

impl<S: fmt::Debug> fmt::Debug for BackpressureToken<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BackpressureToken")
            .field("stream", &self.0.stream)
            .field("backpressure", &self.0.backpressure)
            .finish()
    }
}

impl<S: fmt::Debug> fmt::Debug for BackpressureWrapper<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BackpressureWrapper")
            .field("stream", &self.0.stream)
            .field("backpressure", &self.0.backpressure)
            .finish()
    }
}

impl<I, S> Stream for Backpressure<S>
    where S: Stream<Item=I> + Unpin
{
    type Item = I;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<Self::Item>>
    {
        match self.backpressure.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => self.as_mut().get_pin_mut().poll_next(cx),
        }
    }
}

impl<'a> Future for HasCapacity<'a> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        self.recv.poll(cx)
    }
}

impl<I, S> Stream for BackpressureToken<S>
    where S: Stream<Item=I> + Unpin
{
    type Item = (Token, I);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<Self::Item>>
    {
        unsafe { self.as_mut().map_unchecked_mut(|x| &mut x.0) }
        .poll_next(cx)
        .map(|opt| opt.map(|conn| (self.0.backpressure.token(), conn)))
    }
}

impl<I, S> Stream for BackpressureWrapper<S>
    where S: Stream<Item=I> + Unpin,
          ByteStream: From<(Token, I)>,
{
    type Item = ByteStream;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<Self::Item>>
    {
        unsafe { self.as_mut().map_unchecked_mut(|x| &mut x.0) }
        .poll_next(cx)
        .map(|opt| opt.map(|conn| {
            ByteStream::from((self.0.backpressure.token(), conn))
        }))
    }
}
