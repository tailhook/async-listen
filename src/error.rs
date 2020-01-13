#![deny(meta_variable_misuse)]

use std::fmt;
use std::io;

/// Error hint that can be formatted
///
/// The structure implements `Display` and is usually used in logs like this:
/// ```
/// # use std::io;
/// # let e: io::Error = io::ErrorKind::Other.into();
/// use async_listen::error_hint;
/// eprintln!("Error: {}. {}", e, error_hint(&e));
/// ```
///
/// See [error description](../errors/index.html) for a list of the errors that
/// can return a hint.
///
/// You can also apply a custom formatting (i.e. a different link) for the
/// hint. Just replace an `error_hint` function with something of your own:
///
/// ```
/// # use std::io;
/// fn error_hint(e: &io::Error) -> String {
///     let hint = async_listen::error_hint(e);
///     if hint.is_empty() {
///         return String::new();
///     } else {
///         return format!("{} http://example.org/my-server-errors#{}",
///             hint.hint_text(), hint.link_hash());
///     }
/// }
/// ```
#[derive(Debug)]
pub struct ErrorHint {
    error: Option<KnownError>
}

#[derive(Debug)]
enum KnownError {
    Enfile,
    Emfile,
}

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
///
/// This function is most likely should not be used directly, but rather
/// through one of the following combinators:
/// * [`log_warnings`](trait.ListenExt.html#method.log_warnings)
/// * [`handle_errors`](trait.ListenExt.html#method.handle_errors)
pub fn is_transient_error(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::ConnectionRefused ||
    e.kind() == io::ErrorKind::ConnectionAborted ||
    e.kind() == io::ErrorKind::ConnectionReset
}

macro_rules! error_match {
    ($value:expr => {
        $(
        ($n: pat | wasi: $wasi: pat | haiku: $haiku:pat) => $val: ident,
        )*
    }) => {
        match $value {$(
            #[cfg(any(target_env="wasi", target_os="wasi"))]
            Some($wasi) => Some($val),
            #[cfg(target_os="haiku")]
            Some($haiku) => Some($val),
            #[cfg(all(
                any(unix, windows, target_os="fuchsia"),
                not(any(target_env="wasi", target_os="wasi",target_os="haiku"))
            ))]
            Some($n) => Some($val),
        )*
            _ => None,
        }
    }
}

/// Returns a hint structure that can be formatter to the log output
///
/// # Example
/// ```
/// # use std::io;
/// # let e: io::Error = io::ErrorKind::Other.into();
/// use async_listen::error_hint;
/// eprintln!("Error: {}. {}", e, error_hint(&e));
/// ```
///
/// Error message might look like:
/// ```
/// Error: Too many open files (os error 24). Increase per-process open file limit https://big.ly/async-err#EMFILE
/// ```
///
/// See [error description](errors/index.html) for a list of the errors that
/// can return a hint.
///
/// See [`ErrorHint`] for more info on customizing the output
///
/// [`ErrorHint`]: wrapper_types/struct.ErrorHint.html
pub fn error_hint(e: &io::Error) -> ErrorHint {
    use KnownError::*;
    let error = error_match!(e.raw_os_error() => {
        (24 | wasi: 33 | haiku: -2147459062) => Emfile,
        (23 | wasi: 41 | haiku: -2147454970) => Enfile,
    });
    return ErrorHint { error }
}


impl ErrorHint {
    /// Text of the hint
    ///
    /// Since the text is expected to be printed **after** the error message,
    /// it usually includes call to action, like:
    /// ```text
    /// Increase per-process open file limit
    /// ```
    ///
    /// Usually the hint is good enough to use search engine to find the
    /// solution to the problem. But usually link is printed too.
    pub fn hint_text(&self) -> &'static str {
        use KnownError::*;
        match &self.error {
            None => "",
            Some(Emfile) => "Increase per-process open file limit",
            Some(Enfile) => "Increase system open file limit",
        }
    }

    /// The part of the link after the hash `#` sign
    ///
    /// To make a link prepend with the base URL:
    /// ```
    /// # let h = async_listen::error_hint(&std::io::ErrorKind::Other.into());
    /// println!("{}#{}", h.default_link_base(), h.link_hash())
    /// ```
    ///
    /// It's expected that implementation may customize base link. Mathing
    /// for the link hash is also well supported, for exaple if you want to
    /// change the link only for one of few errors.
    ///
    /// Link hashes are stable (we don't change them in future versions).
    pub fn link_hash(&self) -> &'static str {
        use KnownError::*;
        match &self.error {
            None => "",
            Some(Emfile) => "EMFILE",
            Some(Enfile) => "ENFILE",
        }
    }

    /// Returns current base link printed with the hint
    ///
    /// Current value is `https://big.ly/async-err`. In future versions we
    /// might change the base if we find a better place to host the docs, but
    /// we don't take this decision lightly.
    ///
    /// To make a link, just append a hash part:
    /// ```no_run
    /// # let h = async_listen::error_hint(&std::io::ErrorKind::Other.into());
    /// println!("{}#{}", h.default_link_base(), h.link_hash())
    /// ```
    pub fn default_link_base(&self) -> &'static str {
        return "https://big.ly/async-err";
    }

    /// Returns true if the hint is empty
    ///
    /// Even if there is no hint for the error (error code is unknown)
    /// the `error_hint` function returns the `ErrorHint` object which is
    /// empty when displayed. This is a convenience in most cases, but you
    /// have to check for `is_empty` when formatting your own hint.
    pub fn is_empty(&self) -> bool {
        self.error.is_some()
    }
}

impl fmt::Display for ErrorHint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.error.is_none() {
            return Ok(())
        }
        write!(f, "{} {}#{}",
            self.hint_text(), self.default_link_base(), self.link_hash())
    }
}
