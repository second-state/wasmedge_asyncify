use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use net::WASIRights;

use crate::snapshots::common::memory::{Memory, WasmPtr};
use crate::snapshots::common::net::{self, AddressFamily, SocketType, WasiSocketState};
use crate::snapshots::common::types::*;
use crate::snapshots::env::{self, VFD};
use crate::snapshots::Errno;
use crate::snapshots::WasiCtx;

fn parse_wasi_ip<M: Memory>(mem: &M, addr_ptr: WasmPtr<__wasi_address_t>) -> Result<IpAddr, Errno> {
    let wasi_addr = *(mem.get_data(addr_ptr)?);
    if wasi_addr.buf_len != 4 && wasi_addr.buf_len != 16 {
        return Err(Errno::__WASI_ERRNO_INVAL);
    }
    let addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);

    let addr = if wasi_addr.buf_len == 4 {
        let addr_buf = mem.get_slice(addr_buf_ptr, 4)?;
        IpAddr::V4(Ipv4Addr::new(
            addr_buf[0],
            addr_buf[1],
            addr_buf[2],
            addr_buf[3],
        ))
    } else {
        let addr_buf_ref = mem.get_slice(addr_buf_ptr, 16)?;
        let mut addr_buf = [0u8; 16];
        addr_buf.copy_from_slice(addr_buf_ref);
        IpAddr::V6(Ipv6Addr::from(addr_buf))
    };
    Ok(addr)
}

pub fn sock_open<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    af: __wasi_address_family_t::Type,
    ty: __wasi_sock_type_t::Type,
    ro_fd_ptr: WasmPtr<__wasi_fd_t>,
) -> Result<(), Errno> {
    let mut state = WasiSocketState::default();
    match af {
        __wasi_address_family_t::__WASI_ADDRESS_FAMILY_INET4 => {
            state.sock_type.0 = AddressFamily::Inet4
        }
        __wasi_address_family_t::__WASI_ADDRESS_FAMILY_INET6 => {
            state.sock_type.0 = AddressFamily::Inet6
        }
        _ => return Err(Errno::__WASI_ERRNO_INVAL),
    }
    match ty {
        __wasi_sock_type_t::__WASI_SOCK_TYPE_SOCK_DGRAM => {
            state.fs_rights = WASIRights::SOCK_BIND
                | WASIRights::SOCK_CLOSE
                | WASIRights::SOCK_RECV_FROM
                | WASIRights::SOCK_SEND_TO
                | WASIRights::SOCK_SHUTDOWN
                | WASIRights::POLL_FD_READWRITE;
            state.sock_type.1 = SocketType::Datagram;
        }
        __wasi_sock_type_t::__WASI_SOCK_TYPE_SOCK_STREAM => {
            state.fs_rights = WASIRights::SOCK_BIND
                | WASIRights::SOCK_CLOSE
                | WASIRights::SOCK_RECV
                | WASIRights::SOCK_SEND
                | WASIRights::SOCK_SHUTDOWN
                | WASIRights::POLL_FD_READWRITE;
            state.sock_type.1 = SocketType::Stream;
        }
        _ => return Err(Errno::__WASI_ERRNO_INVAL),
    }

    let s = net::sync::SyncWasiSocket::open(state)?;
    let fd = ctx.insert_vfd(env::VFD::Socket(s).into())?;
    mem.write_data(ro_fd_ptr, fd)?;
    Ok(())
}

