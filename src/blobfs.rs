//! BlobStore FileSystem

use std::mem::MaybeUninit;

use log::*;
use crate::blob::IoChannel;
use crate::event::SpdkEvent;
use crate::{blob_bdev::BlobStoreBDev, complete::LocalComplete, error::*};
use spdk_sys::*;
use std::ffi::{c_void, CString};
use std::os::raw::c_int;

#[derive(Debug, Clone)]
pub struct SpdkFileStat {
    ptr: *mut spdk_file_stat,
}

impl Default for SpdkFileStat {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpdkFilesystem {
    ptr: *mut spdk_filesystem,
}

#[derive(Debug)]
pub struct SpdkFile {
    ptr: *mut spdk_file,
}

impl Default for SpdkFile {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }
}

/// Sync API
impl SpdkFile {
    pub fn close(&self, ctx: &SpdkFsThreadCtx) -> Result<()> {
        let ret = unsafe { spdk_file_close(self.ptr, ctx.ptr) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    pub fn truncate(&self, ctx: &SpdkFsThreadCtx, length: u64) -> Result<()> {
        let ret = unsafe { spdk_file_truncate(self.ptr, ctx.ptr, length) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    pub fn name(&self) -> Result<String> {
        let name = unsafe { spdk_file_get_name(self.ptr).as_ref().unwrap() };
        Ok(name.to_string())
    }

    pub fn get_len(&self) -> Result<u64> {
        let ret = unsafe { spdk_file_get_length(self.ptr) };
        Ok(ret)
    }

    pub fn write(
        &mut self,
        ctx: &SpdkFsThreadCtx,
        data: &[u8],
        offset: u64,
        len: u64,
    ) -> Result<()> {
        let ret = unsafe { spdk_file_write(self.ptr, ctx.ptr, data.as_ptr() as _, offset, len) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    /// Read data to user buffer from given file
    ///
    /// Return positive number for the end position of this read opration
    ///
    /// negative number if fail
    pub fn read(
        &self,
        ctx: &SpdkFsThreadCtx,
        data: &mut [u8],
        offset: u64,
        len: u64,
    ) -> Result<i64> {
        let ret = unsafe { spdk_file_read(self.ptr, ctx.ptr, data.as_mut_ptr() as _, offset, len) };
        Ok(ret)
    }

    pub fn set_priority(&mut self, pri: u32) {
        unsafe {
            spdk_file_set_priority(self.ptr, pri);
        }
    }

    pub fn sync(&mut self, ctx: &SpdkFsThreadCtx) -> Result<()> {
        let ret = unsafe { spdk_file_sync(self.ptr, ctx.ptr) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    /// Get unique id of given file, that is BlobId
    ///
    /// return id size on success
    pub fn get_id(&self, id: &mut [u8], size: u64) -> Result<i32> {
        Ok(unsafe { spdk_file_get_id(self.ptr, id.as_mut_ptr() as *mut c_void, size) })
    }
}

/// Async API
impl SpdkFile {
    pub async fn aclose(&self) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_file_close_async(self.ptr, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    pub async fn atruncate(&self, len: u64) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_file_truncate_async(self.ptr, len, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    pub async fn awrite(
        &self,
        channel: &IoChannel,
        data: &[u8],
        offset: u64,
        len: u64,
    ) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_file_write_async(
                self.ptr,
                channel.ptr,
                data.as_ptr() as _,
                offset,
                len,
                Some(callback),
                arg,
            );
        })
        .await?;
        Ok(())
    }

    pub async fn aread(
        &self,
        channel: &IoChannel,
        data: &mut [u8],
        offset: u64,
        len: u64,
    ) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_file_read_async(
                self.ptr,
                channel.ptr,
                data.as_mut_ptr() as _,
                offset,
                len,
                Some(callback),
                arg,
            );
        })
        .await?;
        Ok(())
    }

    pub async fn async_sync(&self, channel: &IoChannel) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_file_sync_async(self.ptr, channel.ptr, Some(callback), arg);
        })
        .await?;
        Ok(())
    }
}

/// Async API
impl SpdkFilesystem {
    /// init blobfs from bs_dev
    pub async fn init(bs_dev: &mut BlobStoreBDev, opts: &mut SpdkBlobfsOpts) -> Result<Self> {
        let ptr = do_async(|arg| unsafe {
            spdk_fs_init(bs_dev.ptr, &mut opts.0, Some(send_request_fn), Some(callback_with), arg);
        })
        .await?;
        Ok(SpdkFilesystem { ptr })
    }

