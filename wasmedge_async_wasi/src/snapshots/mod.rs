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
    vfs_last_fd: usize,
    exit_code: u32,
}

impl WasiCtx {
    pub fn new() -> Self {
        WasiCtx {
            args: vec![],
            envs: vec![],
            vfs: vec![],
            vfs_preopen_limit: 3,
            vfs_last_fd: 3,
            exit_code: 0,
        }
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
        if let Some(vfs) = self.vfs.get_mut(self.vfs_last_fd..) {
            for entry in vfs {
                if entry.is_none() {
                    let _ = entry.insert(vfd);
                    return Ok(self.vfs_last_fd as __wasi_fd_t);
                }
                self.vfs_last_fd += 1;
            }
        }

        self.vfs.push(Some(vfd));
        self.vfs_last_fd = self.vfs.len() - 1;

        Ok(self.vfs_last_fd as __wasi_fd_t)
    }

    pub fn remove_vfd(&mut self, fd: __wasi_fd_t) -> Result<(), Errno> {
        if fd < 0 {
            return Err(Errno::__WASI_ERRNO_BADF);
        }
        let fd = fd as usize;

        if fd <= self.vfs_preopen_limit {
            return Err(Errno::__WASI_ERRNO_NOTSUP);
        }

        self.vfs_last_fd = fd;
        let fd = self.vfs.get_mut(fd).ok_or(Errno::__WASI_ERRNO_BADF)?;

        let _ = fd.take();
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