pub fn sock_bind<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    addr_ptr: WasmPtr<__wasi_address_t>,
    port: u32,
) -> Result<(), Errno> {
    let ip = parse_wasi_ip(mem, addr_ptr)?;
    let addr = SocketAddr::new(ip, port as u16);

    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        s.bind(addr)?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_listen<M: Memory>(
    ctx: &mut WasiCtx,
    _mem: &mut M,
    fd: __wasi_fd_t,
    backlog: u32,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        s.listen(backlog)?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_accept<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    ro_fd_ptr: WasmPtr<__wasi_fd_t>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let cs = s.accept()?;
        let new_fd = ctx.insert_vfd(env::VFD::Socket(cs).into())?;
        mem.write_data(ro_fd_ptr, new_fd)?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_connect<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    addr_ptr: WasmPtr<__wasi_address_t>,
    port: u32,
) -> Result<(), Errno> {
    let ip = parse_wasi_ip(mem, addr_ptr)?;
    let addr = SocketAddr::new(ip, port as u16);

    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        s.connect(addr)?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_recv<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    buf_ptr: WasmPtr<__wasi_iovec_t>,
    buf_len: __wasi_size_t,
    flags: __wasi_riflags_t::Type,
    ro_data_len_ptr: WasmPtr<__wasi_size_t>,
    ro_flags_ptr: WasmPtr<__wasi_roflags_t::Type>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let mut iovec = mem.mut_iovec(buf_ptr, buf_len)?;
        let mut native_flags = 0;

        if flags & __wasi_riflags_t::__WASI_RIFLAGS_RECV_PEEK > 0 {
            native_flags |= libc::MSG_PEEK;
        }
        if flags & __wasi_riflags_t::__WASI_RIFLAGS_RECV_WAITALL > 0 {
            native_flags |= libc::MSG_WAITALL;
        }

        let (n, trunc) = s.recv(&mut iovec, native_flags)?;
        if trunc {
            mem.write_data(
                ro_flags_ptr,
                __wasi_roflags_t::__WASI_ROFLAGS_RECV_DATA_TRUNCATED,
            )?;
        }

        mem.write_data(ro_data_len_ptr, (n as u32).to_le())?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_recv_from<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    buf_ptr: WasmPtr<__wasi_iovec_t>,
    buf_len: __wasi_size_t,
    wasi_addr_ptr: WasmPtr<__wasi_address_t>,
    port_ptr: WasmPtr<u32>,
    flags: __wasi_riflags_t::Type,
    ro_data_len_ptr: WasmPtr<__wasi_size_t>,
    ro_flags_ptr: WasmPtr<__wasi_roflags_t::Type>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let wasi_addr = *(mem.mut_data(wasi_addr_ptr)?);
        if wasi_addr.buf_len < 16 {
            return Err(Errno::__WASI_ERRNO_INVAL);
        }

        let mut iovec = mem.mut_iovec(buf_ptr, buf_len)?;
        let mut native_flags = 0;

        if flags & __wasi_riflags_t::__WASI_RIFLAGS_RECV_PEEK > 0 {
            native_flags |= libc::MSG_PEEK;
        }
        if flags & __wasi_riflags_t::__WASI_RIFLAGS_RECV_WAITALL > 0 {
            native_flags |= libc::MSG_WAITALL;
        }

        let (n, trunc, addr) = s.recv_from(&mut iovec, native_flags)?;

        let addr_len: u32 = match addr {
            Some(SocketAddr::V4(addrv4)) => {
                let wasi_addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);
                let wasi_addr_buf = mem.mut_slice(wasi_addr_buf_ptr, 4)?;
                wasi_addr_buf.copy_from_slice(&addrv4.ip().octets());
                mem.write_data(port_ptr, (addrv4.port() as u32).to_le())?;
                4
            }
            Some(SocketAddr::V6(addrv6)) => {
                let wasi_addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);
                let wasi_addr_buf = mem.mut_slice(wasi_addr_buf_ptr, 16)?;
                wasi_addr_buf.copy_from_slice(&addrv6.ip().octets());
                mem.write_data(port_ptr, (addrv6.port() as u32).to_le())?;
                16
            }
            None => 0,
        };

        let wasi_addr = mem.mut_data(wasi_addr_ptr)?;
        wasi_addr.buf_len = addr_len.to_le();

        if trunc {
            mem.write_data(
                ro_flags_ptr,
                __wasi_roflags_t::__WASI_ROFLAGS_RECV_DATA_TRUNCATED,
            )?;
        }

        mem.write_data(ro_data_len_ptr, (n as u32).to_le())?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_send<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    buf_ptr: WasmPtr<__wasi_ciovec_t>,
    buf_len: __wasi_size_t,
    _flags: __wasi_siflags_t,
    send_len_ptr: WasmPtr<__wasi_size_t>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let iovec = mem.get_iovec(buf_ptr, buf_len)?;
        let n = s.send(&iovec, libc::MSG_NOSIGNAL)?;
        mem.write_data(send_len_ptr, (n as u32).to_le())?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_send_to<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    buf_ptr: WasmPtr<__wasi_ciovec_t>,
    buf_len: __wasi_size_t,
    wasi_addr_ptr: WasmPtr<__wasi_address_t>,
    port: u32,
    _flags: __wasi_siflags_t,
    send_len_ptr: WasmPtr<__wasi_size_t>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let ip = parse_wasi_ip(mem, wasi_addr_ptr)?;
        let addr = SocketAddr::new(ip, port as u16);
        let iovec = mem.get_iovec(buf_ptr, buf_len)?;

        let n = s.send_to(&iovec, addr, libc::MSG_NOSIGNAL)?;
        mem.write_data(send_len_ptr, (n as u32).to_le())?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_shutdown<M: Memory>(
    ctx: &mut WasiCtx,
    _mem: &mut M,
    fd: __wasi_fd_t,
    how: __wasi_sdflags_t::Type,
) -> Result<(), Errno> {
    use std::net::Shutdown;
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        const BOTH: __wasi_sdflags_t::Type =
            __wasi_sdflags_t::__WASI_SDFLAGS_WR | __wasi_sdflags_t::__WASI_SDFLAGS_RD;

        let how = match how {
            __wasi_sdflags_t::__WASI_SDFLAGS_RD => Shutdown::Read,
            __wasi_sdflags_t::__WASI_SDFLAGS_WR => Shutdown::Write,
            BOTH => Shutdown::Both,
            _ => return Err(Errno::__WASI_ERRNO_INVAL),
        };

        s.shutdown(how)?;
        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_getpeeraddr<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    wasi_addr_ptr: WasmPtr<__wasi_address_t>,
    addr_type: WasmPtr<u32>,
    port_ptr: WasmPtr<u32>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let wasi_addr = *(mem.mut_data(wasi_addr_ptr)?);

        let addr = s.get_peer()?;

        let addr_len: u32 = match addr {
            SocketAddr::V4(addrv4) => {
                let wasi_addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);
                let wasi_addr_buf = mem.mut_slice(wasi_addr_buf_ptr, 4)?;
                wasi_addr_buf.copy_from_slice(&addrv4.ip().octets());
                mem.write_data(port_ptr, (addrv4.port() as u32).to_le())?;
                4
            }
            SocketAddr::V6(addrv6) => {
                let wasi_addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);
                let wasi_addr_buf = mem.mut_slice(wasi_addr_buf_ptr, 16)?;
                wasi_addr_buf.copy_from_slice(&addrv6.ip().octets());
                mem.write_data(port_ptr, (addrv6.port() as u32).to_le())?;
                16
            }
        };

        let wasi_addr = mem.mut_data(wasi_addr_ptr)?;
        wasi_addr.buf_len = addr_len.to_le();
        mem.write_data(addr_type, addr_len.to_le())?;

        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_getlocaladdr<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    wasi_addr_ptr: WasmPtr<__wasi_address_t>,
    addr_type: WasmPtr<u32>,
    port_ptr: WasmPtr<u32>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let wasi_addr = *(mem.mut_data(wasi_addr_ptr)?);

        let addr = s.get_local()?;

        let addr_len: u32 = match addr {
            SocketAddr::V4(addrv4) => {
                let wasi_addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);
                let wasi_addr_buf = mem.mut_slice(wasi_addr_buf_ptr, 4)?;
                wasi_addr_buf.copy_from_slice(&addrv4.ip().octets());
                mem.write_data(port_ptr, (addrv4.port() as u32).to_le())?;
                4
            }
            SocketAddr::V6(addrv6) => {
                let wasi_addr_buf_ptr = WasmPtr::<u8>::from(wasi_addr.buf as usize);
                let wasi_addr_buf = mem.mut_slice(wasi_addr_buf_ptr, 16)?;
                wasi_addr_buf.copy_from_slice(&addrv6.ip().octets());
                mem.write_data(port_ptr, (addrv6.port() as u32).to_le())?;
                16
            }
        };

        let wasi_addr = mem.mut_data(wasi_addr_ptr)?;
        wasi_addr.buf_len = addr_len.to_le();
        mem.write_data(addr_type, addr_len.to_le())?;

        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_getsockopt<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    level: __wasi_sock_opt_level_t::Type,
    name: __wasi_sock_opt_so_t::Type,
    flag: WasmPtr<i32>,
    flag_size_ptr: WasmPtr<__wasi_size_t>,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        let flag_size = *(mem.get_data(flag_size_ptr)?);
        if level != __wasi_sock_opt_level_t::__WASI_SOCK_OPT_LEVEL_SOL_SOCKET {
            return Err(Errno::__WASI_ERRNO_NOSYS);
        }
        let flag_val = match name {
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_REUSEADDR => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                s.get_so_reuseaddr() as i32
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_TYPE => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }

                let (_, t) = s.get_so_type();
                let s = match t {
                    SocketType::Datagram => __wasi_sock_type_t::__WASI_SOCK_TYPE_SOCK_DGRAM,
                    SocketType::Stream => __wasi_sock_type_t::__WASI_SOCK_TYPE_SOCK_STREAM,
                } as i32;
                s
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_ERROR => {
                if let Some(e) = s.get_so_error()? {
                    Errno::from(e).0 as i32
                } else {
                    0
                }
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_DONTROUTE => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_BROADCAST => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_SNDBUF => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                s.get_so_send_buf_size() as i32
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_RCVBUF => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                s.get_so_recv_buf_size() as i32
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_KEEPALIVE => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_OOBINLINE => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_LINGER => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_RCVLOWAT => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_RCVTIMEO => {
                if (flag_size as usize) != std::mem::size_of::<__wasi_timeval>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }

                if let Some(timeout) = s.get_so_recv_timeout() {
                    let timeval = __wasi_timeval {
                        tv_sec: (timeout.as_secs() as i64).to_le(),
                        tv_usec: (timeout.subsec_nanos() as i64).to_le(),
                    };
                    let offset = WasmPtr::<__wasi_timeval>::from(flag.0);
                    mem.write_data(offset, timeval)?;
                }

                return Ok(());
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_SNDTIMEO => {
                if (flag_size as usize) != std::mem::size_of::<__wasi_timeval>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }

                if let Some(timeout) = s.get_so_send_timeout() {
                    let timeval = __wasi_timeval {
                        tv_sec: (timeout.as_secs() as i64).to_le(),
                        tv_usec: (timeout.subsec_nanos() as i64).to_le(),
                    };
                    let offset = WasmPtr::<__wasi_timeval>::from(flag.0);
                    mem.write_data(offset, timeval)?;
                }

                return Ok(());
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_ACCEPTCONN => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                s.get_so_accept_conn()? as i32
            }
            _ => {
                return Err(Errno::__WASI_ERRNO_NOPROTOOPT);
            }
        };

        mem.write_data(flag, flag_val)?;

        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

