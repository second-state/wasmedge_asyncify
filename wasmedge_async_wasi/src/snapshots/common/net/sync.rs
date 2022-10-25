use super::*;
use crate::snapshots::common::types as wasi_types;
use crate::snapshots::common::vfs;
use crate::snapshots::env::Errno;
use socket2::Socket;

#[derive(Debug)]
pub struct SyncWasiSocket {
    pub inner: Socket,
    pub state: WasiSocketState,
}

#[cfg(unix)]
impl mio::event::Source for SyncWasiSocket {
    fn register(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
        interests: mio::Interest,
    ) -> io::Result<()> {
        use mio::unix::SourceFd;
        use std::os::unix::prelude::AsRawFd;
        let fd = self.inner.as_raw_fd();
        let mut source_fd = SourceFd(&fd);
        source_fd.register(registry, token, interests)
    }

    fn reregister(
        &mut self,
        registry: &mio::Registry,
        token: mio::Token,
        interests: mio::Interest,
    ) -> io::Result<()> {
        use mio::unix::SourceFd;
        use std::os::unix::prelude::AsRawFd;
        let fd = self.inner.as_raw_fd();
        let mut source_fd = SourceFd(&fd);
        source_fd.reregister(registry, token, interests)
    }

    fn deregister(&mut self, registry: &mio::Registry) -> io::Result<()> {
        use mio::unix::SourceFd;
        use std::os::unix::prelude::AsRawFd;
        let fd = self.inner.as_raw_fd();
        let mut source_fd = SourceFd(&fd);
        source_fd.deregister(registry)
    }
}

impl SyncWasiSocket {
    pub fn fd_fdstat_get(&self) -> Result<FdStat, Errno> {
        let mut filetype = match self.state.sock_type.1 {
            SocketType::Datagram => FileType::SOCKET_DGRAM,
            SocketType::Stream => FileType::SOCKET_STREAM,
        };
        let flags = if self.state.nonblocking {
            FdFlags::NONBLOCK
        } else {
            FdFlags::empty()
        };

        Ok(FdStat {
            filetype,
            fs_rights_base: self.state.fs_rights,
            fs_rights_inheriting: WASIRights::empty(),
            flags,
        })
    }
}

