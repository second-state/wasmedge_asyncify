#![allow(non_camel_case_types, non_upper_case_globals)]
mod raw_types;

use std::io::ErrorKind;

use wasmedge_types::WasmEdgeResult;

use crate::{types::WasmVal, AsyncLinker, ResultFuture};
use futures::{stream::FuturesUnordered, StreamExt};

fn monotonic_elapsed() -> std::time::Duration {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut t) };
    assert_eq!(
        status,
        0,
        "clock_gettime failed: {}",
        std::io::Error::last_os_error()
    );
    let d = std::time::Duration::new(
        t.tv_sec
            .try_into()
            .expect("The number of seconds should fit in an u64"),
        t.tv_nsec
            .try_into()
            .expect("The number of nanoseconds should fit in an u32"),
    );
    d
}

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
async fn wait_fd(
    real_fd: Result<u64, raw_types::__wasi_errno_t>,
    subs: raw_types::__wasi_subscription_t,
) -> raw_types::__wasi_event_t {
    let mut event = unsafe { std::mem::zeroed::<raw_types::__wasi_event_t>() };
    event.userdata = subs.userdata;
    event.type_ = subs.u.tag;
    match subs.u.tag {
        raw_types::__wasi_eventtype_t::__WASI_EVENTTYPE_CLOCK => {
            let clock = unsafe { subs.u.u.clock };
            match clock.id {
                raw_types::__wasi_clockid_t::__WASI_CLOCKID_REALTIME => {
                    if clock.flags as u16 == 1 {
                        let ddl = std::time::UNIX_EPOCH
                            + std::time::Duration::from_nanos(clock.timeout + clock.precision);
                        if let Ok(d) = ddl.duration_since(std::time::SystemTime::now()) {
                            tokio::time::sleep(d).await
                        }
                    } else {
                        let d = std::time::Duration::from_nanos(clock.timeout + clock.precision);
                        tokio::time::sleep(d).await
                    }
                }
                raw_types::__wasi_clockid_t::__WASI_CLOCKID_MONOTONIC => {
                    if clock.flags as u16 == 1 {
                        if let Some(d) =
                            std::time::Duration::from_nanos(clock.timeout + clock.precision)
                                .checked_sub(monotonic_elapsed())
                        {
                            tokio::time::sleep(d).await
                        }
                    } else {
                        let d = std::time::Duration::from_nanos(clock.timeout + clock.precision);
                        tokio::time::sleep(d).await
                    }
                }
                raw_types::__wasi_clockid_t::__WASI_CLOCKID_THREAD_CPUTIME_ID
                | raw_types::__wasi_clockid_t::__WASI_CLOCKID_PROCESS_CPUTIME_ID => {
                    event.error = raw_types::__wasi_errno_t::__WASI_ERRNO_NODEV
                }
            }
        }
        raw_types::__wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ => match real_fd {
            Ok(real_fd) => {
                let r = tokio::io::unix::AsyncFd::with_interest(
                    real_fd as i32,
                    tokio::io::Interest::READABLE,
                );
                match r {
                    Ok(fd) => match fd.readable().await {
                        Ok(_) => unsafe {
                            let mut recv_n = 0u64;
                            libc::ioctl(real_fd as i32, libc::FIONREAD, &mut recv_n);
                            event.fd_readwrite.nbytes = recv_n;
                        },
                        Err(e) => event.error = raw_types::__wasi_errno_t::from(&e),
                    },
                    Err(e) => event.error = raw_types::__wasi_errno_t::from(&e),
                }
            }
            Err(e) => event.error = e,
        },
        raw_types::__wasi_eventtype_t::__WASI_EVENTTYPE_FD_WRITE => match real_fd {
            Ok(real_fd) => {
                let r = tokio::io::unix::AsyncFd::with_interest(
                    real_fd as i32,
                    tokio::io::Interest::WRITABLE,
                );

                match r {
                    Ok(fd) => match fd.writable().await {
                        Ok(_) => {}
                        Err(e) => event.error = raw_types::__wasi_errno_t::from(&e),
                    },
                    Err(e) => event.error = raw_types::__wasi_errno_t::from(&e),
                }
            }
            Err(e) => event.error = e,
        },
    }

    event
}