    /// load blobfs from bs_dev
    pub async fn load(bs_dev: &mut BlobStoreBDev) -> Result<Self> {
        let ptr = do_async(|arg| unsafe {
            spdk_fs_load(bs_dev.ptr, Some(send_request_fn), Some(callback_with), arg);
        })
        .await?;
        Ok(SpdkFilesystem { ptr })
    }

    /// unload blobfs
    pub async fn unload(&self) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_fs_unload(self.ptr, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    pub async fn astat(&self, name: &str) -> Result<SpdkFileStat> {
        let cname = CString::new(name).expect("Fail to parse name");
        let ptr = do_async(|arg| unsafe {
            spdk_fs_file_stat_async(self.ptr, cname.as_ptr(), Some(callback_with), arg);
        })
        .await?;
        Ok(SpdkFileStat { ptr })
    }

    pub async fn acreate(&self, name: &str) -> Result<()> {
        let cname = CString::new(name).expect("Fail to parse name");
        do_async(|arg| unsafe {
            spdk_fs_create_file_async(self.ptr, cname.as_ptr(), Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    pub async fn aopen(&self, name: &str, flag: u32) -> Result<SpdkFile> {
        let cname = CString::new(name).expect("Fail to parse name");
        let ptr = do_async(|arg| unsafe {
            spdk_fs_open_file_async(self.ptr, cname.as_ptr(), flag, Some(callback_with), arg);
        })
        .await?;
        Ok(SpdkFile { ptr })
    }

    pub async fn arename(&self, from: &str, to: &str) -> Result<()> {
        let from = CString::new(from).expect("Fail to parse old name");
        let to = CString::new(to).expect("Fail to parse new name");
        do_async(|arg| unsafe {
            spdk_fs_rename_file_async(self.ptr, from.as_ptr(), to.as_ptr(), Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    pub async fn adelete(&self, name: &str) -> Result<()> {
        let cname = CString::new(name).expect("Fail to parse name");
        do_async(|arg| unsafe {
            spdk_fs_delete_file_async(self.ptr, cname.as_ptr(), Some(callback), arg);
        })
        .await?;
        Ok(())
    }
}

/// Sync API
impl SpdkFilesystem {
    /// Allocate an I/O channel for async operations
    pub fn alloc_io_channel(&self) -> Result<IoChannel> {
        let ptr = unsafe { spdk_fs_alloc_io_channel(self.ptr) };
        if ptr.is_null() {
            return Err(SpdkError::from(-1));
        }
        Ok(IoChannel { ptr })
    }

    /// Free I/O channel from blobfs
    pub fn free_io_channel(&self, channel: IoChannel) -> Result<()> {
        unsafe {
            spdk_fs_free_io_channel(channel.ptr);
        }
        Ok(())
    }

    /// Allocate a context for synchronous operations
    ///
    /// This is a requirement for sync ops
    pub fn alloc_thread_ctx(&self) -> Result<SpdkFsThreadCtx> {
        let ptr = unsafe { spdk_fs_alloc_thread_ctx(self.ptr) };
        if ptr.is_null() {
            return Err(SpdkError::from(-1));
        }
        Ok(SpdkFsThreadCtx { ptr })
    }

    pub fn stat(&self, ctx: &SpdkFsThreadCtx, name: &str, stat: &mut SpdkFileStat) -> Result<()> {
        let cname = CString::new(name).expect("Fail to parse name");
        let ret = unsafe { spdk_fs_file_stat(self.ptr, ctx.ptr, cname.as_ptr(), stat.ptr) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    pub fn create(&self, ctx: &SpdkFsThreadCtx, name: &str) -> Result<()> {
        let cname = CString::new(name).expect("Failt to parse name");
        let fs = self.clone();
        let ret = unsafe { spdk_fs_create_file(fs.ptr, ctx.ptr, cname.as_ptr()) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    pub fn open(
        &self,
        ctx: &SpdkFsThreadCtx,
        name: &str,
        flags: u32,
        file: &mut SpdkFile,
    ) -> Result<()> {
        let cname = CString::new(name).expect("Fail to parse name");
        let ret =
            unsafe { spdk_fs_open_file(self.ptr, ctx.ptr, cname.as_ptr(), flags, &mut file.ptr) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    pub fn rename(&self, ctx: &SpdkFsThreadCtx, from: &str, to: &str) -> Result<()> {
        let from = CString::new(from).expect("Fail to parse old name");
        let to = CString::new(to).expect("Fail to parse new name");
        let ret = unsafe { spdk_fs_rename_file(self.ptr, ctx.ptr, from.as_ptr(), to.as_ptr()) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    pub fn delete(&self, ctx: &SpdkFsThreadCtx, name: &str) -> Result<()> {
        let cname = CString::new(name).expect("Fail to parse name");
        let ret = unsafe { spdk_fs_delete_file(self.ptr, ctx.ptr, cname.as_ptr()) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    /// set cache size of blobfs in MB
    pub fn set_cache_size(&self, size: u64) -> Result<()> {
        let ret = unsafe { spdk_fs_set_cache_size(size) };
        if ret != 0 {
            return Err(SpdkError::from(-1));
        }
        Ok(())
    }

    /// Obtain cache size in MB
    pub fn get_cache_size(&self) -> Result<u64> {
        Ok(unsafe { spdk_fs_get_cache_size() })
    }
}

#[derive(Debug, Clone)]
pub struct SpdkFsThreadCtx {
    ptr: *mut spdk_fs_thread_ctx,
}

impl Drop for SpdkFsThreadCtx {
    fn drop(&mut self) {
        unsafe {
            spdk_fs_free_thread_ctx(self.ptr);
        }
    }
}

#[derive(Debug)]
pub struct SpdkBlobfsOpts(spdk_blobfs_opts);

impl SpdkBlobfsOpts {
    /// init BlobfsOpts
    ///
    /// major job is set cluster_sz
    pub async fn init() -> Result<Self> {
        let mut fs_opts = MaybeUninit::uninit();
        extern "C" {
            fn spdk_fs_opts_init(fs_opts: *mut spdk_blobfs_opts);
        }
        unsafe {
            spdk_fs_opts_init(fs_opts.as_mut_ptr());
            Ok(SpdkBlobfsOpts(fs_opts.assume_init()))
        }
    }
}

unsafe extern "C" fn send_request_fn(f: Option<unsafe extern "C" fn(*mut c_void)>, arg: *mut c_void){
    let mut e = SpdkEvent::alloc(1, f.unwrap() as *mut c_void, arg).unwrap();
    info!("call send_request");
    e.call();
}

extern "C" fn callback(arg: *mut c_void, fserrno: c_int) {
    callback_with(arg, (), fserrno);
}

extern "C" fn callback_with<T>(arg: *mut c_void, fs: T, fserrno: c_int) {
    let complete = unsafe { &mut *(arg as *mut LocalComplete<Result<T>>) };
    let result = if fserrno != 0 {
        Err(SpdkError::from(fserrno))
    } else {
        Ok(fs)
    };
    complete.complete(result);
}

async fn do_async<T: Unpin>(f: impl FnOnce(*mut c_void)) -> Result<T> {
    let complete = LocalComplete::<Result<T>>::new();
    futures_lite::pin!(complete);
    f(complete.as_arg());
    complete.await
}
