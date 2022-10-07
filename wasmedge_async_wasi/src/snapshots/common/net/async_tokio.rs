use std::ops::DerefMut;

use super::*;
use crate::snapshots::common::types as wasi_types;
use crate::snapshots::common::vfs;
use crate::snapshots::env::Errno;

use socket2::{SockAddr, Socket};
use std::os::unix::prelude::{AsRawFd, RawFd};
use tokio::io::unix::{AsyncFd, TryIoError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct AsyncWasiSocket {
    pub inner: AsyncFd<Socket>,
    pub state: WasiSocketState,
}

impl AsRawFd for AsyncWasiSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

#[inline]
fn handle_timeout_result<T>(
    result: Result<io::Result<T>, tokio::time::error::Elapsed>,
) -> io::Result<T> {
    if let Ok(r) = result {
        r
    } else {
        Err(io::Error::from_raw_os_error(libc::EWOULDBLOCK))
    }
}

impl AsyncWasiSocket {
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
        inner.set_nonblocking(true)?;
        Ok(AsyncWasiSocket {
            inner: AsyncFd::new(inner)?,
            state,
        })
    }

    pub fn bind(&mut self, addr: net::SocketAddr) -> io::Result<()> {
        use socket2::SockAddr;
        let sock_addr = SockAddr::from(addr.clone());
        self.inner.get_ref().bind(&sock_addr)?;
        self.state.local_addr.insert(addr);
        Ok(())
    }

    pub fn listen(&mut self, backlog: u32) -> io::Result<()> {
        let s = self.inner.get_ref();
        s.set_reuse_address(true);
        s.listen(backlog as i32)?;
        self.state.backlog = backlog;
        self.state.so_accept_conn = true;
        Ok(())
    }

    pub async fn accept(&mut self) -> io::Result<Self> {
        let mut new_state = WasiSocketState::default();
        new_state.nonblocking = self.state.nonblocking;

        if self.state.nonblocking {
            let (cs, _) = self.inner.get_ref().accept()?;
            cs.set_nonblocking(true)?;
            Ok(AsyncWasiSocket {
                inner: AsyncFd::new(cs)?,
                state: new_state,
            })
        } else {
            loop {
                let mut guard = self.inner.readable().await?;
                if let Ok(r) = guard.try_io(|s| {
                    let (cs, _) = s.get_ref().accept()?;
                    cs.set_nonblocking(true)?;
                    Ok(AsyncWasiSocket {
                        inner: AsyncFd::new(cs)?,
                        state: new_state,
                    })
                }) {
                    return r;
                } else {
                    continue;
                }
            }
        }
    }

    pub async fn connect(&mut self, addr: net::SocketAddr) -> io::Result<()> {
        let address = SockAddr::from(addr.clone());

        match (self.state.nonblocking, self.state.so_send_timeout) {
            (true, None) => {
                self.inner.get_ref().connect(&address)?;
                self.state.peer_addr = Some(addr);
                Ok(())
            }
            (false, None) => {
                if let Err(e) = self.inner.get_ref().connect(&address) {
                    match e.raw_os_error() {
                        Some(libc::EINPROGRESS) => {}
                        _ => return Err(e),
                    }
                    let _ = self.inner.writable().await?;
                    self.state.peer_addr = Some(addr);
                    Ok(())
                } else {
                    self.state.peer_addr = Some(addr);
                    Ok(())
                }
            }
            (_, Some(timeout)) => {
                if let Err(e) = self.inner.get_ref().connect(&address) {
                    match e.raw_os_error() {
                        Some(libc::EINPROGRESS) => {}
                        _ => return Err(e),
                    }
                    match tokio::time::timeout(timeout, self.inner.writable()).await {
                        Ok(r) => {
                            let _ = r?;
                            self.state.peer_addr = Some(addr);
                            Ok(())
                        }
                        Err(e) => Err(io::Error::from_raw_os_error(libc::EWOULDBLOCK)),
                    }
                } else {
                    self.state.peer_addr = Some(addr);
                    Ok(())
                }
            }
        }
    }

    pub async fn recv<'a>(
        &self,
        bufs: &mut [io::IoSliceMut<'a>],
        flags: libc::c_int,
    ) -> io::Result<(usize, bool)> {
        use socket2::MaybeUninitSlice;

        // Safety: reference Socket::read_vectored
        let bufs =
            unsafe { &mut *(bufs as *mut [io::IoSliceMut<'_>] as *mut [MaybeUninitSlice<'_>]) };

        match (self.state.nonblocking, self.state.so_recv_timeout) {
            (true, None) => {
                let (n, f) = self.inner.get_ref().recv_vectored_with_flags(bufs, flags)?;
                Ok((n, f.is_truncated()))
            }
            (false, None) => loop {
                let mut guard = self.inner.readable().await?;
                if let Ok(r) = guard.try_io(|s| {
                    let (n, f) = s.get_ref().recv_vectored_with_flags(bufs, flags)?;
                    Ok((n, f.is_truncated()))
                }) {
                    break r;
                } else {
                    continue;
                }
            },
            (_, Some(timeout)) => handle_timeout_result(
                tokio::time::timeout(timeout, async {
                    loop {
                        let mut guard = self.inner.readable().await?;
                        if let Ok(r) = guard.try_io(|s| {
                            let (n, f) = s.get_ref().recv_vectored_with_flags(bufs, flags)?;
                            Ok((n, f.is_truncated()))
                        }) {
                            break r;
                        } else {
                            continue;
                        }
                    }
                })
                .await,
            ),
        }
    }

    pub async fn recv_from<'a>(
        &self,
        bufs: &mut [io::IoSliceMut<'a>],
        flags: libc::c_int,
    ) -> io::Result<(usize, bool, Option<net::SocketAddr>)> {
        use socket2::MaybeUninitSlice;

        // Safety: reference Socket::read_vectored
        let bufs =
            unsafe { &mut *(bufs as *mut [io::IoSliceMut<'_>] as *mut [MaybeUninitSlice<'_>]) };

        match (self.state.nonblocking, self.state.so_recv_timeout) {
            (true, None) => {
                let (n, f, addr) = self
                    .inner
                    .get_ref()
                    .recv_from_vectored_with_flags(bufs, flags)?;
                Ok((n, f.is_truncated(), addr.as_socket()))
            }
            (false, None) => loop {
                let mut guard = self.inner.readable().await?;
                if let Ok(r) = guard.try_io(|s| {
                    let (n, f, addr) = s.get_ref().recv_from_vectored_with_flags(bufs, flags)?;
                    Ok((n, f.is_truncated(), addr.as_socket()))
                }) {
                    break r;
                } else {
                    continue;
                }
            },
            (_, Some(timeout)) => handle_timeout_result(
                tokio::time::timeout(timeout, async {
                    loop {
                        let mut guard = self.inner.readable().await?;
                        if let Ok(r) = guard.try_io(|s| {
                            let (n, f, addr) =
                                s.get_ref().recv_from_vectored_with_flags(bufs, flags)?;
                            Ok((n, f.is_truncated(), addr.as_socket()))
                        }) {
                            break r;
                        } else {
                            continue;
                        }
                    }
                })
                .await,
            ),
        }
    }

    pub async fn send<'a>(
        &self,
        bufs: &[io::IoSlice<'a>],
        flags: libc::c_int,
    ) -> io::Result<usize> {
        match (self.state.nonblocking, self.state.so_send_timeout) {
            (true, None) => self.inner.get_ref().send_vectored_with_flags(bufs, flags),
            (false, None) => loop {
                let mut guard = self.inner.writable().await?;
                if let Ok(r) = guard.try_io(|s| s.get_ref().send_vectored_with_flags(bufs, flags)) {
                    break r;
                } else {
                    continue;
                }
            },
            (_, Some(timeout)) => handle_timeout_result(
                tokio::time::timeout(timeout, async {
                    loop {
                        let mut guard = self.inner.writable().await?;
                        if let Ok(r) =
                            guard.try_io(|s| s.get_ref().send_vectored_with_flags(bufs, flags))
                        {
                            break r;
                        } else {
                            continue;
                        }
                    }
                })
                .await,
            ),
        }
    }

    pub async fn send_to<'a>(
        &self,
        bufs: &[io::IoSlice<'a>],
        addr: net::SocketAddr,
        flags: libc::c_int,
    ) -> io::Result<usize> {
        use socket2::{MaybeUninitSlice, SockAddr};
        let address = SockAddr::from(addr);

        match (self.state.nonblocking, self.state.so_send_timeout) {
            (true, None) => self
                .inner
                .get_ref()
                .send_to_vectored_with_flags(bufs, &address, flags),
            (false, None) => loop {
                let mut guard = self.inner.writable().await?;
                if let Ok(r) = guard.try_io(|s| {
                    s.get_ref()
                        .send_to_vectored_with_flags(bufs, &address, flags)
                }) {
                    break r;
                } else {
                    continue;
                }
            },
            (_, Some(timeout)) => handle_timeout_result(
                tokio::time::timeout(timeout, async {
                    loop {
                        let mut guard = self.inner.writable().await?;
                        if let Ok(r) = guard.try_io(|s| {
                            s.get_ref()
                                .send_to_vectored_with_flags(bufs, &address, flags)
                        }) {
                            break r;
                        } else {
                            continue;
                        }
                    }
                })
                .await,
            ),
        }
    }

    pub fn shutdown(&mut self, how: net::Shutdown) -> io::Result<()> {
        self.inner.get_ref().shutdown(how)?;
        self.state.shutdown.insert(how);
        Ok(())
    }

    pub fn get_peer(&mut self) -> io::Result<net::SocketAddr> {
        if let Some(addr) = self.state.peer_addr {
            Ok(addr)
        } else {
            let addr = self.inner.get_ref().peer_addr()?.as_socket().unwrap();
            self.state.peer_addr = Some(addr.clone());
            Ok(addr)
        }
    }

    pub fn get_local(&mut self) -> io::Result<net::SocketAddr> {
        if let Some(addr) = self.state.local_addr {
            Ok(addr)
        } else {
            let addr = self.inner.get_ref().local_addr()?.as_socket().unwrap();
            self.state.local_addr = Some(addr.clone());
            Ok(addr)
        }
    }

    pub fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
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
        self.inner.get_ref().is_listener()
    }

    pub fn set_so_reuseaddr(&mut self, reuseaddr: bool) -> io::Result<()> {
        self.state.so_reuseaddr = reuseaddr;
        Ok(())
    }

    pub fn get_so_reuseaddr(&self) -> bool {
        self.state.so_reuseaddr
    }

    pub fn set_so_recv_buf_size(&mut self, buf_size: usize) -> io::Result<()> {
        self.state.so_recv_buf_size = buf_size;
        Ok(())
    }

    pub fn get_so_recv_buf_size(&self) -> usize {
        self.state.so_recv_buf_size
    }

    pub fn set_so_send_buf_size(&mut self, buf_size: usize) -> io::Result<()> {
        self.state.so_send_buf_size = buf_size;
        Ok(())
    }

    pub fn get_so_send_buf_size(&mut self) -> usize {
        self.state.so_send_buf_size
    }

    pub fn set_so_recv_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.state.so_recv_timeout = timeout;
        self.state.nonblocking = true;
        Ok(())
    }

    pub fn get_so_recv_timeout(&mut self) -> Option<Duration> {
        self.state.so_recv_timeout
    }

    pub fn set_so_send_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.state.so_send_timeout = timeout;
        self.state.nonblocking = true;
        Ok(())
    }

    pub fn get_so_send_timeout(&mut self) -> Option<Duration> {
        self.state.so_send_timeout
    }

    pub fn get_so_error(&mut self) -> io::Result<Option<io::Error>> {
        self.inner.get_ref().take_error()
    }
}
