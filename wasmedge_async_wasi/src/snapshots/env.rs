pub use super::common::error::Errno;
pub use super::common::types as wasi_types;
pub use super::common::vfs;

#[cfg(all(unix, feature = "async_tokio"))]
pub use super::common::net::async_tokio::AsyncWasiSocket;

pub use super::common::net::sync::SyncWasiSocket;

pub enum VFD {
    Inode(vfs::INode),
    Socket(SyncWasiSocket),
    #[cfg(all(unix, feature = "async_tokio"))]
    AsyncSocket(AsyncWasiSocket),
}

impl VFD {
    pub fn is_socket(&self) -> bool {
        if let VFD::Socket(_) = self {
            true
        } else {
            false
        }
    }

    #[cfg(feature = "async_tokio")]
    pub fn is_async_socket(&self) -> bool {
        if let VFD::AsyncSocket(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_inode(&self) -> bool {
        if let VFD::Inode(_) = self {
            true
        } else {
            false
        }
    }
}

// pub trait AsVFD: Send + Sync + From<VFD> {
//     fn as_mut_vfd(&mut self) -> Option<&mut VFD>;
//     fn as_ref_vfd(&self) -> Option<&VFD>;
// }

// impl AsVFD for VFD {
//     fn as_mut_vfd(&mut self) -> Option<&mut VFD> {
//         Some(self)
//     }

//     fn as_ref_vfd(&self) -> Option<&VFD> {
//         Some(self)
//     }
// }

pub trait AsyncVM: Send + Sync {
    fn yield_now(&mut self) -> Result<(), Errno>;
}
