#![allow(non_camel_case_types, non_upper_case_globals)]
mod raw_types;

mod poll_oneoff;
pub use poll_oneoff::async_poll_oneoff;

mod sock_recv;

use std::io::ErrorKind;

impl From<ErrorKind> for raw_types::__wasi_errno_t {
    fn from(e: ErrorKind) -> raw_types::__wasi_errno_t {
        use raw_types::__wasi_errno_t;
        match e {
            ErrorKind::NotFound => __wasi_errno_t::__WASI_ERRNO_NOENT,
            ErrorKind::PermissionDenied => __wasi_errno_t::__WASI_ERRNO_PERM,
            ErrorKind::ConnectionRefused => __wasi_errno_t::__WASI_ERRNO_CONNREFUSED,
            ErrorKind::ConnectionReset => __wasi_errno_t::__WASI_ERRNO_CONNRESET,
            ErrorKind::ConnectionAborted => __wasi_errno_t::__WASI_ERRNO_CONNABORTED,
            ErrorKind::NotConnected => __wasi_errno_t::__WASI_ERRNO_NOTCONN,
            ErrorKind::AddrInUse => __wasi_errno_t::__WASI_ERRNO_ADDRINUSE,
            ErrorKind::AddrNotAvailable => __wasi_errno_t::__WASI_ERRNO_ADDRNOTAVAIL,
            ErrorKind::BrokenPipe => __wasi_errno_t::__WASI_ERRNO_PIPE,
            ErrorKind::AlreadyExists => __wasi_errno_t::__WASI_ERRNO_EXIST,
            ErrorKind::WouldBlock => __wasi_errno_t::__WASI_ERRNO_AGAIN,
            ErrorKind::InvalidInput => __wasi_errno_t::__WASI_ERRNO_IO,
            ErrorKind::InvalidData => __wasi_errno_t::__WASI_ERRNO_IO,
            ErrorKind::TimedOut => __wasi_errno_t::__WASI_ERRNO_TIMEDOUT,
            ErrorKind::WriteZero => __wasi_errno_t::__WASI_ERRNO_IO,
            ErrorKind::Interrupted => __wasi_errno_t::__WASI_ERRNO_INTR,
            ErrorKind::Other => __wasi_errno_t::__WASI_ERRNO_IO,
            ErrorKind::UnexpectedEof => __wasi_errno_t::__WASI_ERRNO_IO,
            ErrorKind::Unsupported => __wasi_errno_t::__WASI_ERRNO_NOTSUP,
            _ => __wasi_errno_t::__WASI_ERRNO_IO,
        }
    }
}

