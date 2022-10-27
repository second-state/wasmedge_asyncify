pub mod common;
pub mod env;
pub mod preview_1;
use common::error::Errno;

use env::wasi_types::__wasi_fd_t;
use env::VFD;
pub struct WasiCtx {
    args: Vec<String>,
    envs: Vec<String>,
    vfs: Vec<Option<VFD>>,
    vfs_preopen_limit: usize,
    vfs_select_index: usize,
    vfs_last_index: usize,
    pub exit_code: u32,
}

impl WasiCtx {
    pub fn new() -> Self {
        let wasi_stdin = VFD::Inode(env::vfs::INode::Stdin(env::vfs::WasiStdin));
        let wasi_stdout = VFD::Inode(env::vfs::INode::Stdout(env::vfs::WasiStdout));
        let wasi_stderr = VFD::Inode(env::vfs::INode::Stderr(env::vfs::WasiStderr));

        let ctx = WasiCtx {
            args: vec![],
            envs: vec![],
            vfs: vec![Some(wasi_stdin), Some(wasi_stdout), Some(wasi_stderr)],
            vfs_preopen_limit: 2,
            vfs_select_index: 2,
            vfs_last_index: 2,
            exit_code: 0,
        };

        ctx
    }

    pub fn push_preopen(&mut self, preopen: env::vfs::WasiPreOpenDir) {
        self.vfs
            .push(Some(VFD::Inode(env::vfs::INode::PreOpenDir(preopen))));
        self.vfs_select_index = self.vfs.len() - 1;
        self.vfs_last_index = self.vfs_select_index;
        self.vfs_preopen_limit += 1;
    }

    pub fn push_arg(&mut self, arg: String) {
        self.args.push(arg);
    }

    pub fn push_env(&mut self, key: &str, value: &str) {
        self.envs.push(format!("{}={}", key, value));
    }

    pub fn get_mut_vfd(&mut self, fd: __wasi_fd_t) -> Result<&mut env::VFD, Errno> {
        if fd < 0 {
            Err(Errno::__WASI_ERRNO_BADF)
        } else {
            let vfd = self
                .vfs
                .get_mut(fd as usize)
                .ok_or(Errno::__WASI_ERRNO_BADF)?
                .as_mut()
                .ok_or(Errno::__WASI_ERRNO_BADF)?;
            if let VFD::Closed = vfd {
                return Err(Errno::__WASI_ERRNO_BADF);
            }
            Ok(vfd)
        }
    }

    pub fn get_vfd(&self, fd: __wasi_fd_t) -> Result<&env::VFD, Errno> {
        if fd < 0 {
            Err(Errno::__WASI_ERRNO_BADF)
        } else {
            let vfd = self
                .vfs
                .get(fd as usize)
                .ok_or(Errno::__WASI_ERRNO_BADF)?
                .as_ref()
                .ok_or(Errno::__WASI_ERRNO_BADF)?;
            if let VFD::Closed = vfd {
                return Err(Errno::__WASI_ERRNO_BADF);
            }
            Ok(vfd)
        }
    }

    pub fn insert_vfd(&mut self, vfd: VFD) -> Result<__wasi_fd_t, Errno> {
        debug_assert!(self.vfs_last_index < self.vfs.len(), "error last index");

        if let Some(vfs) = self.vfs.get_mut(self.vfs_select_index..) {
            for entry in vfs {
                if entry.is_none() {
                    let _ = entry.insert(vfd);
                    if self.vfs_select_index > self.vfs_last_index {
                        self.vfs_last_index = self.vfs_select_index;
                    }
                    return Ok(self.vfs_select_index as __wasi_fd_t);
                }
                self.vfs_select_index += 1;
            }
        }

        self.vfs.push(Some(vfd));
        self.vfs_select_index = self.vfs.len() - 1;
        self.vfs_last_index = self.vfs_select_index;

        Ok(self.vfs_select_index as __wasi_fd_t)
    }

