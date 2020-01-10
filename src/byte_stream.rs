use std::io;
use std::pin::Pin;
use std::task::{Poll, Context};

use futures::io::{AsyncRead, AsyncWrite, IoSlice, IoSliceMut};
use async_std::net::TcpStream;
#[cfg(unix)] use async_std::os::unix::net::UnixStream;

use crate::backpressure::Token;


#[derive(Debug)]
enum Stream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}


/// A wrapper around TcpStream and UnixStream
///
/// This structure is yielded by the stream created by
/// [`ListenExt::backpressure_wrapper`](trait.ListenExt.html#method.backpressure_wrapper)
///
/// This wrapper serves two purposes:
///
/// 1. Holds backpressure token
/// 2. Abstract away differences between TcpStream and UnixStream
///
/// The structure implements AsyncRead and AsyncWrite so can be used for
/// protocol implementation directly.
#[derive(Debug)]
pub struct ByteStream {
    stream: Stream,
    token: Token,
}

trait Assert: AsyncRead + AsyncWrite + Send + Unpin + 'static { }
impl Assert for ByteStream {}

impl ByteStream {
    /// Create a bytestream for a tcp socket
    pub fn new_tcp(token: Token, stream: TcpStream) -> ByteStream {
        ByteStream {
            stream: Stream::Tcp(stream),
            token,
        }
    }

    /// Create a bytestream for a unix socket
    pub fn new_unix(token: Token, stream: UnixStream) -> ByteStream {
        ByteStream {
            stream: Stream::Unix(stream),
            token,
        }
    }
}

impl From<(Token, TcpStream)> for ByteStream {
    fn from((token, stream): (Token, TcpStream)) -> ByteStream {
        ByteStream::new_tcp(token, stream)
    }
}

impl From<(Token, UnixStream)> for ByteStream {
    fn from((token, stream): (Token, UnixStream)) -> ByteStream {
        ByteStream::new_unix(token, stream)
    }
}

impl AsyncRead for ByteStream {

    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8])
        -> Poll<Result<usize, io::Error>>
    {
        match &mut self.stream {
            Stream::Tcp(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_read(cx, buf)
            }
            #[cfg(unix)]
            Stream::Unix(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_read(cx, buf)
            }
        }
    }

    fn poll_read_vectored(mut self: Pin<&mut Self>, cx: &mut Context,
        bufs: &mut [IoSliceMut])
        -> Poll<Result<usize, io::Error>>
    {
        match &mut self.stream {
            Stream::Tcp(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_read_vectored(cx, bufs)
            }
            #[cfg(unix)]
            Stream::Unix(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_read_vectored(cx, bufs)
            }
        }
    }
}

impl AsyncWrite for ByteStream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8])
        -> Poll<Result<usize, io::Error>>
    {
        match &mut self.stream {
            Stream::Tcp(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_write(cx, buf)
            }
            #[cfg(unix)]
            Stream::Unix(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_write(cx, buf)
            }
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Result<(), io::Error>>
    {
        match &mut self.stream {
            Stream::Tcp(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_flush(cx)
            }
            #[cfg(unix)]
            Stream::Unix(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_flush(cx)
            }
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Result<(), io::Error>>
    {
        match &mut self.stream {
            Stream::Tcp(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_close(cx)
            }
            #[cfg(unix)]
            Stream::Unix(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_close(cx)
            }
        }
    }
    fn poll_write_vectored(mut self: Pin<&mut Self>, cx: &mut Context,
        bufs: &[IoSlice])
        -> Poll<Result<usize, io::Error>>
    {
        match &mut self.stream {
            Stream::Tcp(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_write_vectored(cx, bufs)
            }
            #[cfg(unix)]
            Stream::Unix(s) => {
                unsafe { Pin::new_unchecked(s) }.poll_write_vectored(cx, bufs)
            }
        }
    }
}