impl From<&std::io::Error> for raw_types::__wasi_errno_t {
    fn from(e: &std::io::Error) -> Self {
        use raw_types::__wasi_errno_t;
        if let Some(error_code) = e.raw_os_error() {
            match error_code {
                0 => __wasi_errno_t::__WASI_ERRNO_SUCCESS,
                libc::E2BIG => __wasi_errno_t::__WASI_ERRNO_2BIG,
                libc::EACCES => __wasi_errno_t::__WASI_ERRNO_ACCES,
                libc::EADDRINUSE => __wasi_errno_t::__WASI_ERRNO_ADDRINUSE,
                libc::EADDRNOTAVAIL => __wasi_errno_t::__WASI_ERRNO_ADDRNOTAVAIL,
                libc::EAFNOSUPPORT => __wasi_errno_t::__WASI_ERRNO_AFNOSUPPORT,
                libc::EAGAIN => __wasi_errno_t::__WASI_ERRNO_AGAIN,
                libc::EALREADY => __wasi_errno_t::__WASI_ERRNO_ALREADY,
                libc::EBADF => __wasi_errno_t::__WASI_ERRNO_BADF,
                libc::EBADMSG => __wasi_errno_t::__WASI_ERRNO_BADMSG,
                libc::EBUSY => __wasi_errno_t::__WASI_ERRNO_BUSY,
                libc::ECANCELED => __wasi_errno_t::__WASI_ERRNO_CANCELED,
                libc::ECHILD => __wasi_errno_t::__WASI_ERRNO_CHILD,
                libc::ECONNABORTED => __wasi_errno_t::__WASI_ERRNO_CONNABORTED,
                libc::ECONNREFUSED => __wasi_errno_t::__WASI_ERRNO_CONNREFUSED,
                libc::ECONNRESET => __wasi_errno_t::__WASI_ERRNO_CONNRESET,
                libc::EDEADLK => __wasi_errno_t::__WASI_ERRNO_DEADLK,
                libc::EDESTADDRREQ => __wasi_errno_t::__WASI_ERRNO_DESTADDRREQ,
                libc::EDOM => __wasi_errno_t::__WASI_ERRNO_DOM,
                libc::EDQUOT => __wasi_errno_t::__WASI_ERRNO_DQUOT,
                libc::EEXIST => __wasi_errno_t::__WASI_ERRNO_EXIST,
                libc::EFAULT => __wasi_errno_t::__WASI_ERRNO_FAULT,
                libc::EFBIG => __wasi_errno_t::__WASI_ERRNO_FBIG,
                libc::EHOSTUNREACH => __wasi_errno_t::__WASI_ERRNO_HOSTUNREACH,
                libc::EIDRM => __wasi_errno_t::__WASI_ERRNO_IDRM,
                libc::EILSEQ => __wasi_errno_t::__WASI_ERRNO_ILSEQ,
                libc::EINPROGRESS => __wasi_errno_t::__WASI_ERRNO_INPROGRESS,
                libc::EINTR => __wasi_errno_t::__WASI_ERRNO_INTR,
                libc::EINVAL => __wasi_errno_t::__WASI_ERRNO_INVAL,
                libc::EIO => __wasi_errno_t::__WASI_ERRNO_IO,
                libc::EISCONN => __wasi_errno_t::__WASI_ERRNO_ISCONN,
                libc::EISDIR => __wasi_errno_t::__WASI_ERRNO_ISDIR,
                libc::ELOOP => __wasi_errno_t::__WASI_ERRNO_LOOP,
                libc::EMFILE => __wasi_errno_t::__WASI_ERRNO_MFILE,
                libc::EMLINK => __wasi_errno_t::__WASI_ERRNO_MLINK,
                libc::EMSGSIZE => __wasi_errno_t::__WASI_ERRNO_MSGSIZE,
                libc::EMULTIHOP => __wasi_errno_t::__WASI_ERRNO_MULTIHOP,
                libc::ENAMETOOLONG => __wasi_errno_t::__WASI_ERRNO_NAMETOOLONG,
                libc::ENETDOWN => __wasi_errno_t::__WASI_ERRNO_NETDOWN,
                libc::ENETRESET => __wasi_errno_t::__WASI_ERRNO_NETRESET,
                libc::ENETUNREACH => __wasi_errno_t::__WASI_ERRNO_NETUNREACH,
                libc::ENFILE => __wasi_errno_t::__WASI_ERRNO_NFILE,
                libc::ENOBUFS => __wasi_errno_t::__WASI_ERRNO_NOBUFS,
                libc::ENODEV => __wasi_errno_t::__WASI_ERRNO_NODEV,
                libc::ENOENT => __wasi_errno_t::__WASI_ERRNO_NOENT,
                libc::ENOEXEC => __wasi_errno_t::__WASI_ERRNO_NOEXEC,
                libc::ENOLCK => __wasi_errno_t::__WASI_ERRNO_NOLCK,
                libc::ENOLINK => __wasi_errno_t::__WASI_ERRNO_NOLINK,
                libc::ENOMEM => __wasi_errno_t::__WASI_ERRNO_NOMEM,
                libc::ENOMSG => __wasi_errno_t::__WASI_ERRNO_NOMSG,
                libc::ENOPROTOOPT => __wasi_errno_t::__WASI_ERRNO_NOPROTOOPT,
                libc::ENOSPC => __wasi_errno_t::__WASI_ERRNO_NOSPC,
                libc::ENOSYS => __wasi_errno_t::__WASI_ERRNO_NOSYS,
                libc::ENOTCONN => __wasi_errno_t::__WASI_ERRNO_NOTCONN,
                libc::ENOTDIR => __wasi_errno_t::__WASI_ERRNO_NOTDIR,
                libc::ENOTEMPTY => __wasi_errno_t::__WASI_ERRNO_NOTEMPTY,
                libc::ENOTRECOVERABLE => __wasi_errno_t::__WASI_ERRNO_NOTRECOVERABLE,
                libc::ENOTSOCK => __wasi_errno_t::__WASI_ERRNO_NOTSOCK,
                libc::ENOTSUP => __wasi_errno_t::__WASI_ERRNO_NOTSUP,
                libc::ENOTTY => __wasi_errno_t::__WASI_ERRNO_NOTTY,
                libc::ENXIO => __wasi_errno_t::__WASI_ERRNO_NXIO,
                libc::EOVERFLOW => __wasi_errno_t::__WASI_ERRNO_OVERFLOW,
                libc::EOWNERDEAD => __wasi_errno_t::__WASI_ERRNO_OWNERDEAD,
                libc::EPERM => __wasi_errno_t::__WASI_ERRNO_PERM,
                libc::EPIPE => __wasi_errno_t::__WASI_ERRNO_PIPE,
                libc::EPROTO => __wasi_errno_t::__WASI_ERRNO_PROTO,
                libc::EPROTONOSUPPORT => __wasi_errno_t::__WASI_ERRNO_PROTONOSUPPORT,
                libc::EPROTOTYPE => __wasi_errno_t::__WASI_ERRNO_PROTOTYPE,
                libc::ERANGE => __wasi_errno_t::__WASI_ERRNO_RANGE,
                libc::EROFS => __wasi_errno_t::__WASI_ERRNO_ROFS,
                libc::ESPIPE => __wasi_errno_t::__WASI_ERRNO_SPIPE,
                libc::ESRCH => __wasi_errno_t::__WASI_ERRNO_SRCH,
                libc::ESTALE => __wasi_errno_t::__WASI_ERRNO_STALE,
                libc::ETIMEDOUT => __wasi_errno_t::__WASI_ERRNO_TIMEDOUT,
                libc::ETXTBSY => __wasi_errno_t::__WASI_ERRNO_TXTBSY,
                libc::EXDEV => __wasi_errno_t::__WASI_ERRNO_XDEV,
                _ => __wasi_errno_t::__WASI_ERRNO_NOTSUP,
            }
        } else {
            let kind = e.kind();
            raw_types::__wasi_errno_t::from(kind)
        }
    }
}
