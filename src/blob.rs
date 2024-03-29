//! Blob Storage System

use crate::{blob_bdev::BlobStoreBDev, complete::LocalComplete, error::*};
use log::*;
use serde::{Deserialize, Serialize};
use spdk_sys::*;
use std::ffi::c_void;
use std::fmt;
use std::os::raw::c_int;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

#[derive(Debug)]
pub struct Blobstore {
    pub ptr: *mut spdk_blob_store,
}

impl Default for Blobstore {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }
}

unsafe impl Send for Blobstore {}
unsafe impl Sync for Blobstore {}

impl Blobstore {
    /// Get the cluster size in bytes.
    pub fn cluster_size(&self) -> u64 {
        unsafe { spdk_bs_get_cluster_size(self.ptr) }
    }

    /// Get the page size in bytes.
    pub fn page_size(&self) -> u64 {
        unsafe { spdk_bs_get_page_size(self.ptr) }
    }

    /// Get the io unit size in bytes.
    pub fn io_unit_size(&self) -> u64 {
        unsafe { spdk_bs_get_io_unit_size(self.ptr) }
    }

    /// Get the number of free clusters.
    pub fn free_cluster_count(&self) -> u64 {
        unsafe { spdk_bs_free_cluster_count(self.ptr) }
    }

    /// Get the total number of clusters accessible by user.
    pub fn total_data_cluster_count(&self) -> u64 {
        unsafe { spdk_bs_total_data_cluster_count(self.ptr) }
    }

    /// Allocate an I/O channel for the given blobstore.
    pub fn alloc_io_channel(&self) -> Result<IoChannel> {
        let ptr = unsafe { spdk_bs_alloc_io_channel(self.ptr) };
        if ptr.is_null() {
            // FIXME: proper error
            return Err(SpdkError::from(-1));
        }
        Ok(IoChannel { ptr })
    }

    /// Initialize a blobstore on the given device.
    pub async fn init(bs_dev: &mut BlobStoreBDev) -> Result<Blobstore> {
        let ptr = do_async(|arg| unsafe {
            spdk_bs_init(bs_dev.ptr, std::ptr::null_mut(), Some(callback_with), arg);
        })
        .await?;
        Ok(Blobstore { ptr })
    }

    pub fn init_sync(bs_dev: &mut BlobStoreBDev, cb_arg: *mut c_void) -> Result<()> {
        unsafe {
            spdk_bs_init(
                bs_dev.ptr,
                std::ptr::null_mut(),
                Some(init_callback),
                cb_arg,
            );
        };
        Ok(())
    }

    /// Load a blobstore on the given device
    pub async fn load(bs_dev: &mut BlobStoreBDev) -> Result<Blobstore> {
        let ptr = do_async(|arg| unsafe {
            spdk_bs_load(bs_dev.ptr, std::ptr::null_mut(), Some(callback_with), arg);
        })
        .await?;
        Ok(Blobstore { ptr })
    }

    pub fn load_sync(bs_dev: &mut BlobStoreBDev, cb_arg: *mut c_void) -> Result<()> {
        unsafe {
            spdk_bs_load(
                bs_dev.ptr,
                std::ptr::null_mut(),
                Some(init_callback),
                cb_arg,
            );
        };
        Ok(())
    }

