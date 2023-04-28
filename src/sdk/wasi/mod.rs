use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use crate::error::{CoreError, CoreExecutionError};
use crate::module::{AsyncWasmFn, ResultFuture, SyncWasmFn};
use crate::types::{ValType, WasmVal};
use crate::{types, ImportModule, Memory};
pub use wasmedge_async_wasi::snapshots;
use wasmedge_async_wasi::snapshots::common::memory::WasmPtr;
use wasmedge_async_wasi::snapshots::env::Errno;
use wasmedge_async_wasi::snapshots::preview_1 as p;
use wasmedge_async_wasi::snapshots::WasiCtx;

mod memory;

#[cfg(feature = "wasi_serialize")]
use serialize::IoState;
#[cfg(feature = "wasi_serialize")]
pub use wasmedge_async_wasi::snapshots::serialize;

pub struct AsyncWasiCtx {
    pub wasi_ctx: WasiCtx,
    /// (timeout,yield_hook):
    ///
    /// When `yield_hook` is not `None`, `accept()` and `poll_oneoff()` will wait a `Duration` (yield_hook.0),
    /// if (yield_hook.1) return true, runtime will yield.
    #[cfg(feature = "wasi_serialize")]
    pub yield_hook: Option<(std::time::Duration, fn(&IoState) -> bool)>,
}
pub struct AsyncWasiImport(ImportModule<AsyncWasiCtx>);
impl Deref for AsyncWasiImport {
    type Target = ImportModule<AsyncWasiCtx>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for AsyncWasiImport {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn to_wasm_return(r: Result<(), Errno>) -> Vec<types::WasmVal> {
    let code = if let Err(e) = r { e.0 } else { 0 };
    vec![types::WasmVal::I32(code as i32)]
}

#[inline]
fn func_type_miss_match_error() -> CoreError {
    log::trace!("FuncTypeMismatch");
    CoreError::Execution(CoreExecutionError::FuncTypeMismatch)
}

#[derive(Debug)]
pub struct NotDirError;

pub enum WasiFunc {
    SyncFn(
        String,
        (Vec<ValType>, Vec<ValType>),
        SyncWasmFn<AsyncWasiCtx>,
    ),
    AsyncFn(
        String,
        (Vec<ValType>, Vec<ValType>),
        AsyncWasmFn<AsyncWasiCtx>,
    ),
}

impl AsyncWasiImport {
    pub fn new() -> Option<Self> {
        Self::with_wasi_ctx(WasiCtx::new())
    }

    pub fn wasi_impls() -> Vec<WasiFunc> {
        macro_rules! sync_fn {
            ($name:expr,$ty:expr,$f:ident) => {
                WasiFunc::SyncFn($name.into(), $ty, $f)
            };
        }
        macro_rules! async_fn {
            ($name:expr,$ty:expr,$f:ident) => {
                WasiFunc::AsyncFn($name.into(), $ty, $f)
            };
        }
        let module = vec![
            sync_fn!(
                "args_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                args_get
            ),
            sync_fn!(
                "args_sizes_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                args_sizes_get
            ),
            sync_fn!(
                "environ_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                environ_get
            ),
            sync_fn!(
                "environ_sizes_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                environ_sizes_get
            ),
            sync_fn!(
                "clock_res_get",
                (vec![ValType::I32, ValType::I64], vec![ValType::I32]),
                clock_res_get
            ),
            sync_fn!(
                "clock_time_get",
                (
                    vec![ValType::I32, ValType::I64, ValType::I32],
                    vec![ValType::I32],
                ),
                clock_time_get
            ),
            sync_fn!(
                "random_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                random_get
            ),
            sync_fn!(
                "fd_prestat_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_prestat_get
            ),
            sync_fn!(
                "fd_prestat_dir_name",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_prestat_dir_name
            ),
            sync_fn!(
                "fd_renumber",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_renumber
            ),
            sync_fn!(
                "fd_advise",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_advise
            ),
            sync_fn!(
                "fd_allocate",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64],
                    vec![ValType::I32],
                ),
                fd_allocate
            ),
            sync_fn!(
                "fd_close",
                (vec![ValType::I32], vec![ValType::I32]),
                fd_close
            ),
            sync_fn!(
                "fd_seek",
                (
                    vec![ValType::I32, ValType::I64, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_seek
            ),
            sync_fn!("fd_sync", (vec![ValType::I32], vec![ValType::I32]), fd_sync),
            sync_fn!(
                "fd_datasync",
                (vec![ValType::I32], vec![ValType::I32]),
                fd_datasync
            ),
            sync_fn!(
                "fd_tell",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_tell
            ),
            sync_fn!(
                "fd_fdstat_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_fdstat_get
            ),
            sync_fn!(
                "fd_fdstat_set_flags",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_fdstat_set_flags
            ),
            sync_fn!(
                "fd_fdstat_set_rights",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64],
                    vec![ValType::I32],
                ),
                fd_fdstat_set_rights
            ),
            sync_fn!(
                "fd_filestat_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_filestat_get
            ),
            sync_fn!(
                "fd_filestat_set_size",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_filestat_set_size
            ),
            sync_fn!(
                "fd_filestat_set_times",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_filestat_set_times
            ),
            sync_fn!(
                "fd_read",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_read
            ),
            sync_fn!(
                "fd_pread",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                fd_pread
            ),
            sync_fn!(
                "fd_write",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_write
            ),
            sync_fn!(
                "fd_pwrite",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                fd_pwrite
            ),
            sync_fn!(
                "fd_readdir",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                fd_readdir
            ),
            sync_fn!(
                "path_create_directory",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                path_create_directory
            ),
            sync_fn!(
                "path_filestat_get",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_filestat_get
            ),
            sync_fn!(
                "path_filestat_set_times",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_filestat_set_times
            ),
            sync_fn!(
                "path_link",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_link
            ),
            sync_fn!(
                "path_open",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I64,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_open
            ),
            sync_fn!(
                "path_readlink",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_readlink
            ),
            sync_fn!(
                "path_remove_directory",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                path_remove_directory
            ),
            sync_fn!(
                "path_rename",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_rename
            ),
            sync_fn!(
                "path_rename",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_rename
            ),
            sync_fn!(
                "path_unlink_file",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                path_unlink_file
            ),
            sync_fn!("proc_exit", (vec![ValType::I32], vec![]), proc_exit),
            sync_fn!(
                "proc_raise",
                (vec![ValType::I32], vec![ValType::I32]),
                proc_raise
            ),
            sync_fn!("sched_yield", (vec![], vec![ValType::I32]), sched_yield),
            sync_fn!(
                "sock_open",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_open
            ),
            sync_fn!(
                "sock_bind",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_bind
            ),
            sync_fn!(
                "sock_listen",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                sock_listen
            ),
            async_fn!(
                "sock_accept",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                sock_accept
            ),
            async_fn!(
                "sock_connect",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_connect
            ),
            async_fn!(
                "sock_recv",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_recv
            ),
            async_fn!(
                "sock_recv_from",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_recv_from
            ),
            async_fn!(
                "sock_send",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_send
            ),
            async_fn!(
                "sock_send_to",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_send_to
            ),
            sync_fn!(
                "sock_shutdown",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                sock_shutdown
            ),
            sync_fn!(
                "sock_getpeeraddr",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_getpeeraddr
            ),
            sync_fn!(
                "sock_getlocaladdr",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_getlocaladdr
            ),
            sync_fn!(
                "sock_getsockopt",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_getsockopt
            ),
            sync_fn!(
                "sock_setsockopt",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_setsockopt
            ),
            async_fn!(
                "poll_oneoff",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                poll_oneoff
            ),
            async_fn!(
                "sock_lookup_ip",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_lookup_ip
            ),
        ];
        module
    }