impl SyncWasiSocket {
    pub fn open(state: WasiSocketState) -> io::Result<Self> {
        use socket2::{Domain, Protocol, Type};
        let inner = match state.sock_type {
            (AddressFamily::Inet4, SocketType::Datagram) => {
                Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?
            }
            (AddressFamily::Inet4, SocketType::Stream) => {
                Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?
            }
            (AddressFamily::Inet6, SocketType::Datagram) => {
                Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?
            }
            (AddressFamily::Inet6, SocketType::Stream) => {
                Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP))?
            }
        };
        Ok(SyncWasiSocket { inner, state })
    }

    pub fn bind(&mut self, addr: net::SocketAddr) -> io::Result<()> {
        use socket2::SockAddr;
        let sock_addr = SockAddr::from(addr.clone());
        self.inner.bind(&sock_addr)?;
        self.state.fs_rights.remove(WASIRights::SOCK_BIND);
        self.state.local_addr.insert(addr);
        Ok(())
    }

    pub fn listen(&mut self, backlog: u32) -> io::Result<()> {
        self.inner.set_reuse_address(true)?;
        self.inner.listen(backlog as i32)?;
        self.state.backlog = backlog;
        self.state.so_accept_conn = true;
        Ok(())
    }

    pub fn accept(&self) -> io::Result<Self> {
        let (s, _) = self.inner.accept()?;
        let mut new_state = WasiSocketState::default();
        new_state.nonblocking = self.state.nonblocking;
        Ok(SyncWasiSocket {
            inner: s,
            state: new_state,
        })
    }

    pub fn connect(&mut self, addr: net::SocketAddr) -> io::Result<()> {
        use socket2::SockAddr;
        let address = SockAddr::from(addr.clone());
        self.inner.connect(&address)?;
        self.state.peer_addr = Some(addr);
        Ok(())
    }

    pub fn recv<'a>(
        &self,
        bufs: &mut [io::IoSliceMut<'a>],
        flags: libc::c_int,
    ) -> io::Result<(usize, bool)> {
        use socket2::MaybeUninitSlice;

        // Safety: reference Socket::read_vectored
        let bufs =
            unsafe { &mut *(bufs as *mut [io::IoSliceMut<'_>] as *mut [MaybeUninitSlice<'_>]) };

        let (n, f) = self.inner.recv_vectored_with_flags(bufs, flags)?;
        Ok((n, f.is_truncated()))
    }

    pub fn recv_from<'a>(
        &self,
        bufs: &mut [io::IoSliceMut<'a>],
        flags: libc::c_int,
    ) -> io::Result<(usize, bool, Option<net::SocketAddr>)> {
        use socket2::MaybeUninitSlice;

        // Safety: reference Socket::read_vectored
        let bufs =
            unsafe { &mut *(bufs as *mut [io::IoSliceMut<'_>] as *mut [MaybeUninitSlice<'_>]) };

        let (n, f, addr) = self.inner.recv_from_vectored_with_flags(bufs, flags)?;
        Ok((n, f.is_truncated(), addr.as_socket()))
    }

    pub fn send<'a>(&self, bufs: &[io::IoSlice<'a>], flags: libc::c_int) -> io::Result<usize> {
        self.inner.send_vectored_with_flags(bufs, flags)
    }

    pub fn send_to<'a>(
        &self,
        bufs: &[io::IoSlice<'a>],
        addr: net::SocketAddr,
        flags: libc::c_int,
    ) -> io::Result<usize> {
        use socket2::SockAddr;
        let address = SockAddr::from(addr);
        self.inner
            .send_to_vectored_with_flags(bufs, &address, flags)
    }

    pub fn shutdown(&mut self, how: net::Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)?;
        self.state.shutdown.insert(how);
        Ok(())
    }

    pub fn get_peer(&mut self) -> io::Result<net::SocketAddr> {
        if let Some(addr) = self.state.peer_addr {
            Ok(addr)
        } else {
            let addr = self.inner.peer_addr()?.as_socket().unwrap();
            self.state.peer_addr = Some(addr.clone());
            Ok(addr)
        }
    }

    pub fn get_local(&mut self) -> io::Result<net::SocketAddr> {
        if let Some(addr) = self.state.local_addr {
            Ok(addr)
        } else {
            let addr = self.inner.local_addr()?.as_socket().unwrap();
            self.state.local_addr = Some(addr.clone());
            Ok(addr)
        }
    }

    pub fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        self.inner.set_nonblocking(nonblocking)?;
        self.state.nonblocking = nonblocking;
        Ok(())
    }

    pub fn get_nonblocking(&self) -> bool {
        self.state.nonblocking
    }

    pub fn get_so_type(&self) -> (AddressFamily, SocketType) {
        self.state.sock_type
    }

    pub fn get_so_accept_conn(&self) -> io::Result<bool> {
        self.inner.is_listener()
    }

    pub fn set_so_reuseaddr(&mut self, reuseaddr: bool) -> io::Result<()> {
        self.inner.set_reuse_address(reuseaddr)?;
        self.state.so_reuseaddr = reuseaddr;
        Ok(())
    }

    pub fn get_so_reuseaddr(&self) -> bool {
        self.state.so_reuseaddr
    }

    pub fn set_so_recv_buf_size(&mut self, buf_size: usize) -> io::Result<()> {
        self.inner.set_recv_buffer_size(buf_size)?;
        self.state.so_recv_buf_size = buf_size;
        Ok(())
    }

    pub fn get_so_recv_buf_size(&self) -> usize {
        self.state.so_recv_buf_size
    }

    pub fn set_so_send_buf_size(&mut self, buf_size: usize) -> io::Result<()> {
        self.inner.set_send_buffer_size(buf_size)?;
        self.state.so_send_buf_size = buf_size;
        Ok(())
    }

    pub fn get_so_send_buf_size(&mut self) -> usize {
        self.state.so_send_buf_size
    }

    pub fn set_so_recv_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.inner.set_read_timeout(timeout)?;
        self.state.so_recv_timeout = timeout;
        Ok(())
    }

    pub fn get_so_recv_timeout(&mut self) -> Option<Duration> {
        self.state.so_recv_timeout
    }

    pub fn set_so_send_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.inner.set_write_timeout(timeout)?;
        self.state.so_send_timeout = timeout;
        Ok(())
    }

    pub fn get_so_send_timeout(&mut self) -> Option<Duration> {
        self.state.so_send_timeout
    }

    pub fn get_so_error(&mut self) -> io::Result<Option<io::Error>> {
        self.inner.take_error()
    }
}