    /// Unload the blobstore.
    ///
    /// It will flush all volatile data to disk.
    /// WARN: all io_channels must be dropped before unload!
    pub async fn unload(&self) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_bs_unload(self.ptr, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    pub fn unload_sync(&self, cb_arg: *mut c_void) -> Result<()> {
        if self.ptr.is_null() {
            error!("blobstore ptr is null");
        }
        unsafe {
            spdk_bs_unload(self.ptr, Some(unload_callback), cb_arg);
        }
        Ok(())
    }

    /// Create a new blob with default option values on the given blobstore.
    pub async fn create_blob(&self) -> Result<BlobId> {
        let id = do_async(|arg| unsafe {
            spdk_bs_create_blob(self.ptr, Some(callback_with), arg);
        })
        .await?;
        Ok(BlobId { id })
    }

    /// Create blob, sync API
    ///
    /// cb_arg: Arc<Mutex< BlobId >>
    pub fn create_blob_sync(
        &self,
        // cb_fn: extern "C" fn(*mut c_void, spdk_blob_id, i32),
        cb_arg: *mut c_void,
    ) -> Result<()> {
        unsafe {
            spdk_bs_create_blob(self.ptr, Some(create_callback), cb_arg);
        }
        Ok(())
    }

    /// Open a blob from the given blobstore.
    pub async fn open_blob(&self, blob_id: BlobId) -> Result<Blob> {
        let ptr = do_async(|arg| unsafe {
            spdk_bs_open_blob(self.ptr, blob_id.id, Some(callback_with), arg);
        })
        .await?;
        let io_unit_size = self.io_unit_size();
        Ok(Blob { ptr, io_unit_size })
    }

    /// Open blob, sync API
    ///
    /// cb_arg: Arc<Mutex<Blob>>
    pub fn open_blob_sync(
        &self,
        blob_id: &BlobId,
        // cb_fn: extern "C" fn(*mut c_void, *mut spdk_blob, c_int),
        cb_arg: *mut c_void,
    ) -> Result<()> {
        unsafe {
            spdk_bs_open_blob(self.ptr, blob_id.id, Some(open_callback), cb_arg);
        }
        Ok(())
    }

    /// Delete an existing blob from the given blobstore.
    pub async fn delete_blob(&self, blob_id: BlobId) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_bs_delete_blob(self.ptr, blob_id.id, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    /// Delete blob, sync API
    pub fn delete_blob_sync(&self, blob_id: &BlobId, cb_arg: *mut c_void) -> Result<()> {
        unsafe {
            spdk_bs_delete_blob(self.ptr, blob_id.id, Some(delete_callback), cb_arg);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlobId {
    id: spdk_blob_id,
}

impl Default for BlobId {
    fn default() -> Self {
        Self { id: 0 }
    }
}

impl fmt::Display for BlobId {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> fmt::Result {
        write!(f, "BlobId({:?})", self.id)
    }
}

#[derive(Debug)]
pub struct IoChannel {
    pub ptr: *mut spdk_io_channel,
}

unsafe impl Send for IoChannel {}
unsafe impl Sync for IoChannel {}

impl Drop for IoChannel {
    fn drop(&mut self) {
        unsafe { spdk_bs_free_io_channel(self.ptr) };
    }
}

#[derive(Debug)]
pub struct Blob {
    pub ptr: *mut spdk_blob,
    io_unit_size: u64,
}

impl Default for Blob {
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            io_unit_size: 512,
        }
    }
}

unsafe impl Send for Blob {}
unsafe impl Sync for Blob {}

impl Clone for Blob {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
            io_unit_size: self.io_unit_size,
        }
    }
}

impl Copy for Blob {}

impl Blob {
    /// Get the number of clusters allocated to the blob.
    pub fn num_clusters(&self) -> u64 {
        unsafe { spdk_blob_get_num_clusters(self.ptr) }
    }

    /// Get the blob id.
    pub fn blob_id(&self) -> BlobId {
        let id = unsafe { spdk_blob_get_id(self.ptr) };
        BlobId { id }
    }

    /// Read data from a blob.
    pub async fn read(&self, io_channel: &IoChannel, offset: u64, buf: &mut [u8]) -> Result<()> {
        assert_eq!(buf.len() as u64 % self.io_unit_size, 0);
        let units = buf.len() as u64 / self.io_unit_size;
        do_async(|arg| unsafe {
            spdk_blob_io_read(
                self.ptr,
                io_channel.ptr,
                buf.as_mut_ptr() as _,
                offset,
                units,
                Some(callback),
                arg,
            );
        })
        .await
    }

