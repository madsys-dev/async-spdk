use async_spdk::*;
use std::sync::Arc;
use async_spdk::blob::*;
use crate::common::{DeviceEngine, EngineBlob};

impl DeviceEngine{
    /// new a device engine by name and config_file
    pub fn new(name: &str, config_file: &str)-> Self{
        let bs = event::AppOpts::new()
                    .name("madio")
                    .config_file(config_file)
                    .block_on(DeviceEngine::open_bs(name.clone()))
                    .unwrap();
        let io_size = bs.io_unit_size();
        Self{
            bs: Arc::new(bs),
            name: String::from(name),
            io_size: io_size,
        }
    }
    
    /// helper for DeviceEngine::new function
    async fn open_bs(name: &str) -> Result<Blobstore>{
        let mut bs_dev = blob_bdev::BlobStoreBDev::create(name)?;
        let bs = blob::Blobstore::init(&mut bs_dev).await?;
        Ok(bs)
    }

    /// close blobstore handle
    /// todo: check io_channel closed or not
    pub async fn close(&mut self)->Result<()>{
        self.bs.unload().await?;
        Ok(())
    }

}

impl DeviceEngine{
    async fn write(&self, offset: u64, blob_id: BlobId, buf: &[u8])->Result<()>{
        let blob = self.bs.open_blob(blob_id).await?;
        let channel = self.bs.alloc_io_channel()?;
        blob.write(&channel, offset, buf).await?;
        drop(channel);
        Ok(())
    }

    async fn read(&self, offset: u64, blob_id: BlobId, buf: &mut [u8]) -> Result<()>{
        let blob = self.bs.open_blob(blob_id).await?;
        let channel = self.bs.alloc_io_channel()?;
        blob.read(&channel, offset, buf).await?;
        drop(channel);
        Ok(())
    }

    /// create a blob with #size clusters 
    /// note: size is number of clusters, usually 1MB per cluster
    async fn create_blob(&self, size: u64) -> Result<EngineBlob>{
        let blob_id = self.bs.create_blob().await?;
        let blob = self.bs.open_blob(blob_id).await?;
        blob.resize(size).await?;
        blob.sync_metadata().await?;
        Ok(EngineBlob{
            bl: Arc::new(blob),
        })
    }
}







