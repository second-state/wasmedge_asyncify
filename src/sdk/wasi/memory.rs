use crate::Memory;
use wasmedge_async_wasi::snapshots::common::memory::{Memory as WasiMem, WasmPtr};
use wasmedge_async_wasi::snapshots::env::wasi_types::__wasi_ciovec_t;
use wasmedge_async_wasi::snapshots::env::wasi_types::__wasi_size_t;
use wasmedge_async_wasi::snapshots::env::Errno;

impl WasiMem for Memory {
    fn get_data<'a, T: Sized>(&'a self, offset: WasmPtr<T>) -> Result<&'a T, Errno> {
        unsafe {
            let r = std::mem::size_of::<T>();
            let ptr = self
                .data_pointer_raw(offset.0, r)
                .ok_or(Errno::__WASI_ERRNO_FAULT)?;
            Ok(ptr.cast::<T>().as_ref().unwrap())
        }
    }

    fn get_slice<'a, T: Sized>(&'a self, offset: WasmPtr<T>, len: usize) -> Result<&'a [T], Errno> {
        unsafe {
            let r = std::mem::size_of::<T>() * len;
            let ptr = self
                .data_pointer_raw(offset.0, r)
                .ok_or(Errno::__WASI_ERRNO_FAULT)? as *const T;
            Ok(std::slice::from_raw_parts(ptr, len))
        }
    }

    fn get_iovec<'a>(
        &'a self,
        iovec_ptr: WasmPtr<__wasi_ciovec_t>,
        iovec_len: __wasi_size_t,
    ) -> Result<Vec<std::io::IoSlice<'a>>, Errno> {
        unsafe {
            let iovec = self.get_slice(iovec_ptr, iovec_len as usize)?.to_vec();
            let mut result = Vec::with_capacity(iovec.len());
            for i in iovec {
                let len = i.buf_len as usize;
                let ptr = self
                    .data_pointer_raw(i.buf as usize, len)
                    .ok_or(Errno::__WASI_ERRNO_FAULT)?;
                let s = std::io::IoSlice::new(std::slice::from_raw_parts(ptr, len));
                result.push(s);
            }
            Ok(result)
        }
    }

    fn mut_data<'a, T: Sized>(&'a mut self, offset: WasmPtr<T>) -> Result<&'a mut T, Errno> {
        unsafe {
            let r = std::mem::size_of::<T>();
            let ptr = self
                .data_pointer_mut_raw(offset.0, r)
                .ok_or(Errno::__WASI_ERRNO_FAULT)?;
            Ok(ptr.cast::<T>().as_mut().unwrap())
        }
    }

    fn mut_slice<'a, T: Sized>(
        &'a mut self,
        offset: WasmPtr<T>,
        len: usize,
    ) -> Result<&'a mut [T], Errno> {
        unsafe {
            let r = std::mem::size_of::<T>() * len;
            let ptr = self
                .data_pointer_raw(offset.0, r)
                .ok_or(Errno::__WASI_ERRNO_FAULT)? as *mut T;
            Ok(std::slice::from_raw_parts_mut(ptr, len))
        }
    }

    fn mut_iovec<'a>(
        &'a mut self,
        iovec_ptr: WasmPtr<wasmedge_async_wasi::snapshots::env::wasi_types::__wasi_iovec_t>,
        iovec_len: wasmedge_async_wasi::snapshots::env::wasi_types::__wasi_size_t,
    ) -> Result<Vec<std::io::IoSliceMut<'a>>, Errno> {
        unsafe {
            let iovec = self.get_slice(iovec_ptr, iovec_len as usize)?.to_vec();
            let mut result = Vec::with_capacity(iovec.len());
            for i in iovec {
                let len = i.buf_len as usize;
                let ptr = self
                    .data_pointer_mut_raw(i.buf as usize, len)
                    .ok_or(Errno::__WASI_ERRNO_FAULT)?;
                let s = std::io::IoSliceMut::new(std::slice::from_raw_parts_mut(ptr, len));
                result.push(s);
            }
            Ok(result)
        }
    }

    fn write_data<'a, T: Sized>(&'a mut self, offset: WasmPtr<T>, data: T) -> Result<(), Errno> {
        let p = self.mut_data(offset)?;
        *p = data;
        Ok(())
    }
}
