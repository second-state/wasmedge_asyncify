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

            Ok(vfd)
        }
    }

    pub fn get_vfd(&self, fd: __wasi_fd_t) -> Result<&env::VFD, Errno> {
        if fd < 0 {
            Err(Errno::__WASI_ERRNO_BADF)
        } else {
            let t = self
                .vfs
                .get(fd as usize)
                .ok_or(Errno::__WASI_ERRNO_BADF)?
                .as_ref()
                .ok_or(Errno::__WASI_ERRNO_BADF)?;

            Ok(t)
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

    // todo: add test
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
