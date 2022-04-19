use async_spdk::*;
use std::sync::Arc;
use async_spdk::blob::*;
use crate::common::EngineBlob;
use event::AppOpts;
use log::*;

#[derive(Debug)]
pub struct DeviceEngine{
    pub bs: Arc<Blobstore>,
    pub name: String,
    pub io_size: u64,
}

impl DeviceEngine{
    pub fn get_name(&self) ->Result<String>{
        Ok(self.name.clone())
    }

    pub fn get_io_size(&self) -> Result<u64>{
        Ok(self.io_size)
    }

}

impl DeviceEngine{
    /// here name refers to the device name specified by json
    pub async fn new(name: &str) -> Result<Self>{
        let bs = DeviceEngine::open_bs(name).await?;
        let size = bs.io_unit_size();
        let ret = DeviceEngine{
            bs: Arc::new(bs),
            name: String::from(name),
            io_size: size,
        };
        Ok(ret)
    }

    /// helper for DeviceEngine::new function
    async fn open_bs(name: &str) -> Result<Blobstore>{
        let mut bs_dev = blob_bdev::BlobStoreBDev::create(name)?;
        let bs = blob::Blobstore::init(&mut bs_dev).await?;
        Ok(bs)
    }

    pub async fn write(&self, offset: u64, blob_id: BlobId, buf: &[u8])->Result<()>{
        let blob = self.bs.open_blob(blob_id).await?;
        let channel = self.bs.alloc_io_channel()?;
        blob.write(&channel, offset, buf).await?;
        blob.close().await?;
        drop(channel);
        Ok(())
    }

    pub async fn read(&self, offset: u64, blob_id: BlobId, buf: &mut [u8]) -> Result<()>{
        let blob = self.bs.open_blob(blob_id).await?;
        let channel = self.bs.alloc_io_channel()?;
        blob.read(&channel, offset, buf).await?;
        blob.close().await?;
        drop(channel);
        Ok(())
    }

    /// create a blob with #size clusters 
    /// note: size is number of clusters, usually 1MB per cluster
    pub async fn create_blob(&self, size: u64) -> Result<EngineBlob>{
        let blob_id = self.bs.create_blob().await?;
        let blob = self.bs.open_blob(blob_id).await?;
        blob.resize(size).await?;
        blob.sync_metadata().await?;
        blob.close().await?;
        Ok(EngineBlob{
            bl: blob_id,
        })
    }

    pub async fn delete_blob(&self, bid: BlobId) -> Result<()>{
        self.bs.delete_blob(bid).await?;
        Ok(())
    }

    /// close blobstore handle
    /// todo: check io_channel closed or not
    pub async fn close_bs(&self)->Result<()>{
        self.bs.unload().await?;
        Ok(())
    }
}
