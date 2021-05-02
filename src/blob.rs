//! Blob Storage System

use crate::{blob_bdev::BlobStoreBDev, complete::LocalComplete, error::*};
use spdk_sys::*;
use std::ffi::c_void;
use std::fmt;
use std::os::raw::c_int;

#[derive(Debug)]
pub struct Blobstore {
    ptr: *mut spdk_blob_store,
}

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

    /// Unload the blobstore.
    ///
    /// It will flush all volatile data to disk.
    pub async fn unload(self) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_bs_unload(self.ptr, Some(callback), arg);
        })
        .await?;
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

    /// Open a blob from the given blobstore.
    pub async fn open_blob(&self, blob_id: BlobId) -> Result<Blob> {
        let ptr = do_async(|arg| unsafe {
            spdk_bs_open_blob(self.ptr, blob_id.id, Some(callback_with), arg);
        })
        .await?;
        Ok(Blob { ptr })
    }

    /// Delete an existing blob from the given blobstore.
    pub async fn delete_blob(&self, blob_id: BlobId) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_bs_delete_blob(self.ptr, blob_id.id, Some(callback), arg);
        })
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobId {
    id: spdk_blob_id,
}

impl fmt::Display for BlobId {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> fmt::Result {
        write!(f, "BlobId({:?})", self.id)
    }
}

#[derive(Debug)]
pub struct IoChannel {
    pub(crate) ptr: *mut spdk_io_channel,
}

impl Drop for IoChannel {
    fn drop(&mut self) {
        unsafe { spdk_bs_free_io_channel(self.ptr) };
    }
}

#[derive(Debug)]
pub struct Blob {
    ptr: *mut spdk_blob,
}

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
        do_async(|arg| unsafe {
            spdk_blob_io_read(
                self.ptr,
                io_channel.ptr,
                buf.as_mut_ptr() as _,
                offset,
                buf.len() as _,
                Some(callback),
                arg,
            );
        })
        .await
    }

    /// Write data to a blob.
    pub async fn write(&self, io_channel: &IoChannel, offset: u64, buf: &[u8]) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_blob_io_write(
                self.ptr,
                io_channel.ptr,
                buf.as_ptr() as _,
                offset,
                buf.len() as _,
                Some(callback),
                arg,
            );
        })
        .await
    }

    /// Write zeros into area of a blob.
    pub async fn write_zero(&self, io_channel: &IoChannel, offset: u64, len: u64) -> Result<()> {
        do_async(|arg| unsafe {
            spdk_blob_io_write_zeroes(self.ptr, io_channel.ptr, offset, len, Some(callback), arg);
        })
        .await
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

async fn do_async<T: Unpin>(f: impl FnOnce(*mut c_void)) -> Result<T> {
    let complete = LocalComplete::<Result<T>>::new();
    futures_lite::pin!(complete);
    f(complete.as_arg());
    complete.await
}
