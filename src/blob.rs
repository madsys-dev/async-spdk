//! Blob Storage System

use spdk_sys::*;
use std::fmt;

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

    // pub fn alloc_io_channel(&mut self) -> Result<IoChannel, BlobstoreError> {
    //     let io_channel = unsafe { spdk_bs_alloc_io_channel(self.ptr) };
    //     if io_channel.is_null() {
    //         return Err(BlobstoreError::IoChannelAllocateError);
    //     }
    //     Ok(IoChannel { io_channel })
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct BlobId {
    id: spdk_blob_id,
}

impl fmt::Display for BlobId {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> fmt::Result {
        write!(f, "BlobId({:?})", self.id)
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
}
