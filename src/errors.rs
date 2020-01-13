//! # Documentation of the Error Hints
//!
//! This page is the destination of the links shown by
//! [`error_hint`](../fn.error_hint.html).
//!
//! List of errors having a hint:
//!
//! * [Too many open files](#EMFILE) / [EMFILE](#EMFILE)
//! * [Too many open files in system](#ENFILE) / [ENFILE](#ENFILE)
//!
//!
//! # Too Many Open Files <a name='EMFILE'></a>
//!
//! | Posix Name | EMFILE |
//! |---|---|
//! | Message | `Too many open files (os error 24)` |
//! | Hint | `Increase per-process open file limit` |
//! | Link | `https://big.ly/async-err#EMFILE` |
//!
//! ## Common Causes
//!
//! 1. File descriptor limit for the process is too low
//! 2. Limit of number of simultaneous connections is either too high or
//!    unimplemented
//!
//! The (2) can be fixed by applying [`backpressure`] abstraction from this
//! crate.
//!
//! The rest of this section discusses how to change file decriptor limit.
//!
//! [`backpressure`]: ../backpressure/fn.new.html
//!
//! ## Choosing a Limit
//!
//! There is no one good strategy. But here are few hints:
//!
//! 1. It must be lower than limit on simultaneous connections.
//! 2. Sometimes it's several times lower, like if you need to open a file
//!    or to open a backend connection for each client, it should be 2x lower
//!    plus some offset.
//! 3. Measure an average memory used by each client and divide memory
//!    available by that value.
//!
//! We use `10000` as an example value later in the text.
//!
//! ## Linux
//!
//! Either use `ulimit -n` in the same shell:
//! ```console
//! $ ulimit -n 10000
//! $ ./your_app
//! ```
//!
//! If you get the error, you need superuser privileges:
//! ```console
//! $ ulimit -n 10000
//! ulimit: value exceeds hard limit
//! $ sudo -s
//! # ulimit -n 10000
//! # su your_user
//! $ ./your_app
//! ```
//!
//! On most systems there is `/etc/security/limits.conf` to make persistent
//! changes:
//! ```text
//! your_user        hard nofile 10000
//! your_user        sort nofile 10000
//! ```
//!
//! Run ``su your_user`` to apply changes to your current shell without reboot.
//!
//! [More information](https://duckduckgo.com/?q=Increase+per-process+open+file+limit+linux)
//!
//! ## Docker
//!
//! Docker uses limit of `65535` by default. To increase it add a parameter:
//! ```shell
//! docker run --ulimit nofile=10000:10000 ...
//! ```
//!
//! ## MacOS
//!
//! On MacOS raising ulimit doesn't require permissions:
//! ```console
//! $ ulimit -n 10000
//! $ ./your_app
//! ```
//!
//! [More information](https://duckduckgo.com/?q=Increase+per-process+open+file+limit+macos)
//!
//! # Too Many Open Files in System <a name='ENFILE'></a>
//!
//! | Posix Name | ENFILE |
//! |---|---|
//! | Message | `Too many open files in system (os error 23)` |
//! | Hint | `Increase system open file limit` |
//! | Link | `https://big.ly/async-err#ENFILE` |
//!
//! ## Common Causes
//!
//! 1. Per-process file descriptor limit is larger than system one
//! 2. File descriptor limit on the system is too small
//! 3. Limit of number of simultaneous connections is either too high or
//!    unimplemented
//!
//! The (3) can be fixed by applying [`backpressure`] abstraction from this
//! crate.
//!
//! Changing (1) is described in the [section above](#EMFILE).
//!
//! The rest of this section discusses how to change system file decriptor
//! limit.
//!
//! ## Choosing a Limit
//!
//! Usually system limit depends on the memory and doesn't have to be
//! increased. So be careful and consult your system docs for more info.
//!
//! ## Linux
//!
//! Checking the limit:
//! ```console
//! $ cat /proc/sys/fs/file-max
//! 818354
//! ```
//!
//! Setting a limit:
//! ```
//! $ sudo sysctl fs.file-max=1500000
//! ```
//!
//! This only works **until reboot**. On some systems, to preserve this
//! setting after reboot you can run:
//! ```
//! $ sudo sysctl -w fs.file-max=1500000
//! ```
//! (Note `-w`)
//!
//! [More information](https://duckduckgo.com/?q=Increase+system+open+file+limit+linux)
//!
//! ## MacOS
//!
//! Checking a limit:
//! ```shell
//! launchctl limit maxfiles
//! ```
//!
//! Setting a limit, **until reboot**:
//! ```shell
//! sudo sysctl -w kern.maxfiles=20480
//! ```
//!
//! To make the permanent change, add the following to `/etc/sysctl.conf`:
//! ```config
//! kern.maxfiles=65536
//! kern.maxfilesperproc=65536
//! ```
//!
//! [More information](https://duckduckgo.com/?q=Increase+system+open+file+limit+macos)