pub fn sock_setsockopt<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    fd: __wasi_fd_t,
    level: __wasi_sock_opt_level_t::Type,
    name: __wasi_sock_opt_so_t::Type,
    flag: WasmPtr<i32>,
    flag_size: __wasi_size_t,
) -> Result<(), Errno> {
    let sock_fd = ctx.get_mut_vfd(fd)?;
    if let VFD::Socket(s) = sock_fd {
        if level != __wasi_sock_opt_level_t::__WASI_SOCK_OPT_LEVEL_SOL_SOCKET {
            return Err(Errno::__WASI_ERRNO_NOSYS);
        }
        match name {
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_REUSEADDR => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                let flag_val = *(mem.get_data(flag)?) > 0;
                s.set_so_reuseaddr(flag_val)?;
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_TYPE => return Err(Errno::__WASI_ERRNO_FAULT),
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_ERROR => {
                return Err(Errno::__WASI_ERRNO_FAULT)
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_DONTROUTE => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_BROADCAST => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_SNDBUF => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                let flag_val = *(mem.get_data(flag)?);
                s.set_so_send_buf_size(flag_val as usize)?;
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_RCVBUF => {
                if (flag_size as usize) != std::mem::size_of::<i32>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                let flag_val = *(mem.get_data(flag)?);
                s.set_so_recv_buf_size(flag_val as usize)?;
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_KEEPALIVE => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_OOBINLINE => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_LINGER => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_RCVLOWAT => {
                return Err(Errno::__WASI_ERRNO_NOSYS);
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_RCVTIMEO => {
                if (flag_size as usize) != std::mem::size_of::<__wasi_timeval>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                let offset = WasmPtr::<__wasi_timeval>::from(flag.0);
                let timeval = *(mem.get_data(offset)?);
                let (tv_sec, tv_usec) =
                    (i64::from_le(timeval.tv_sec), i64::from_le(timeval.tv_usec));

                let timeout = if tv_sec == 0 && tv_usec == 0 {
                    None
                } else {
                    Some(Duration::new(tv_sec as u64, tv_usec as u32))
                };

                s.set_so_recv_timeout(timeout)?;
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_SNDTIMEO => {
                if (flag_size as usize) != std::mem::size_of::<__wasi_timeval>() {
                    return Err(Errno::__WASI_ERRNO_INVAL);
                }
                let offset = WasmPtr::<__wasi_timeval>::from(flag.0);
                let timeval = *(mem.get_data(offset)?);
                let (tv_sec, tv_usec) =
                    (i64::from_le(timeval.tv_sec), i64::from_le(timeval.tv_usec));

                let timeout = if tv_sec == 0 && tv_usec == 0 {
                    None
                } else {
                    Some(Duration::new(tv_sec as u64, tv_usec as u32))
                };

                s.set_so_send_timeout(timeout)?;
            }
            __wasi_sock_opt_so_t::__WASI_SOCK_OPT_SO_ACCEPTCONN => {
                return Err(Errno::__WASI_ERRNO_FAULT);
            }
            _ => {
                return Err(Errno::__WASI_ERRNO_NOPROTOOPT);
            }
        };

        Ok(())
    } else {
        Err(Errno::__WASI_ERRNO_NOTSOCK)
    }
}

#[cfg(unix)]
pub fn poll_oneoff<M: Memory>(
    ctx: &mut WasiCtx,
    mem: &mut M,
    in_ptr: WasmPtr<__wasi_subscription_t>,
    out_ptr: WasmPtr<__wasi_event_t>,
    nsubscriptions: __wasi_size_t,
    revents_num_ptr: WasmPtr<__wasi_size_t>,
) -> Result<(), Errno> {
    use mio::{Events, Interest, Poll, Token};
    use net::{PrePoll, SubscriptionClock, SubscriptionFd};

    pub fn handler_events<M: Memory>(
        ctx: &mut WasiCtx,
        mem: &mut M,
        out_ptr: WasmPtr<__wasi_event_t>,
        nsubscriptions: __wasi_size_t,
        revents_num_ptr: WasmPtr<__wasi_size_t>,
        events: &mio::Events,
        clock: Option<SubscriptionClock>,
        fds: &[SubscriptionFd],
    ) -> Result<(), Errno> {
        let r_events = mem.mut_slice(out_ptr, nsubscriptions as usize)?;
        let mut ri = 0;

        if events.is_empty() && clock.is_some() {
            let clock = clock.unwrap();
            let r_event = &mut r_events[0];
            r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_CLOCK;
            r_event.userdata = clock.userdata;
            mem.write_data(revents_num_ptr, 1)?;
            return Ok(());
        }

        for event in events.iter() {
            let i = event.token().0;
            let sub_fd = fds[i];
            match sub_fd.type_ {
                net::SubscriptionFdType::Write(userdata) => {
                    let r_event = &mut r_events[ri];
                    r_event.userdata = userdata;
                    r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_WRITE;
                    if event.is_write_closed() {
                        r_event.fd_readwrite.flags =
                            __wasi_eventrwflags_t::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP;
                    }
                    if event.is_error() {
                        if let VFD::Socket(s) = ctx.get_mut_vfd(sub_fd.fd)? {
                            if let Ok(Some(e)) = s.get_so_error() {
                                r_event.error = Errno::from(e).0;
                            }
                        }
                    }
                    ri += 1;
                }
                net::SubscriptionFdType::Read(userdata) => {
                    let r_event = &mut r_events[ri];
                    r_event.userdata = userdata;
                    r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                    if event.is_read_closed() {
                        r_event.fd_readwrite.flags =
                            __wasi_eventrwflags_t::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP;
                    }
                    if event.is_error() {
                        if let VFD::Socket(s) = ctx.get_mut_vfd(sub_fd.fd)? {
                            if let Ok(Some(e)) = s.get_so_error() {
                                r_event.error = Errno::from(e).0;
                            }
                        }
                    }
                    ri += 1;
                }
                net::SubscriptionFdType::Both {
                    read: r_userdata,
                    write: w_userdata,
                } => {
                    if event.is_readable() {
                        let r_event = &mut r_events[ri];
                        r_event.userdata = r_userdata;
                        r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                        if event.is_read_closed() {
                            r_event.fd_readwrite.flags =
                                __wasi_eventrwflags_t::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP;
                        }
                        if event.is_error() {
                            if let VFD::Socket(s) = ctx.get_mut_vfd(sub_fd.fd)? {
                                if let Ok(Some(e)) = s.get_so_error() {
                                    r_event.error = Errno::from(e).0;
                                }
                            }
                        }
                        ri += 1;
                    }
                    if event.is_writable() {
                        let r_event = &mut r_events[ri];
                        r_event.userdata = w_userdata;
                        r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_WRITE;
                        if event.is_write_closed() {
                            r_event.fd_readwrite.flags =
                                __wasi_eventrwflags_t::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP;
                        }
                        if event.is_error() {
                            if let VFD::Socket(s) = ctx.get_mut_vfd(sub_fd.fd)? {
                                if let Ok(Some(e)) = s.get_so_error() {
                                    r_event.error = Errno::from(e).0;
                                }
                            }
                        }
                        ri += 1;
                    }
                    if event.is_error() && !event.is_readable() && !event.is_writable() {
                        let r_event = &mut r_events[ri];
                        r_event.userdata = r_userdata;
                        r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                        if event.is_read_closed() {
                            r_event.fd_readwrite.flags =
                                __wasi_eventrwflags_t::__WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP;
                        }
                        if event.is_error() {
                            if let VFD::Socket(s) = ctx.get_mut_vfd(sub_fd.fd)? {
                                if let Ok(Some(e)) = s.get_so_error() {
                                    r_event.error = Errno::from(e).0;
                                }
                            }
                        }
                        ri += 1;
                    }
                }
            }
        }

        mem.write_data(revents_num_ptr, ri as u32)
    }

    if nsubscriptions <= 0 {
        return Ok(());
    }

    let subs = mem.get_slice(in_ptr, nsubscriptions as usize)?;
    let prepoll = PrePoll::from_wasi_subscription(subs)?;
    match prepoll {
        PrePoll::OnlyFd(fd_vec) => {
            if fd_vec.is_empty() {
                mem.write_data(revents_num_ptr, 0)?;
            } else {
                let mut poller = Poll::new()?;
                let mut events = Events::with_capacity(fd_vec.len());

                for (i, fd) in fd_vec.iter().enumerate() {
                    if let VFD::Socket(s) = ctx.get_mut_vfd(fd.fd)? {
                        let interests = match fd.type_ {
                            net::SubscriptionFdType::Read(_) => Interest::READABLE,
                            net::SubscriptionFdType::Write(_) => Interest::WRITABLE,
                            net::SubscriptionFdType::Both { .. } => {
                                Interest::READABLE | Interest::WRITABLE
                            }
                        };

                        poller.registry().register(s, Token(i), interests)?;
                    } else {
                        let r_event = mem.mut_data(out_ptr)?;
                        match fd.type_ {
                            net::SubscriptionFdType::Read(userdata) => {
                                r_event.userdata = userdata;
                                r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                            }
                            net::SubscriptionFdType::Write(userdata) => {
                                r_event.userdata = userdata;
                                r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_WRITE;
                            }
                            net::SubscriptionFdType::Both { read: userdata, .. } => {
                                r_event.userdata = userdata;
                                r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                            }
                        }

                        r_event.error = Errno::__WASI_ERRNO_NOTSOCK.0;
                        mem.write_data(revents_num_ptr, 1)?;
                        return Ok(());
                    }
                }

                poller.poll(&mut events, None)?;

                handler_events(
                    ctx,
                    mem,
                    out_ptr,
                    nsubscriptions,
                    revents_num_ptr,
                    &events,
                    None,
                    &fd_vec,
                )?;
            }
        }
        PrePoll::ClockAndFd(clock, fd_vec) => {
            let mut poller = Poll::new()?;
            let mut events = Events::with_capacity(fd_vec.len());

            for (i, fd) in fd_vec.iter().enumerate() {
                if let VFD::Socket(s) = ctx.get_mut_vfd(fd.fd)? {
                    let interests = match fd.type_ {
                        net::SubscriptionFdType::Read(_) => Interest::READABLE,
                        net::SubscriptionFdType::Write(_) => Interest::WRITABLE,
                        net::SubscriptionFdType::Both { .. } => {
                            Interest::READABLE | Interest::WRITABLE
                        }
                    };

                    poller.registry().register(s, Token(i), interests)?;
                } else {
                    let r_event = mem.mut_data(out_ptr)?;
                    match fd.type_ {
                        net::SubscriptionFdType::Read(userdata) => {
                            r_event.userdata = userdata;
                            r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                        }
                        net::SubscriptionFdType::Write(userdata) => {
                            r_event.userdata = userdata;
                            r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_WRITE;
                        }
                        net::SubscriptionFdType::Both { read: userdata, .. } => {
                            r_event.userdata = userdata;
                            r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_FD_READ;
                        }
                    }

                    r_event.error = Errno::__WASI_ERRNO_NOTSOCK.0;
                    mem.write_data(revents_num_ptr, 1)?;
                    return Ok(());
                }
            }

            poller.poll(&mut events, clock.timeout)?;

            handler_events(
                ctx,
                mem,
                out_ptr,
                nsubscriptions,
                revents_num_ptr,
                &events,
                Some(clock),
                &fd_vec,
            )?;
        }
        PrePoll::OnlyClock(clock) => {
            if let Some(e) = clock.err {
                let r_event = mem.mut_data(out_ptr)?;
                r_event.userdata = clock.userdata;
                r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_CLOCK;
                r_event.error = Errno::from(e).0;
                mem.write_data(revents_num_ptr, 1)?;
                return Ok(());
            }
            if let Some(dur) = clock.timeout {
                std::thread::sleep(dur);
                let r_event = mem.mut_data(out_ptr)?;
                r_event.userdata = clock.userdata;
                r_event.type_ = __wasi_eventtype_t::__WASI_EVENTTYPE_CLOCK;
                mem.write_data(revents_num_ptr, 1)?;
                return Ok(());
            }
        }
    }

    Ok(())
}

#[cfg(windows)]
pub fn poll_oneoff<M: Memory>(
    _ctx: &mut WasiCtx,
    _mem: &mut M,
    _in_ptr: WasmPtr<__wasi_subscription_t>,
    _out_ptr: WasmPtr<__wasi_event_t>,
    _nsubscriptions: __wasi_size_t,
    _revents_num_ptr: WasmPtr<__wasi_size_t>,
) -> Result<(), Errno> {
    return Err(Errno::__WASI_ERRNO_NOSYS);
}
