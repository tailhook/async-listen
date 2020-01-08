use std::io;

/// Returns true if the error is transient
///
/// The transient error is defined here as an error after which we can continue
/// accepting subsequent connections without the risk of a tight loop.
///
/// For example, a per-connection error like `ConnectionReset` in `accept()`
/// system call means then next connection might be ready to be accepted
/// immediately.
///
/// All other errors should incur a timeout before the next `accept()` is
/// performed.  The timeout is useful to handle resource exhaustion errors
/// like ENFILE and EMFILE: file descriptor might be released after some time
/// but the error will be the same if we continue to accept in a tight loop.
pub fn is_transient_error(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::ConnectionRefused ||
    e.kind() == io::ErrorKind::ConnectionAborted ||
    e.kind() == io::ErrorKind::ConnectionReset
}