    /// Read data from a blob, sync API
    pub fn read_sync(
        &self,
        io_channel: &IoChannel,
        offset: u64,
        buf: &mut [u8],
        cb_arg: *mut c_void,
    ) -> Result<()> {
        assert_eq!(buf.len() as u64 % self.io_unit_size, 0);
        let units = buf.len() as u64 / self.io_unit_size;
        unsafe {
            spdk_blob_io_read(
                self.ptr,
                io_channel.ptr,
                buf.as_mut_ptr() as _,
                offset,
                units,
                Some(rw_callback),
                cb_arg,
            );
        }
        Ok(())
    }

    /// Write data to a blob.
    pub async fn write(&self, io_channel: &IoChannel, offset: u64, buf: &[u8]) -> Result<()> {
        assert_eq!(buf.len() as u64 % self.io_unit_size, 0);
        let units = buf.len() as u64 / self.io_unit_size;
        do_async(|arg| unsafe {
            spdk_blob_io_write(
                self.ptr,
                io_channel.ptr,
                buf.as_ptr() as _,
                offset,
                units,
                Some(callback),
                arg,
            );
        })
        .await
    }

    /// Write data to a blob, sync API
    pub fn write_sync(
        &self,
        io_channel: &IoChannel,
        offset: u64,
        buf: &[u8],
        cb_arg: *mut c_void,
    ) -> Result<()> {
        assert_eq!(buf.len() as u64 % self.io_unit_size, 0);
        let units = buf.len() as u64 / self.io_unit_size;
        unsafe {
            spdk_blob_io_write(
                self.ptr,
                io_channel.ptr,
                buf.as_ptr() as _,
                offset,
                units,
                Some(rw_callback),
                cb_arg,
            );
        }
        Ok(())
    }

    /// Write zeros into area of a blob.
    pub async fn write_zero(&self, io_channel: &IoChannel, offset: u64, len: u64) -> Result<()> {
        assert_eq!(len % self.io_unit_size, 0);
        let units = len / self.io_unit_size;
        do_async(|arg| unsafe {
            spdk_blob_io_write_zeroes(self.ptr, io_channel.ptr, offset, units, Some(callback), arg);
        })
        .await
    }

    /// Write zeros to a blob, sync API
    pub fn write_zero_sync(
        &self,
        io_channel: &IoChannel,
        offset: u64,
        len: u64,
        cb_arg: *mut c_void,
    ) -> Result<()> {
        assert_eq!(len % self.io_unit_size, 0);
        let units = len / self.io_unit_size;
        unsafe {
            spdk_blob_io_write_zeroes(
                self.ptr,
                io_channel.ptr,
                offset,
                units,
                Some(rw_callback),
                cb_arg,
            );
        }
        Ok(())
    }

    /// Resize a blob to `size` clusters.
    ///
    /// These changes are not persisted to disk until spdk_bs_md_sync_blob() is called.
    /// If called before previous resize finish, it will fail with errno -EBUSY.
    pub async fn resize(&self, size: u64) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_blob_resize(self.ptr, size, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    /// Resize a blob, sync API
    pub fn resize_sync(
        &self,
        size: u64,
        // cb_fn: unsafe extern "C" fn(*mut c_void, c_int),
        cb_arg: *mut c_void,
    ) -> Result<()> {
        unsafe {
            spdk_blob_resize(self.ptr, size, Some(resize_callback), cb_arg);
        }
        Ok(())
    }

    /// Sync a blob.
    ///
    /// Make a blob persistent. This applies to open, resize, set xattr, and remove xattr.
    /// These operations will not be persistent until the blob has been synced.
    pub async fn sync_metadata(&self) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_blob_sync_md(self.ptr, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    /// Sync blob's metadata, sync API
    pub fn sync_metadata_sync(
        &self,
        // cb_fn: unsafe extern "C" fn(*mut c_void, c_int),
        cb_arg: *mut c_void,
    ) -> Result<()> {
        unsafe {
            spdk_blob_sync_md(self.ptr, Some(sync_md_callback), cb_arg);
        }
        Ok(())
    }

    /// Close a blob.
    ///
    /// This will automatically sync.
    pub async fn close(self) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_blob_close(self.ptr, Some(callback), arg);
        })
        .await?;
        Ok(())
    }

