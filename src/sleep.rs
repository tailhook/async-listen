use std::fmt;
use std::io;
use std::pin::Pin;
use std::time::Duration;

use async_std::future::Future;
use async_std::stream::Stream;
use async_std::task::{sleep, Context, Poll};

use crate::is_transient_error;

/// A stream adapter that retries on error
///
/// See
/// [`ListenExt::sleep_on_error`](../trait.ListenExt.html#method.sleep_on_error)
/// for more info.
pub struct HandleErrors<S> {
    stream: S,
    sleep_on_warning: Duration,
    timeout: Option<Pin<Box<dyn Future<Output=()> + 'static + Send>>>,
}

impl<S: fmt::Debug> fmt::Debug for HandleErrors<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("HandleErrors")
            .field("stream", &self.stream)
            .field("sleep_on_warning", &self.sleep_on_warning)
            .finish()
    }
}

impl<S: Unpin> Unpin for HandleErrors<S> {}

impl<S> HandleErrors<S> {
    pub(crate) fn new(stream: S, sleep_on_warning: Duration)
        -> HandleErrors<S>
    {
        HandleErrors { stream, sleep_on_warning, timeout: None }
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// adapter is pulling from.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Consumes this adapter, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<I, S> Stream for HandleErrors<S>
    where S: Stream<Item=Result<I, io::Error>> + Unpin,
{
    type Item = I;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<Self::Item>>
    {
        if let Some(ref mut to) = self.timeout {
            match to.as_mut().poll(cx) {
                Poll::Ready(_) => {}
                Poll::Pending => return Poll::Pending,
            }
        }
        self.timeout = None;
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(v))) => return Poll::Ready(Some(v)),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(Err(ref e)))
                if is_transient_error(e) => continue,
                Poll::Ready(Some(Err(_))) => {
                    let mut timeout = Box::pin(sleep(self.sleep_on_warning));
                    match timeout.as_mut().poll(cx) {
                        Poll::Pending => {
                            self.timeout = Some(timeout);
                            return Poll::Pending;
                        }
                        Poll::Ready(()) => continue,
                    }
                }
            }
        }
    }
}