fn replace_with_native_handle(
    linker: &AsyncLinker,
    subs: &raw_types::__wasi_subscription_t,
) -> Result<u64, raw_types::__wasi_errno_t> {
    unsafe {
        match subs.u.tag {
            raw_types::__wasi_eventtype_t::__WASI_EVENTTYPE_CLOCK => Ok(0),
            raw_types::__wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ => linker
                .wasi_get_native_handle(subs.u.u.fd_read.file_descriptor)
                .ok_or(raw_types::__wasi_errno_t::__WASI_ERRNO_BADF),
            raw_types::__wasi_eventtype_t::__WASI_EVENTTYPE_FD_WRITE => linker
                .wasi_get_native_handle(subs.u.u.fd_write.file_descriptor)
                .ok_or(raw_types::__wasi_errno_t::__WASI_ERRNO_BADF),
        }
    }
}

async fn async_poll_oneoff_(
    linker: &mut AsyncLinker,
    in_ptr: usize,
    out_ptr: usize,
    nsubscriptions: usize,
    revents_num_ptr: usize,
) -> WasmEdgeResult<raw_types::__wasi_errno_t> {
    if nsubscriptions == 0 {
        let in_slice = linker.get_memory_mut::<u32>("memory", revents_num_ptr, 1)?;
        in_slice[0] = 0;
        return Ok(raw_types::__wasi_errno_t::__WASI_ERRNO_SUCCESS);
    }

    let in_slice =
        linker.get_memory::<raw_types::__wasi_subscription_t>("memory", in_ptr, nsubscriptions)?;

    let mut wait = FuturesUnordered::new();

    for s in in_slice {
        let real_fd = replace_with_native_handle(linker, s);
        wait.push(wait_fd(real_fd, *s))
    }

    let n = {
        let r_event = linker.get_memory_mut::<raw_types::__wasi_event_t>(
            "memory",
            out_ptr,
            nsubscriptions,
        )?;

        let mut i = 0;

        let v = wait.select_next_some().await;
        r_event[i] = v;
        i += 1;

        'wait_poll: loop {
            if i >= nsubscriptions {
                break 'wait_poll;
            }
            futures::select! {
                v = wait.next()=>{
                    if let Some(v) = v{
                        r_event[i] = v;
                        i+=1;
                    }else{
                        break 'wait_poll;
                    }
                }
                default =>{
                    break 'wait_poll;
                }
            };
        }
        i
    };

    let in_slice = linker.get_memory_mut::<u32>("memory", revents_num_ptr, 1)?;
    in_slice[0] = (n as u32).to_le();
    return Ok(raw_types::__wasi_errno_t::__WASI_ERRNO_SUCCESS);
}

#[allow(unused)]
pub fn async_poll_oneoff(linker: &mut AsyncLinker, args: Vec<WasmVal>) -> ResultFuture {
    match args.get(0..4) {
        Some(
            [WasmVal::I32(in_ptr), WasmVal::I32(out_ptr), WasmVal::I32(nsubscriptions), WasmVal::I32(revents_ptr)],
        ) => {
            let in_ptr = *in_ptr as usize;
            let out_ptr = *out_ptr as usize;
            let nsubscriptions = *nsubscriptions as usize;
            let revents_ptr = *revents_ptr as usize;

            Box::new(async move {
                let r = async_poll_oneoff_(linker, in_ptr, out_ptr, nsubscriptions, revents_ptr)
                    .await?;
                Ok(vec![WasmVal::I32(r as i32)])
            })
        }
        _ => Box::new(async {
            Ok(vec![WasmVal::I32(
                raw_types::__wasi_errno_t::__WASI_ERRNO_FAULT as i32,
            )])
        }),
    }
}