    pub fn remove_vfd(&mut self, fd: __wasi_fd_t) -> Result<(), Errno> {
        debug_assert!(self.vfs_last_index < self.vfs.len(), "error last index");

        if fd <= self.vfs_preopen_limit as i32 {
            return Err(Errno::__WASI_ERRNO_NOTSUP);
        }

        let fd = fd as usize;

        let vfd = self.vfs.get_mut(fd).ok_or(Errno::__WASI_ERRNO_BADF)?;
        let _ = vfd.take();

        if fd != self.vfs_last_index {
            self.vfs_select_index = fd.min(self.vfs_select_index);
        } else {
            // find last not empty fd
            let mut i = self.vfs_last_index;
            loop {
                let vfd = &self.vfs[i];
                if vfd.is_some() {
                    self.vfs_last_index = i;
                    self.vfs_select_index = self.vfs_select_index.min(i);
                    break;
                } else {
                    i -= 1;
                }
            }
        }

        Ok(())
    }

    pub fn renumber_vfd(&mut self, from: __wasi_fd_t, to: __wasi_fd_t) -> Result<(), Errno> {
        if from < 0 || to < 0 {
            return Err(Errno::__WASI_ERRNO_BADF);
        }

        let to = to as usize;
        let from = from as usize;

        if from <= self.vfs_preopen_limit || to <= self.vfs_preopen_limit {
            return Err(Errno::__WASI_ERRNO_NOTSUP);
        };

        let from_entry = self.vfs.get_mut(from).ok_or(Errno::__WASI_ERRNO_BADF)?;
        let from_entry = from_entry.take();
        if from_entry.is_none() {
            return Err(Errno::__WASI_ERRNO_BADF);
        }

        if to > self.vfs.len() {
            self.vfs.resize_with(to, Default::default);
        }

        self.vfs.insert(to, from_entry);
        Ok(())
    }
}

#[cfg(test)]
mod vfs_test {
    use std::path::PathBuf;

    use super::env::*;
    use super::*;

