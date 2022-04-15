use async_spdk::blob::{BlobId};
use async_trait::async_trait;
use async_spdk::*;
use crate::common::*;

#[async_trait]
pub trait DevEngine: Send + Sync{
    /// write data with IO_UNIT size in a blob
    /// todo: implement specific error type
    async fn write(&self, offset: u64, blob_id: BlobId, buf: &[u8])->Result<()>;

    /// read data with IO_UNIT size in a blob
    async fn read(&self, offset: u64, blob_id: BlobId, buf: &mut [u8]) -> Result<()>;

    /// create a blob, return blobID 
    async fn create_blob(&self, size: u64) -> Result<EngineBlob>;
}