    pub fn with_wasi_ctx_and_impls(wasi_ctx: WasiCtx, impls: Vec<WasiFunc>) -> Option<Self> {
        let wasi_ctx = AsyncWasiCtx {
            wasi_ctx,
            #[cfg(feature = "wasi_serialize")]
            yield_hook: None,
        };
        let mut module = ImportModule::create("wasi_snapshot_preview1", wasi_ctx).ok()?;
        for f in impls {
            match f {
                WasiFunc::SyncFn(name, ty, f) => {
                    module.add_sync_func(&name, ty, f).ok()?;
                }
                WasiFunc::AsyncFn(name, ty, f) => unsafe {
                    module.add_async_func_uncheck(&name, ty, f).ok()?;
                },
            }
        }
        Some(AsyncWasiImport(module))
    }

    pub fn with_wasi_ctx(wasi_ctx: WasiCtx) -> Option<Self> {
        let wasi_ctx = AsyncWasiCtx {
            wasi_ctx,
            #[cfg(feature = "wasi_serialize")]
            yield_hook: None,
        };
        let mut module = ImportModule::create("wasi_snapshot_preview1", wasi_ctx).ok()?;
        module
            .add_sync_func(
                "args_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                args_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "args_sizes_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                args_sizes_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "environ_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                environ_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "environ_sizes_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                environ_sizes_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "clock_res_get",
                (vec![ValType::I32, ValType::I64], vec![ValType::I32]),
                clock_res_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "clock_time_get",
                (
                    vec![ValType::I32, ValType::I64, ValType::I32],
                    vec![ValType::I32],
                ),
                clock_time_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "random_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                random_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_prestat_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_prestat_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_prestat_dir_name",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_prestat_dir_name,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_renumber",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_renumber,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_advise",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_advise,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_allocate",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64],
                    vec![ValType::I32],
                ),
                fd_allocate,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_close",
                (vec![ValType::I32], vec![ValType::I32]),
                fd_close,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_seek",
                (
                    vec![ValType::I32, ValType::I64, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_seek,
            )
            .ok()?;
        module
            .add_sync_func("fd_sync", (vec![ValType::I32], vec![ValType::I32]), fd_sync)
            .ok()?;
        module
            .add_sync_func(
                "fd_datasync",
                (vec![ValType::I32], vec![ValType::I32]),
                fd_datasync,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_tell",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_tell,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_fdstat_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_fdstat_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_fdstat_set_flags",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_fdstat_set_flags,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_fdstat_set_rights",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64],
                    vec![ValType::I32],
                ),
                fd_fdstat_set_rights,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_filestat_get",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_filestat_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_filestat_set_size",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                fd_filestat_set_size,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_filestat_set_times",
                (
                    vec![ValType::I32, ValType::I64, ValType::I64, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_filestat_set_times,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_read",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_read,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_pread",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                fd_pread,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_write",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                fd_write,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_pwrite",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                fd_pwrite,
            )
            .ok()?;
        module
            .add_sync_func(
                "fd_readdir",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                fd_readdir,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_create_directory",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                path_create_directory,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_filestat_get",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_filestat_get,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_filestat_set_times",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I64,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_filestat_set_times,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_link",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_link,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_open",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I64,
                        ValType::I64,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_open,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_readlink",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_readlink,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_remove_directory",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                path_remove_directory,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_rename",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_rename,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_rename",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                path_rename,
            )
            .ok()?;
        module
            .add_sync_func(
                "path_unlink_file",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                path_unlink_file,
            )
            .ok()?;
        module
            .add_sync_func("proc_exit", (vec![ValType::I32], vec![]), proc_exit)
            .ok()?;
        module
            .add_sync_func(
                "proc_raise",
                (vec![ValType::I32], vec![ValType::I32]),
                proc_raise,
            )
            .ok()?;
        module
            .add_sync_func("sched_yield", (vec![], vec![ValType::I32]), sched_yield)
            .ok()?;
        module
            .add_sync_func(
                "sock_open",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_open,
            )
            .ok()?;
        module
            .add_sync_func(
                "sock_bind",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_bind,
            )
            .ok()?;

        module
            .add_sync_func(
                "sock_listen",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                sock_listen,
            )
            .ok()?;
        unsafe {
            module
                .add_async_func_uncheck(
                    "sock_accept",
                    (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                    sock_accept,
                )
                .ok()?;
            module
                .add_async_func_uncheck(
                    "sock_connect",
                    (
                        vec![ValType::I32, ValType::I32, ValType::I32],
                        vec![ValType::I32],
                    ),
                    sock_connect,
                )
                .ok()?;
            module
                .add_async_func_uncheck(
                    "sock_recv",
                    (
                        vec![
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                        ],
                        vec![ValType::I32],
                    ),
                    sock_recv,
                )
                .ok()?;
            module
                .add_async_func_uncheck(
                    "sock_recv_from",
                    (
                        vec![
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                        ],
                        vec![ValType::I32],
                    ),
                    sock_recv_from,
                )
                .ok()?;
            module
                .add_async_func_uncheck(
                    "sock_send",
                    (
                        vec![
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                        ],
                        vec![ValType::I32],
                    ),
                    sock_send,
                )
                .ok()?;
            module
                .add_async_func_uncheck(
                    "sock_send_to",
                    (
                        vec![
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                        ],
                        vec![ValType::I32],
                    ),
                    sock_send_to,
                )
                .ok()?;
        }
        module
            .add_sync_func(
                "sock_shutdown",
                (vec![ValType::I32, ValType::I32], vec![ValType::I32]),
                sock_shutdown,
            )
            .ok()?;
        module
            .add_sync_func(
                "sock_getpeeraddr",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_getpeeraddr,
            )
            .ok()?;
        module
            .add_sync_func(
                "sock_getlocaladdr",
                (
                    vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                    vec![ValType::I32],
                ),
                sock_getlocaladdr,
            )
            .ok()?;
        module
            .add_sync_func(
                "sock_getsockopt",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_getsockopt,
            )
            .ok()?;
        module
            .add_sync_func(
                "sock_setsockopt",
                (
                    vec![
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    vec![ValType::I32],
                ),
                sock_setsockopt,
            )
            .ok()?;
        unsafe {
            module
                .add_async_func_uncheck(
                    "poll_oneoff",
                    (
                        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                        vec![ValType::I32],
                    ),
                    poll_oneoff,
                )
                .ok()?;
            module
                .add_async_func_uncheck(
                    "sock_lookup_ip",
                    (
                        vec![
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                            ValType::I32,
                        ],
                        vec![ValType::I32],
                    ),
                    sock_lookup_ip,
                )
                .ok()?;
        }
        Some(AsyncWasiImport(module))
    }

    pub fn push_preopen(
        &mut self,
        host_path: PathBuf,
        guest_path: PathBuf,
    ) -> Result<(), NotDirError> {
        use wasmedge_async_wasi::snapshots::common::vfs::WasiPreOpenDir;

        let dir_meta = std::fs::metadata(&host_path).or(Err(NotDirError))?;

        if !dir_meta.is_dir() {
            return Err(NotDirError);
        }

        self.0
            .data
            .wasi_ctx
            .push_preopen(WasiPreOpenDir::new(host_path, guest_path));
        Ok(())
    }

    pub fn push_arg(&mut self, arg: String) {
        self.0.data.wasi_ctx.push_arg(arg);
    }

    pub fn push_env(&mut self, key: &str, value: &str) {
        self.0.data.wasi_ctx.push_env(key, value);
    }
}

pub fn args_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("args_get enter");

    if let Some([WasmVal::I32(argv), WasmVal::I32(argv_buf)]) = args.get(0..2) {
        let argv = *argv as usize;
        let argv_buf = *argv_buf as usize;
        Ok(to_wasm_return(p::args_get(
            &mut ctx.wasi_ctx,
            mem,
            WasmPtr::from(argv),
            WasmPtr::from(argv_buf),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn args_sizes_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("args_sizes_get enter");
    if let Some([WasmVal::I32(argc), WasmVal::I32(argv_buf_size)]) = args.get(0..2) {
        let argc = *argc as usize;
        let argv_buf_size = *argv_buf_size as usize;
        Ok(to_wasm_return(p::args_sizes_get(
            &mut ctx.wasi_ctx,
            mem,
            WasmPtr::from(argc),
            WasmPtr::from(argv_buf_size),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn environ_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("environ_get enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let environ = *p1 as usize;
        let environ_buf = *p2 as usize;
        Ok(to_wasm_return(p::environ_get(
            &mut ctx.wasi_ctx,
            mem,
            WasmPtr::from(environ),
            WasmPtr::from(environ_buf),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn environ_sizes_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("environ_sizes_get enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let environ_count = *p1 as usize;
        let environ_buf_size = *p2 as usize;
        Ok(to_wasm_return(p::environ_sizes_get(
            &mut ctx.wasi_ctx,
            mem,
            WasmPtr::from(environ_count),
            WasmPtr::from(environ_buf_size),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn clock_res_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("clock_res_get enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let clock_id = *p1 as u32;
        let resolution_ptr = *p2 as usize;
        Ok(to_wasm_return(p::clock_res_get(
            &mut ctx.wasi_ctx,
            mem,
            clock_id,
            WasmPtr::from(resolution_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn clock_time_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("clock_time_get enter");
    if let Some([WasmVal::I32(p1), WasmVal::I64(p2), WasmVal::I32(p3)]) = args.get(0..3) {
        let clock_id = *p1 as u32;
        let precision = *p2 as u64;
        let time_ptr = *p3 as usize;

        Ok(to_wasm_return(p::clock_time_get(
            &mut ctx.wasi_ctx,
            mem,
            clock_id,
            precision,
            WasmPtr::from(time_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn random_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("random_get enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let buf = *p1 as usize;
        let buf_len = *p2 as u32;

        Ok(to_wasm_return(p::random_get(
            &mut ctx.wasi_ctx,
            mem,
            WasmPtr::from(buf),
            buf_len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_prestat_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_prestat_get enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let fd = *p1;
        let prestat_ptr = *p2 as usize;

        Ok(to_wasm_return(p::fd_prestat_get(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(prestat_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_prestat_dir_name<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_prestat_dir_name enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..3) {
        let fd = *p1;
        let path_buf_ptr = *p2 as usize;
        let path_max_len = *p3 as u32;

        Ok(to_wasm_return(p::fd_prestat_dir_name(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(path_buf_ptr),
            path_max_len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_renumber<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_renumber enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let from = *p1;
        let to = *p2;

        Ok(to_wasm_return(p::fd_renumber(
            &mut ctx.wasi_ctx,
            mem,
            from,
            to,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_advise<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_advise enter");
    if let Some([WasmVal::I32(p1), WasmVal::I64(p2), WasmVal::I64(p3), WasmVal::I32(p4)]) =
        args.get(0..4)
    {
        let fd = *p1;
        let offset = *p2 as u64;
        let len = *p3 as u64;
        let advice = *p4 as u8;

        Ok(to_wasm_return(p::fd_advise(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            offset,
            len,
            advice,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_allocate<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_allocate enter");
    if let Some([WasmVal::I32(p1), WasmVal::I64(p2), WasmVal::I64(p3)]) = args.get(0..3) {
        let fd = *p1;
        let offset = *p2 as u64;
        let len = *p3 as u64;

        Ok(to_wasm_return(p::fd_allocate(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            offset,
            len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_close<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_close enter");
    if let Some([WasmVal::I32(p1)]) = args.get(0..1) {
        let fd = *p1;

        Ok(to_wasm_return(p::fd_close(&mut ctx.wasi_ctx, mem, fd)))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_seek<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_seek enter");
    let n = 4;
    if let Some([WasmVal::I32(p1), WasmVal::I64(p2), WasmVal::I32(p3), WasmVal::I32(p4)]) =
        args.get(0..n)
    {
        let fd = *p1;
        let offset = *p2;
        let whence = *p3 as u8;
        let newoffset_ptr = *p4 as usize;

        Ok(to_wasm_return(p::fd_seek(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            offset,
            whence,
            WasmPtr::from(newoffset_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_sync<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_sync enter");
    if let Some([WasmVal::I32(p1)]) = args.get(0..1) {
        let fd = *p1;

        Ok(to_wasm_return(p::fd_sync(&mut ctx.wasi_ctx, mem, fd)))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_datasync<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_datasync enter");
    if let Some([WasmVal::I32(p1)]) = args.get(0..1) {
        let fd = *p1;

        Ok(to_wasm_return(p::fd_datasync(&mut ctx.wasi_ctx, mem, fd)))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_tell<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_tell enter");
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..2) {
        let fd = *p1;
        let offset = *p2 as usize;

        Ok(to_wasm_return(p::fd_tell(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(offset),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_fdstat_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_fdstat_get enter");
    let n = 2;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
        let fd = *p1;
        let buf_ptr = *p2 as usize;

        Ok(to_wasm_return(p::fd_fdstat_get(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(buf_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_fdstat_set_flags<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_fdstat_set_flags enter");
    let n = 2;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
        let fd = *p1;
        let flags = *p2 as u16;

        Ok(to_wasm_return(p::fd_fdstat_set_flags(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            flags,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_fdstat_set_rights<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_fdstat_set_rights enter");
    let n = 3;
    if let Some([WasmVal::I32(p1), WasmVal::I64(p2), WasmVal::I64(p3)]) = args.get(0..n) {
        let fd = *p1;
        let fs_rights_base = *p2 as u64;
        let fs_rights_inheriting = *p3 as u64;

        Ok(to_wasm_return(p::fd_fdstat_set_rights(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            fs_rights_base,
            fs_rights_inheriting,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_filestat_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_filestat_get enter");
    let n = 2;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
        let fd = *p1;
        let buf = *p2 as usize;

        Ok(to_wasm_return(p::fd_filestat_get(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(buf),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_filestat_set_size<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_filestat_set_size enter");
    let n = 2;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
        let fd = *p1;
        let buf = *p2 as usize;

        Ok(to_wasm_return(p::fd_filestat_get(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(buf),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_filestat_set_times<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_filestat_set_times enter");
    let n = 4;
    if let Some([WasmVal::I32(p1), WasmVal::I64(p2), WasmVal::I64(p3), WasmVal::I32(p4)]) =
        args.get(0..n)
    {
        let fd = *p1;
        let st_atim = *p2 as u64;
        let st_mtim = *p3 as u64;
        let fst_flags = *p4 as u16;

        Ok(to_wasm_return(p::fd_filestat_set_times(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            st_atim,
            st_mtim,
            fst_flags,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_read<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_read enter");
    let n = 4;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4)]) =
        args.get(0..n)
    {
        let fd = *p1;
        let iovs = *p2 as usize;
        let iovs_len = *p3 as u32;
        let nread = *p4 as usize;

        Ok(to_wasm_return(p::fd_read(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(iovs),
            iovs_len,
            WasmPtr::from(nread),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_pread<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_pread enter");
    let n = 5;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I64(p4), WasmVal::I32(p5)],
    ) = args.get(0..n)
    {
        let fd = *p1;
        let iovs = *p2 as usize;
        let iovs_len = *p3 as u32;
        let offset = *p4 as u64;
        let nread = *p5 as usize;

        Ok(to_wasm_return(p::fd_pread(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(iovs),
            iovs_len,
            offset,
            WasmPtr::from(nread),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_write<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_write enter");
    let n = 4;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4)]) =
        args.get(0..n)
    {
        let fd = *p1;
        let iovs = *p2 as usize;
        let iovs_len = *p3 as u32;
        let nwritten = *p4 as usize;

        Ok(to_wasm_return(p::fd_write(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(iovs),
            iovs_len,
            WasmPtr::from(nwritten),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_pwrite<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_pwrite enter");
    let n = 5;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I64(p4), WasmVal::I32(p5)],
    ) = args.get(0..n)
    {
        let fd = *p1;
        let iovs = *p2 as usize;
        let iovs_len = *p3 as u32;
        let offset = *p4 as u64;
        let nwritten = *p5 as usize;

        Ok(to_wasm_return(p::fd_pwrite(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(iovs),
            iovs_len,
            offset,
            WasmPtr::from(nwritten),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn fd_readdir<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("fd_readdir enter");
    let n = 5;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I64(p4), WasmVal::I32(p5)],
    ) = args.get(0..n)
    {
        let fd = *p1;
        let buf = *p2 as usize;
        let buf_len = *p3 as u32;
        let cookie = *p4 as u64;
        let bufused_ptr = *p5 as usize;

        Ok(to_wasm_return(p::fd_readdir(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(buf),
            buf_len,
            cookie,
            WasmPtr::from(bufused_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn path_create_directory<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_create_directory enter");
    let n = 3;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..n) {
        let dirfd = *p1;
        let path_ptr = *p2 as usize;
        let path_len = *p3 as u32;

        Ok(to_wasm_return(p::path_create_directory(
            &mut ctx.wasi_ctx,
            mem,
            dirfd,
            WasmPtr::from(path_ptr),
            path_len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn path_filestat_get<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_filestat_get enter");
    let n = 5;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5)],
    ) = args.get(0..n)
    {
        let fd = *p1;
        let flags = *p2 as u32;
        let path_ptr = *p3 as usize;
        let path_len = *p4 as u32;
        let file_stat_ptr = *p5 as usize;

        Ok(to_wasm_return(p::path_filestat_get(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            flags,
            WasmPtr::from(path_ptr),
            path_len,
            WasmPtr::from(file_stat_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn path_filestat_set_times<'a, T>(
    _: &'a mut T,
    _mem: &'a mut Memory,
    _ctx: &'a mut AsyncWasiCtx,
    _args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_filestat_set_times enter");
    Ok(vec![WasmVal::I32(Errno::__WASI_ERRNO_NOSYS.0 as i32)])
}

pub fn path_link<'a, T>(
    _: &'a mut T,
    _mem: &'a mut Memory,
    _ctx: &'a mut AsyncWasiCtx,
    _args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_link enter");
    Ok(vec![WasmVal::I32(Errno::__WASI_ERRNO_NOSYS.0 as i32)])
}

pub fn path_open<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_open enter");
    let n = 9;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5), WasmVal::I64(p6), WasmVal::I64(p7), WasmVal::I32(p8), WasmVal::I32(p9)],
    ) = args.get(0..n)
    {
        let dirfd = *p1;
        let dirflags = *p2 as u32;
        let path = *p3 as usize;
        let path_len = *p4 as u32;
        let o_flags = *p5 as u16;
        let fs_rights_base = *p6 as u64;
        let fs_rights_inheriting = *p7 as u64;
        let fs_flags = *p8 as u16;
        let fd_ptr = *p9 as usize;

        Ok(to_wasm_return(p::path_open(
            &mut ctx.wasi_ctx,
            mem,
            dirfd,
            dirflags,
            WasmPtr::from(path),
            path_len,
            o_flags,
            fs_rights_base,
            fs_rights_inheriting,
            fs_flags,
            WasmPtr::from(fd_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn path_readlink<'a, T>(
    _: &'a mut T,
    _mem: &'a mut Memory,
    _ctx: &'a mut AsyncWasiCtx,
    _args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_readlink enter");
    Ok(vec![WasmVal::I32(Errno::__WASI_ERRNO_NOSYS.0 as i32)])
}

pub fn path_remove_directory<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_remove_directory enter");
    let n = 3;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..n) {
        let fd = *p1;
        let path_ptr = *p2 as usize;
        let path_len = *p3 as u32;

        Ok(to_wasm_return(p::path_remove_directory(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(path_ptr),
            path_len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn path_rename<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_rename enter");
    let n = 6;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5), WasmVal::I32(p6)],
    ) = args.get(0..n)
    {
        let old_fd = *p1;
        let old_path = *p2 as usize;
        let old_path_len = *p3 as u32;
        let new_fd = *p4;
        let new_path = *p5 as usize;
        let new_path_len = *p6 as u32;

        Ok(to_wasm_return(p::path_rename(
            &mut ctx.wasi_ctx,
            mem,
            old_fd,
            WasmPtr::from(old_path),
            old_path_len,
            new_fd,
            WasmPtr::from(new_path),
            new_path_len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn path_symlink<'a, T>(
    _: &'a mut T,
    _mem: &'a mut Memory,
    _ctx: &'a mut AsyncWasiCtx,
    _args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_symlink enter");
    Ok(vec![WasmVal::I32(Errno::__WASI_ERRNO_NOSYS.0 as i32)])
}

pub fn path_unlink_file<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("path_unlink_file enter");
    let n = 3;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..n) {
        let fd = *p1;
        let path_ptr = *p2 as usize;
        let path_len = *p3 as u32;

        Ok(to_wasm_return(p::path_unlink_file(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(path_ptr),
            path_len,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn proc_exit<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("proc_exit enter");
    let n = 1;
    if let Some([WasmVal::I32(p1)]) = args.get(0..n) {
        let code = *p1 as u32;
        p::proc_exit(&mut ctx.wasi_ctx, mem, code);
        Err(CoreError::terminated())
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn proc_raise<'a, T>(
    _: &'a mut T,
    _mem: &'a mut Memory,
    _ctx: &'a mut AsyncWasiCtx,
    _args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("proc_raise enter");
    Ok(vec![WasmVal::I32(Errno::__WASI_ERRNO_NOSYS.0 as i32)])
}

// todo: ld asyncify yield
pub fn sched_yield<'a, T>(
    _: &'a mut T,
    _mem: &'a mut Memory,
    _ctx: &'a mut AsyncWasiCtx,
    _args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sched_yield enter");
    Ok(vec![WasmVal::I32(Errno::__WASI_ERRNO_NOSYS.0 as i32)])
}

//socket

pub fn sock_open<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_open enter");
    let n = 3;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..n) {
        let af = *p1 as u8;
        let ty = *p2 as u8;
        let ro_fd_ptr = *p3 as usize;

        Ok(to_wasm_return(p::async_socket::sock_open(
            &mut ctx.wasi_ctx,
            mem,
            af,
            ty,
            WasmPtr::from(ro_fd_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_bind<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_bind enter");
    let n = 3;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..n) {
        let fd = *p1;
        let addr_ptr = *p2 as usize;
        let port = *p3 as u32;
        Ok(to_wasm_return(p::async_socket::sock_bind(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(addr_ptr),
            port,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_listen<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_listen enter");
    let n = 2;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
        let fd = *p1;
        let backlog = *p2 as u32;

        Ok(to_wasm_return(p::async_socket::sock_listen(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            backlog,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_accept<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_accept enter");
    Box::new(async move {
        let n = 2;
        if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
            let fd = *p1;
            let ro_fd_ptr = *p2 as usize;

            #[cfg(feature = "wasi_serialize")]
            if let Some((dur, hook)) = ctx.yield_hook {
                'a: loop {
                    let accept_fut = p::async_socket::sock_accept(
                        &mut ctx.wasi_ctx,
                        mem,
                        fd,
                        WasmPtr::from(ro_fd_ptr),
                    );
                    if let Ok(r) = tokio::time::timeout(dur, accept_fut).await {
                        return Ok(to_wasm_return(r));
                    } else {
                        if hook(&ctx.wasi_ctx.io_state) {
                            return Err(CoreError::Yield);
                        } else {
                            continue 'a;
                        }
                    }
                }
            }

            Ok(to_wasm_return(
                p::async_socket::sock_accept(&mut ctx.wasi_ctx, mem, fd, WasmPtr::from(ro_fd_ptr))
                    .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_connect<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_connect enter");
    Box::new(async move {
        let n = 3;
        if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3)]) = args.get(0..n) {
            let fd = *p1;
            let addr_ptr = *p2 as usize;
            let port = *p3 as u32;

            Ok(to_wasm_return(
                p::async_socket::sock_connect(
                    &mut ctx.wasi_ctx,
                    mem,
                    fd,
                    WasmPtr::from(addr_ptr),
                    port,
                )
                .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_recv<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_recv enter");
    Box::new(async move {
        let n = 6;
        if let Some(
            [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5), WasmVal::I32(p6)],
        ) = args.get(0..n)
        {
            let fd = *p1;
            let buf_ptr = *p2 as usize;
            let buf_len = *p3 as u32;
            let flags = *p4 as u16;
            let ro_data_len_ptr = *p5 as usize;
            let ro_flags_ptr = *p6 as usize;

            Ok(to_wasm_return(
                p::async_socket::sock_recv(
                    &mut ctx.wasi_ctx,
                    mem,
                    fd,
                    WasmPtr::from(buf_ptr),
                    buf_len,
                    flags,
                    WasmPtr::from(ro_data_len_ptr),
                    WasmPtr::from(ro_flags_ptr),
                )
                .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_recv_from<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_recv_from enter");
    Box::new(async move {
        let n = 8;
        if let Some(
            [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5), WasmVal::I32(p6), WasmVal::I32(p7), WasmVal::I32(p8)],
        ) = args.get(0..n)
        {
            let fd = *p1;
            let buf_ptr = *p2 as usize;
            let buf_len = *p3 as u32;
            let wasi_addr_ptr = *p4 as usize;
            let port_ptr = *p5 as usize;
            let flags = *p6 as u16;
            let ro_data_len_ptr = *p7 as usize;
            let ro_flags_ptr = *p8 as usize;

            Ok(to_wasm_return(
                p::async_socket::sock_recv_from(
                    &mut ctx.wasi_ctx,
                    mem,
                    fd,
                    WasmPtr::from(buf_ptr),
                    buf_len,
                    WasmPtr::from(wasi_addr_ptr),
                    WasmPtr::from(port_ptr),
                    flags,
                    WasmPtr::from(ro_data_len_ptr),
                    WasmPtr::from(ro_flags_ptr),
                )
                .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_send<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_send enter");
    Box::new(async move {
        let n = 5;
        if let Some(
            [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5)],
        ) = args.get(0..n)
        {
            let fd = *p1;
            let buf_ptr = *p2 as usize;
            let buf_len = *p3 as u32;
            let flags = *p4 as u16;
            let send_len_ptr = *p5 as usize;

            Ok(to_wasm_return(
                p::async_socket::sock_send(
                    &mut ctx.wasi_ctx,
                    mem,
                    fd,
                    WasmPtr::from(buf_ptr),
                    buf_len,
                    flags,
                    WasmPtr::from(send_len_ptr),
                )
                .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_send_to<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_send_to enter");
    Box::new(async move {
        let n = 7;
        if let Some(
            [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5), WasmVal::I32(p6), WasmVal::I32(p7)],
        ) = args.get(0..n)
        {
            let fd = *p1;
            let buf_ptr = *p2 as usize;
            let buf_len = *p3 as u32;
            let wasi_addr_ptr = *p4 as usize;
            let port = *p5 as u32;
            let flags = *p6 as u16;
            let send_len_ptr = *p7 as usize;

            Ok(to_wasm_return(
                p::async_socket::sock_send_to(
                    &mut ctx.wasi_ctx,
                    mem,
                    fd,
                    WasmPtr::from(buf_ptr),
                    buf_len,
                    WasmPtr::from(wasi_addr_ptr),
                    port,
                    flags,
                    WasmPtr::from(send_len_ptr),
                )
                .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_shutdown<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_shutdown enter");
    let n = 2;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2)]) = args.get(0..n) {
        let fd = *p1;
        let how = *p2 as u8;
        Ok(to_wasm_return(p::async_socket::sock_shutdown(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            how,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_getpeeraddr<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_getpeeraddr enter");
    let n = 4;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4)]) =
        args.get(0..n)
    {
        let fd = *p1;
        let wasi_addr_ptr = *p2 as usize;
        let addr_type = *p3 as usize;
        let port_ptr = *p4 as usize;
        Ok(to_wasm_return(p::async_socket::sock_getpeeraddr(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(wasi_addr_ptr),
            WasmPtr::from(addr_type),
            WasmPtr::from(port_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_getlocaladdr<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_getlocaladdr enter");
    let n = 4;
    if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4)]) =
        args.get(0..n)
    {
        let fd = *p1;
        let wasi_addr_ptr = *p2 as usize;
        let addr_type = *p3 as usize;
        let port_ptr = *p4 as usize;
        Ok(to_wasm_return(p::async_socket::sock_getlocaladdr(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            WasmPtr::from(wasi_addr_ptr),
            WasmPtr::from(addr_type),
            WasmPtr::from(port_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_getsockopt<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_getsockopt enter");
    let n = 5;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5)],
    ) = args.get(0..n)
    {
        let fd = *p1;
        let level = *p2 as u32;
        let name = *p3 as u32;
        let flag = *p4 as usize;
        let flag_size_ptr = *p5 as usize;
        Ok(to_wasm_return(p::async_socket::sock_getsockopt(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            level,
            name,
            WasmPtr::from(flag),
            WasmPtr::from(flag_size_ptr),
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn sock_setsockopt<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> Result<Vec<WasmVal>, CoreError> {
    log::trace!("sock_setsockopt enter");
    let n = 5;
    if let Some(
        [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5)],
    ) = args.get(0..n)
    {
        let fd = *p1;
        let level = *p2 as u32;
        let name = *p3 as u32;
        let flag = *p4 as usize;
        let flag_size = *p5 as u32;
        Ok(to_wasm_return(p::async_socket::sock_setsockopt(
            &mut ctx.wasi_ctx,
            mem,
            fd,
            level,
            name,
            WasmPtr::from(flag),
            flag_size,
        )))
    } else {
        Err(func_type_miss_match_error())
    }
}

pub fn poll_oneoff<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("poll_oneoff enter");
    Box::new(async move {
        let n = 4;
        if let Some([WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4)]) =
            args.get(0..n)
        {
            let in_ptr = *p1 as usize;
            let out_ptr = *p2 as usize;
            let nsubscriptions = *p3 as u32;
            let revents_num_ptr = *p4 as usize;

            #[cfg(feature = "wasi_serialize")]
            if let Some((dur, hook)) = ctx.yield_hook {
                'a: loop {
                    let poll_oneoff_fut = p::async_poll::poll_oneoff(
                        &mut ctx.wasi_ctx,
                        mem,
                        WasmPtr::from(in_ptr),
                        WasmPtr::from(out_ptr),
                        nsubscriptions,
                        WasmPtr::from(revents_num_ptr),
                    );
                    if let Ok(r) = tokio::time::timeout(dur, poll_oneoff_fut).await {
                        return Ok(to_wasm_return(r));
                    } else {
                        if hook(&ctx.wasi_ctx.io_state) {
                            return Err(CoreError::Yield);
                        } else {
                            continue 'a;
                        }
                    }
                }
            }

            Ok(to_wasm_return(
                p::async_poll::poll_oneoff(
                    &mut ctx.wasi_ctx,
                    mem,
                    WasmPtr::from(in_ptr),
                    WasmPtr::from(out_ptr),
                    nsubscriptions,
                    WasmPtr::from(revents_num_ptr),
                )
                .await,
            ))
        } else {
            Err(func_type_miss_match_error())
        }
    })
}

pub fn sock_lookup_ip<'a, T>(
    _: &'a mut T,
    mem: &'a mut Memory,
    ctx: &'a mut AsyncWasiCtx,
    args: Vec<types::WasmVal>,
) -> ResultFuture<'a> {
    log::trace!("sock_lookup_ip enter");
    Box::new(async move {
        let n = 6;
        if let Some(
            [WasmVal::I32(p1), WasmVal::I32(p2), WasmVal::I32(p3), WasmVal::I32(p4), WasmVal::I32(p5), WasmVal::I32(p6)],
        ) = args.get(0..n)
        {
            let host_name_ptr = *p1 as usize;
            let host_name_len = *p2 as u32;
            let lookup_type = *p3 as u8;
            let addr_buf = *p4 as usize;
            let addr_buf_max_len = *p5 as u32;
            let raddr_num_ptr = *p6 as usize;
            Ok(to_wasm_return(
                p::async_socket::sock_lookup_ip(
                    &mut ctx.wasi_ctx,
                    mem,
                    WasmPtr::from(host_name_ptr),
                    host_name_len,
                    lookup_type,
                    WasmPtr::from(addr_buf),
                    addr_buf_max_len,
                    WasmPtr::from(raddr_num_ptr),
                )
                .await,
            ))
        } else {
            println!("sock_lookup_ip type_miss");
            Err(func_type_miss_match_error())
        }
    })
}