    #[test]
    fn vfd_opt() {
        // [0,1,2]
        let mut ctx = WasiCtx::new();
        // [0,1,2,3(*)]
        ctx.push_preopen(vfs::WasiPreOpenDir::new(
            PathBuf::from("."),
            PathBuf::from("."),
        ));

        assert_eq!(ctx.vfs_preopen_limit, 3, "vfs_preopen_limit");

        fn vfd_stub() -> VFD {
            VFD::Inode(vfs::INode::Stdin(vfs::WasiStdin))
        }

        // [0,1,2,3,4(*)]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_last_index, 4, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");
        assert_eq!(
            ctx.vfs_select_index, fd as usize,
            "vfs_select_index == fd(4)"
        );

        // [0,1,2,3,4,5(*)]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_last_index, 5, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 5, "vfs_select_index");
        assert_eq!(
            ctx.vfs_select_index, fd as usize,
            "vfs_select_index == fd(5)"
        );

        // [0,1,2,3,none(*),5]
        ctx.remove_vfd(4).unwrap();
        assert_eq!(ctx.vfs_last_index, 5, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");

        // [0,1,2,3,4(*),5]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_last_index, 5, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");
        assert_eq!(
            ctx.vfs_select_index, fd as usize,
            "vfs_select_index == fd(4)"
        );

        // [0,1,2,3,none(*),5]
        ctx.remove_vfd(4).unwrap();
        assert_eq!(ctx.vfs_last_index, 5, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");

        // [0,1,2,3(*),none,none]
        ctx.remove_vfd(5).unwrap();
        assert_eq!(ctx.vfs_last_index, 3, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 3, "vfs_select_index");
        assert_eq!(ctx.vfs.len(), 6, "vfs.len()==6");

        // [0,1,2,3,4(*),none]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_last_index, 4, "vfs_last_index");
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");
        assert_eq!(
            ctx.vfs_select_index, fd as usize,
            "vfs_select_index == fd(4)"
        );
        assert_eq!(ctx.vfs.len(), 6, "vfs.len()==6");

        // [0,1,2,3,4,5(*)]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_select_index, 5, "vfs_select_index");
        assert_eq!(fd, 5, "fd==5");

        // [0,1,2,3,4,5,6(*)]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_select_index, 6, "vfs_select_index");
        assert_eq!(fd, 6, "fd==6");

        assert_eq!(ctx.vfs.len(), 7, "vfs.len()==7");

        // [0,1,2,3,4,none(*),6]
        ctx.remove_vfd(5).unwrap();
        assert_eq!(ctx.vfs_select_index, 5, "vfs_select_index");
        // [0,1,2,3,none(*),none,6]
        ctx.remove_vfd(4).unwrap();
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");

        // [0,1,2,3,4(*),none,6]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");
        assert_eq!(fd, 4, "fd==4");

        // [0,1,2,3,4,5(*),6]
        let fd = ctx.insert_vfd(vfd_stub()).unwrap();
        assert_eq!(ctx.vfs_select_index, 5, "vfs_select_index");
        assert_eq!(fd, 5, "fd==5");

        // [0,1,2,3,none(*),5,6]
        ctx.remove_vfd(4).unwrap();
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");
        assert_eq!(ctx.vfs_last_index, 6, "vfs_select_index");

        // [0,1,2,3,none(*),none,6]
        ctx.remove_vfd(5).unwrap();
        assert_eq!(ctx.vfs_select_index, 4, "vfs_select_index");
        assert_eq!(ctx.vfs_last_index, 6, "vfs_select_index");

        // [0,1,2,3(*),none,none,none]
        ctx.remove_vfd(6).unwrap();
        assert_eq!(ctx.vfs_select_index, 3, "vfs_select_index");
        assert_eq!(ctx.vfs_last_index, 3, "vfs_select_index");
        assert_eq!(ctx.vfs.len(), 7, "vfs.len()==7");
    }
}

#[cfg(feature = "serialize")]
pub mod serialize {
    use std::path::PathBuf;

    use super::common::net::async_tokio::AsyncWasiSocket;
    use super::common::net::{AddressFamily, SocketType, WasiSocketState};
    use super::common::vfs::{self, INode, WASIRights};
    use super::VFD;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct SerialWasiCtx {
        pub args: Vec<String>,
        pub envs: Vec<String>,
        pub vfs: Vec<SerialVFD>,
        pub vfs_preopen_limit: usize,
        pub vfs_select_index: usize,
        pub vfs_last_index: usize,
        pub exit_code: u32,
    }

    impl From<&super::WasiCtx> for SerialWasiCtx {
        fn from(ctx: &super::WasiCtx) -> Self {
            let vfs = ctx.vfs[0..=ctx.vfs_last_index]
                .iter()
                .map(|vfd| SerialVFD::from(vfd));
            Self {
                args: ctx.args.clone(),
                envs: ctx.envs.clone(),
                vfs: vfs.collect(),
                vfs_preopen_limit: ctx.vfs_preopen_limit,
                vfs_select_index: ctx.vfs_select_index,
                vfs_last_index: ctx.vfs_last_index,
                exit_code: ctx.exit_code,
            }
        }
    }

    impl SerialWasiCtx {
        pub fn resume(
            self,
            host_path_map_fn: fn(guest_path: &str) -> PathBuf,
        ) -> std::io::Result<super::WasiCtx> {
            let Self {
                args,
                envs,
                vfs,
                vfs_preopen_limit,
                vfs_select_index,
                vfs_last_index,
                exit_code,
            } = self;
            let mut wasi_vfs = Vec::with_capacity(vfs.len());
            for f in vfs {
                wasi_vfs.push(f.into_vfd(host_path_map_fn)?);
            }
            Ok(super::WasiCtx {
                args,
                envs,
                vfs: wasi_vfs,
                vfs_preopen_limit,
                vfs_select_index,
                vfs_last_index,
                exit_code,
            })
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub enum SerialSocketType {
        TCP4,
        TCP6,
        UDP4,
        UDP6,
    }

    impl From<(AddressFamily, SocketType)> for SerialSocketType {
        fn from(sock_type: (AddressFamily, SocketType)) -> Self {
            match sock_type {
                (AddressFamily::Inet4, SocketType::Datagram) => SerialSocketType::UDP4,
                (AddressFamily::Inet4, SocketType::Stream) => SerialSocketType::TCP4,
                (AddressFamily::Inet6, SocketType::Datagram) => SerialSocketType::UDP6,
                (AddressFamily::Inet6, SocketType::Stream) => SerialSocketType::TCP6,
            }
        }
    }

    impl Into<(AddressFamily, SocketType)> for SerialSocketType {
        fn into(self) -> (AddressFamily, SocketType) {
            match self {
                SerialSocketType::TCP4 => (AddressFamily::Inet4, SocketType::Stream),
                SerialSocketType::TCP6 => (AddressFamily::Inet6, SocketType::Stream),
                SerialSocketType::UDP4 => (AddressFamily::Inet4, SocketType::Datagram),
                SerialSocketType::UDP6 => (AddressFamily::Inet6, SocketType::Datagram),
            }
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    pub struct SerialWasiSocketState {
        pub sock_type: SerialSocketType,
        pub local_addr: Option<String>,
        pub peer_addr: Option<String>,
        pub backlog: u32,
        pub nonblocking: bool,
        pub so_reuseaddr: bool,
        pub so_accept_conn: bool,
        pub so_recv_buf_size: usize,
        pub so_send_buf_size: usize,
        pub so_recv_timeout: Option<u64>, // nano_sec
        pub so_send_timeout: Option<u64>, // nano_sec,
        pub fs_rights: u64,
    }

    impl From<&WasiSocketState> for SerialWasiSocketState {
        fn from(state: &WasiSocketState) -> Self {
            SerialWasiSocketState {
                sock_type: state.sock_type.into(),
                local_addr: state.local_addr.map(|addr| addr.to_string()),
                peer_addr: state.peer_addr.map(|addr| addr.to_string()),
                backlog: state.backlog,
                nonblocking: state.nonblocking,
                so_reuseaddr: state.so_reuseaddr,
                so_accept_conn: state.so_accept_conn,
                so_recv_buf_size: state.so_recv_buf_size,
                so_send_buf_size: state.so_send_buf_size,
                so_recv_timeout: state.so_recv_timeout.map(|d| d.as_nanos() as u64),
                so_send_timeout: state.so_send_timeout.map(|d| d.as_nanos() as u64),
                fs_rights: state.fs_rights.bits(),
            }
        }
    }

    impl Into<WasiSocketState> for SerialWasiSocketState {
        fn into(self) -> WasiSocketState {
            WasiSocketState {
                sock_type: self.sock_type.clone().into(),
                local_addr: self.local_addr.map(|s| s.parse().unwrap()),
                peer_addr: self.peer_addr.map(|s| s.parse().unwrap()),
                backlog: self.backlog,
                shutdown: None,
                nonblocking: self.nonblocking,
                so_reuseaddr: self.so_reuseaddr,
                so_accept_conn: self.so_accept_conn,
                so_recv_buf_size: self.so_recv_buf_size,
                so_send_buf_size: self.so_send_buf_size,
                so_recv_timeout: self
                    .so_recv_timeout
                    .map(|d| std::time::Duration::from_nanos(d)),
                so_send_timeout: self
                    .so_send_timeout
                    .map(|d| std::time::Duration::from_nanos(d)),
                fs_rights: WASIRights::from_bits_truncate(self.fs_rights),
            }
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(tag = "type")]
    pub enum SerialVFD {
        Empty,
        Std {
            fd: u8,
        },
        PreOpenDir {
            guest_path: String,
            dir_rights: u64,
            file_rights: u64,
        },
        WasiDir,
        WasiFile,
        Closed,
        TcpServer {
            state: SerialWasiSocketState,
        },
        UdpSocket {
            state: SerialWasiSocketState,
        },
    }

    impl From<&Option<VFD>> for SerialVFD {
        fn from(vfd: &Option<VFD>) -> Self {
            match vfd {
                Some(VFD::Closed) => Self::Closed,
                Some(VFD::Inode(INode::Dir(_))) => Self::WasiDir,
                Some(VFD::Inode(INode::File(_))) => Self::WasiFile,
                Some(VFD::Inode(INode::PreOpenDir(pre_open))) => {
                    let guest_path = format!("{}", pre_open.guest_path.display());
                    Self::PreOpenDir {
                        guest_path,
                        dir_rights: pre_open.dir_rights.bits(),
                        file_rights: pre_open.file_rights.bits(),
                    }
                }
                Some(VFD::Inode(INode::Stdin(_))) => Self::Std { fd: 0 },
                Some(VFD::Inode(INode::Stdout(_))) => Self::Std { fd: 1 },
                Some(VFD::Inode(INode::Stderr(_))) => Self::Std { fd: 2 },
                Some(VFD::AsyncSocket(AsyncWasiSocket { inner, state })) => match inner {
                    super::common::net::async_tokio::AsyncWasiSocketInner::PreOpen(_) => {
                        Self::Closed
                    }
                    super::common::net::async_tokio::AsyncWasiSocketInner::AsyncFd(_) => {
                        if state.shutdown.is_some() {
                            Self::Closed
                        } else {
                            let state: SerialWasiSocketState = state.into();
                            match state.sock_type {
                                SerialSocketType::TCP4 | SerialSocketType::TCP6 => {
                                    if state.so_accept_conn {
                                        Self::TcpServer { state }
                                    } else {
                                        Self::Closed
                                    }
                                }
                                SerialSocketType::UDP4 | SerialSocketType::UDP6 => {
                                    Self::UdpSocket { state }
                                }
                            }
                        }
                    }
                },
                None => Self::Empty,
            }
        }
    }

    impl SerialVFD {
        /// `host_path_map_fn` : return a `host_path` that `guest_path` should to map;
        pub fn into_vfd(
            self,
            host_path_map_fn: fn(guest_path: &str) -> PathBuf,
        ) -> std::io::Result<Option<VFD>> {
            let vfd = match self {
                SerialVFD::Empty => None,
                SerialVFD::Std { fd: 0 } => Some(VFD::Inode(INode::Stdin(vfs::WasiStdin))),
                SerialVFD::Std { fd: 1 } => Some(VFD::Inode(INode::Stdout(vfs::WasiStdout))),
                SerialVFD::Std { fd: 2 } => Some(VFD::Inode(INode::Stderr(vfs::WasiStderr))),
                SerialVFD::Std { .. } => None,
                SerialVFD::PreOpenDir {
                    guest_path,
                    dir_rights,
                    file_rights,
                } => {
                    let mut preopen = vfs::WasiPreOpenDir::new(
                        host_path_map_fn(&guest_path),
                        PathBuf::from(guest_path),
                    );
                    preopen.dir_rights = vfs::WASIRights::from_bits_truncate(dir_rights);
                    preopen.file_rights = vfs::WASIRights::from_bits_truncate(file_rights);
                    Some(VFD::Inode(INode::PreOpenDir(preopen)))
                }
                SerialVFD::TcpServer { state } => {
                    let mut vfd_state: WasiSocketState = state.into();
                    let local_addr = vfd_state
                        .local_addr
                        .ok_or(std::io::Error::from(std::io::ErrorKind::AddrNotAvailable))?;
                    let backlog = vfd_state.backlog.clamp(128, vfd_state.backlog);
                    vfd_state.backlog = backlog;

                    let mut async_socket = AsyncWasiSocket::open(vfd_state)?;
                    async_socket.bind(local_addr)?;
                    async_socket.listen(backlog)?;

                    Some(VFD::AsyncSocket(async_socket))
                }
                SerialVFD::UdpSocket { state } => {
                    let vfd_state: WasiSocketState = state.into();
                    let local_addr = vfd_state
                        .local_addr
                        .ok_or(std::io::Error::from(std::io::ErrorKind::AddrNotAvailable))?;

                    let mut async_socket = AsyncWasiSocket::open(vfd_state)?;
                    async_socket.bind(local_addr)?;

                    Some(VFD::AsyncSocket(async_socket))
                }
                _ => Some(VFD::Closed),
            };
            Ok(vfd)
        }
    }

    #[tokio::test]
    async fn test_json_serial() {
        use super::common::net;
        let mut wasi_ctx = super::WasiCtx::new();
        wasi_ctx.push_arg("abc".to_string());
        wasi_ctx.push_env("a", "1");
        wasi_ctx.push_preopen(vfs::WasiPreOpenDir::new(
            ".".parse().unwrap(),
            ".".parse().unwrap(),
        ));

        // tcp4
        let state = net::WasiSocketState::default();
        let mut s = net::async_tokio::AsyncWasiSocket::open(state).unwrap();
        s.bind("0.0.0.0:1234".parse().unwrap()).unwrap();
        s.listen(128).unwrap();
        wasi_ctx.insert_vfd(VFD::AsyncSocket(s)).unwrap();

        let state = net::WasiSocketState::default();
        let s = net::async_tokio::AsyncWasiSocket::open(state).unwrap();
        wasi_ctx.insert_vfd(VFD::AsyncSocket(s)).unwrap();

        let serial: SerialWasiCtx = (&wasi_ctx).into();

        let s = serde_json::to_string_pretty(&serial).unwrap();

        println!("{s}");
    }
}
