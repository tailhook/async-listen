use std::io;
use std::fmt;
use std::pin::Pin;

use async_std::stream::Stream;
use async_std::task::{Poll, Context};

use crate::is_transient_error;

/// A stream adapter that logs errors which aren't transient
///
/// See
/// [`ListenExt::log_warnings`](../trait.ListenExt.html#method.log_warnings)
/// for more info.
pub struct LogWarnings<S, F> {
    stream: S,
    logger: F,
}

impl<S: fmt::Debug, F> fmt::Debug for LogWarnings<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LogWarnings")
            .field("stream", &self.stream)
            .finish()
    }
}

impl<S: Unpin, F> Unpin for LogWarnings<S, F> {}

impl<S, F> LogWarnings<S, F> {
    pub(crate) fn new(stream: S, f: F) -> LogWarnings<S, F> {
        LogWarnings {
            stream,
            logger: f,
        }
    }

    /// Acquires a reference to the underlying stream that this adapter is
    /// pulling from.
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// Acquires a mutable reference to the underlying stream that this
    /// adapter is pulling from.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Acquires a pinned mutable reference to the underlying stream that this
    /// adapter is pulling from.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe {
            self.map_unchecked_mut(|x| &mut x.stream)
        }
    }

    /// Consumes this adapter, returning the underlying stream.
    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<I, S, F> Stream for LogWarnings<S, F>
    where S: Stream<Item=Result<I, io::Error>> + Unpin,
          F: FnMut(&io::Error),
{
    type Item = Result<I, io::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Option<Self::Item>>
    {
        let res = self.as_mut().get_pin_mut().poll_next(cx);
        match &res {
            Poll::Ready(Some(Err(e))) if !is_transient_error(e)
            => (self.get_mut().logger)(e),
            _ => {}
        };
        return res;
    }
}
