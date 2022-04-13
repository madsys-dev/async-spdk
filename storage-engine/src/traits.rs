use async_spdk::blob::{BlobId, Blob};
use async_trait::async_trait;
use async_spdk::*;

#[async_trait]
pub trait DevEngine: Send + Sync{
    /// write data with IO_UNIT size in a blob
    /// todo: implement specific error type
    async fn write(&self, offset: u64, blobId: BlobId, buf: &[u8])->Result<()>;

    /// read data with IO_UNIT size in a blob
    async fn read(&self, offset: u64, blobId: BlobId, buf: &mut [u8]) -> Result<()>;

    /// create a blob, return blobID 
    async fn create_blob(&self, size: u64) -> Result<Blob>;
}



