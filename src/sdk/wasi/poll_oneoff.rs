use super::raw_types;

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
) -> Result<(), raw_types::__wasi_errno_t> {
    if nsubscriptions == 0 {
        let in_slice = linker
            .get_memory_mut::<u32>("memory", revents_num_ptr, 1)
            .map_err(|_| raw_types::__wasi_errno_t::__WASI_ERRNO_FAULT)?;
        in_slice[0] = 0;
        return Ok(());
    }

    let in_slice = linker
        .get_memory::<raw_types::__wasi_subscription_t>("memory", in_ptr, nsubscriptions)
        .map_err(|_| raw_types::__wasi_errno_t::__WASI_ERRNO_FAULT)?;

    let mut wait = FuturesUnordered::new();

    for s in in_slice {
        let real_fd = replace_with_native_handle(linker, s);
        wait.push(wait_fd(real_fd, *s))
    }

    let n = {
        let r_event = linker
            .get_memory_mut::<raw_types::__wasi_event_t>("memory", out_ptr, nsubscriptions)
            .map_err(|_| raw_types::__wasi_errno_t::__WASI_ERRNO_FAULT)?;

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

    let in_slice = linker
        .get_memory_mut::<u32>("memory", revents_num_ptr, 1)
        .map_err(|_| raw_types::__wasi_errno_t::__WASI_ERRNO_FAULT)?;
    in_slice[0] = (n as u32).to_le();
    return Ok(());
}

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
                if let Err(r) =
                    async_poll_oneoff_(linker, in_ptr, out_ptr, nsubscriptions, revents_ptr).await
                {
                    Ok(vec![WasmVal::I32(r as i32)])
                } else {
                    Ok(vec![WasmVal::I32(
                        raw_types::__wasi_errno_t::__WASI_ERRNO_SUCCESS as i32,
                    )])
                }
            })
        }
        _ => Box::new(async {
            Ok(vec![WasmVal::I32(
                raw_types::__wasi_errno_t::__WASI_ERRNO_FAULT as i32,
            )])
        }),
    }
}