    /// Close a blob, sync API
    pub fn close_sync(self, cb_arg: *mut c_void) -> Result<()> {
        unsafe {
            spdk_blob_close(self.ptr, Some(close_blob_callback), cb_arg);
        }
        Ok(())
    }
}

extern "C" fn callback(arg: *mut c_void, bserrno: c_int) {
    callback_with(arg, (), bserrno);
}

extern "C" fn callback_with<T>(arg: *mut c_void, bs: T, bserrno: c_int) {
    let complete = unsafe { &mut *(arg as *mut LocalComplete<Result<T>>) };
    let result = if bserrno != 0 {
        Err(SpdkError::from(bserrno))
    } else {
        Ok(bs)
    };
    complete.complete(result);
}

extern "C" fn init_callback(mut arg: *mut c_void, bs: *mut spdk_blob_store, bserrno: c_int) {
    if bserrno != 0 {
        error!("bs error");
    }
    if bs.is_null() {
        error!("bs pointer is null");
    }
    let (bs_, n) = unsafe { *Box::from_raw(arg as *mut (Arc<Mutex<Blobstore>>, Arc<Notify>)) };
    unsafe {
        bs_.lock().unwrap().ptr = bs;
        n.notify_one();
    }
}

extern "C" fn open_callback(mut arg: *mut c_void, blob: *mut spdk_blob, bserrno: c_int) {
    if bserrno != 0 {
        error!("open error");
    }
    if blob.is_null() {
        error!("open blob pointer null");
    }
    let (blob_, n) = unsafe { *Box::from_raw(arg as *mut (Arc<Mutex<Blob>>, Arc<Notify>)) };
    unsafe {
        blob_.lock().unwrap().ptr = blob;
        n.notify_one();
    }
}

extern "C" fn create_callback(mut arg: *mut c_void, blob_id: spdk_blob_id, bserrno: c_int) {
    if bserrno != 0 {
        error!("create error");
    }
    let (blob_id_, n) = unsafe { *Box::from_raw(arg as *mut (Arc<Mutex<BlobId>>, Arc<Notify>)) };
    unsafe {
        blob_id_.lock().unwrap().id = blob_id;
        n.notify_one();
    }
}

extern "C" fn resize_callback(arg: *mut c_void, bserrno: c_int) {
    if bserrno != 0 {
        error!("resize error");
    }
    let n = unsafe { *Box::from_raw(arg as *mut Arc<Notify>) };
    n.notify_one();
}

extern "C" fn sync_md_callback(arg: *mut c_void, bserrno: c_int) {
    if bserrno != 0 {
        error!("sync metadata error");
    }
    let n = unsafe { *Box::from_raw(arg as *mut Arc<Notify>) };
    n.notify_one();
}

extern "C" fn unload_callback(arg: *mut c_void, bserrno: c_int) {
    if bserrno != 0 {
        error!("bs unload error");
    }
    let n = unsafe { *Box::from_raw(arg as *mut Arc<Notify>) };
    n.notify_one();
}

extern "C" fn rw_callback(arg: *mut c_void, bserrno: c_int) {
    if bserrno != 0 {
        error!("read/write error: {}", bserrno);
    }
    let n = unsafe { *Box::from_raw(arg as *mut Arc<Notify>) };
    n.notify_one();
}

extern "C" fn delete_callback(arg: *mut c_void, bserrno: c_int) {
    if bserrno != 0 {
        error!("delete blob error");
    }
    let n = unsafe { *Box::from_raw(arg as *mut Arc<Notify>) };
    n.notify_one();
}

extern "C" fn close_blob_callback(arg: *mut c_void, bserrno: c_int) {
    if bserrno != 0 {
        error!("close blob error");
    }
    let n = unsafe { *Box::from_raw(arg as *mut Arc<Notify>) };
    n.notify_one();
}

async fn do_async<T: Unpin>(f: impl FnOnce(*mut c_void)) -> Result<T> {
    let complete = LocalComplete::<Result<T>>::new();
    futures_lite::pin!(complete);
    f(complete.as_arg());
    complete.await
}
