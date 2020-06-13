use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Poll, Context};

use async_std::io::{Read, Write, IoSlice, IoSliceMut};
use async_std::net::{TcpStream, Shutdown};
#[cfg(unix)] use async_std::os::unix::net::UnixStream;

use crate::backpressure::Token;


#[derive(Debug, Clone)]
enum Stream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}

/// A peer address for either Tcp or Unix socket
///
/// This enum is returned by
/// [`ByteStream::peer_addr`](struct.ByteStream.html#method.peer_addr).
///
///
/// The enum contains `Unix` option even on platforms that don't support
/// unix sockets (Windows) to make code easier to write (less `#[cfg(unix)]`
/// attributes all over the code).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PeerAddr {
    /// The peer address is TCP socket address.
    Tcp(SocketAddr),
    /// The peer address is Unix socket path. `None` if socket is unnamed.
    Unix(Option<PathBuf>),
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
#[derive(Debug, Clone)]
pub struct ByteStream {
    stream: Stream,
    token: Option<Token>,
}

trait Assert: Read + Write + Send + Unpin + 'static { }
impl Assert for ByteStream {}

impl fmt::Display for PeerAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PeerAddr::Tcp(s) => s.fmt(f),
            PeerAddr::Unix(None) => "<unnamed>".fmt(f),
            PeerAddr::Unix(Some(s)) => s.display().fmt(f),
        }
    }
}

impl ByteStream {
    /// Create a bytestream for a tcp socket
    pub fn new_tcp(token: Token, stream: TcpStream) -> ByteStream {
        ByteStream {
            stream: Stream::Tcp(stream),
            token: Some(token),
        }
    }

    /// Create a bytestream for a tcp socket (without token)
    ///
    /// This can be used with interfaces that require a `ByteStream` but
    /// aren't got from the listener that have backpressure applied. For
    /// example, if you have two listeners in the single app or even for
    /// client connections.
    pub fn new_tcp_detached(stream: TcpStream) -> ByteStream {
        ByteStream {
            stream: Stream::Tcp(stream),
            token: None,
        }
    }

    /// Create a bytestream for a unix socket
    #[cfg(unix)]
    pub fn new_unix(token: Token, stream: UnixStream) -> ByteStream {
        ByteStream {
            stream: Stream::Unix(stream),
            token: Some(token),
        }
    }

    /// Create a bytestream for a unix socket (without token)
    ///
    /// This can be used with interfaces that require a `ByteStream` but
    /// aren't got from the listener that have backpressure applied. For
    /// example, if you have two listeners in the single app or even for
    /// client connections.
    #[cfg(unix)]
    pub fn new_unix_detached(stream: UnixStream) -> ByteStream {
        ByteStream {
            stream: Stream::Unix(stream),
            token: None,
        }
    }

    /// Returns the remote address that this stream is connected to.
    ///
    /// Note: even on non-unix platforms (Windows)
    /// [`PeerAddr`](enum.PeerAddr.html) still contains `Unix` option so you
    /// don't have to use conditional compilation when matching.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let peer = stream.peer_addr()?;
    /// match peer.peer_addr()? {
    ///     PeerAddr::Tcp(addr) => println!("TCP addr {}", addr),
    ///     PeerAddr::Unix(None) => println!("Unnamed unix socket"),
    ///     PeerAddr::Unix(Some(path)) => println!("Unix {}", path.display()),
    /// }
    /// ```
    pub fn peer_addr(&self) -> io::Result<PeerAddr> {
        match &self.stream {
            Stream::Tcp(s) => s.peer_addr().map(PeerAddr::Tcp),
            #[cfg(unix)]
            Stream::Unix(s) => {
                s.peer_addr()
                .map(|a| a.as_pathname().map(|p| p.to_owned()))
                .map(PeerAddr::Unix)
            }
        }
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// For Unix sockets this function always returns true (Unix sockets
    /// always behave like the option is off).
    ///
    /// For more information about this option, see [`set_nodelay`].
    ///
    /// [`set_nodelay`]: #method.set_nodelay
    pub fn nodelay(&self) -> io::Result<bool> {
        match &self.stream {
            Stream::Tcp(s) => s.nodelay(),
            #[cfg(unix)]
            Stream::Unix(_) => Ok(true),
        }
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that
    /// segments are always sent as soon as possible, even if there is only a
    /// small amount of data. When not set, data is buffered until there is a
    /// sufficient amount to send out, thereby avoiding the frequent sending of
    /// small packets.
    ///
    /// For Unix sockets this function does nothing (Unix sockets always behave
    /// like the option is enabled, and there is no way to change that).
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        match &self.stream {
            Stream::Tcp(s) => s.set_nodelay(nodelay),
            #[cfg(unix)]
            Stream::Unix(_) => Ok(()),
        }
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the
    /// specified portions to immediately return with an appropriate value
    /// (see the documentation of Shutdown).
    pub fn shutdown(&self, how: Shutdown) -> Result<(), io::Error> {
        match &self.stream {
            Stream::Tcp(s) => s.shutdown(how),
            #[cfg(unix)]
            Stream::Unix(s) => s.shutdown(how),
        }
    }
}

impl From<(Token, TcpStream)> for ByteStream {
    fn from((token, stream): (Token, TcpStream)) -> ByteStream {
        ByteStream::new_tcp(token, stream)
    }
}

#[cfg(unix)]
impl From<(Token, UnixStream)> for ByteStream {
    fn from((token, stream): (Token, UnixStream)) -> ByteStream {
        ByteStream::new_unix(token, stream)
    }
}

impl Read for ByteStream {

    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_read(cx, buf)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_read(cx, buf)
            }
        }
    }

    fn poll_read_vectored(self: Pin<&mut Self>, cx: &mut Context,
        bufs: &mut [IoSliceMut])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_read_vectored(cx, bufs)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_read_vectored(cx, bufs)
            }
        }
    }
}

impl Read for &ByteStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_read(cx, buf)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_read(cx, buf)
            }
        }
    }
    fn poll_read_vectored(self: Pin<&mut Self>, cx: &mut Context,
        bufs: &mut [IoSliceMut])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_read_vectored(cx, bufs)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_read_vectored(cx, bufs)
            }
        }
    }
}

impl Write for ByteStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_write(cx, buf)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_write(cx, buf)
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Result<(), io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_flush(cx)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_flush(cx)
            }
        }
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Result<(), io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_close(cx)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_close(cx)
            }
        }
    }
    fn poll_write_vectored(self: Pin<&mut Self>, cx: &mut Context,
        bufs: &[IoSlice])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_write_vectored(cx, bufs)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_write_vectored(cx, bufs)
            }
        }
    }
}

impl Write for &ByteStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_write(cx, buf)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_write(cx, buf)
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Result<(), io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_flush(cx)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_flush(cx)
            }
        }
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context)
        -> Poll<Result<(), io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_close(cx)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_close(cx)
            }
        }
    }
    fn poll_write_vectored(self: Pin<&mut Self>, cx: &mut Context,
        bufs: &[IoSlice])
        -> Poll<Result<usize, io::Error>>
    {
        match self.stream {
            Stream::Tcp(ref s) => {
                Pin::new(&mut &*s).poll_write_vectored(cx, bufs)
            }
            #[cfg(unix)]
            Stream::Unix(ref s) => {
                Pin::new(&mut &*s).poll_write_vectored(cx, bufs)
            }
        }
    }
}
