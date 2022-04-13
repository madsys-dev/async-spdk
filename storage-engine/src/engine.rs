use async_spdk::*;
use std::sync::Arc;
use crate::traits::DevEngine;
use async_spdk::blob::*;
use async_trait::async_trait;

#[derive(Debug)]
pub struct DeviceEngine{
    bs: Arc<Blobstore>,
    name: String,
    io_size: u64,
}

#[derive(Debug)]
pub struct EngineBlob{
    bl: Blob,
}

unsafe impl Send for EngineBlob {}
unsafe impl Sync for EngineBlob {}

unsafe impl Send for DeviceEngine {}
unsafe impl Sync for DeviceEngine {}

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
    
    /// helper for 'new' function
    async fn open_bs(name: &str) -> Result<Blobstore>{
        let mut bs_dev = blob_bdev::BlobStoreBDev::create(name)?;
        let bs = blob::Blobstore::init(&mut bs_dev).await?;
        Ok(bs)
    }

    /// close blobstore handle
    /// todo: check io_channel closed or not
    pub async fn close(&mut self){
        self.bs.unload().await?;
    }

}

#[async_trait]
impl DevEngine for DeviceEngine{
    async fn write(&self, offset: u64, blobId: BlobId, buf: &[u8])->Result<()>{
        let blob = self.bs.open_blob(blobId).await?;
        let channel = self.bs.alloc_io_channel()?;
        blob.write(&channel, offset, buf).await?;
        drop(channel);
        Ok(())
    }

    async fn read(&self, offset: u64, blobId: BlobId, buf: &mut [u8]) -> Result<()>{
        let blob = self.bs.open_blob(blobId).await?;
        let channel = self.bs.alloc_io_channel()?;
        blob.read(&channel, offset, buf).await?;
        drop(channel);
        Ok(())
    }

    async fn create_blob(&self, size: u64) -> Result<EngineBlob>{
        let blob_id = self.bs.create_blob().await?;
        let blob = self.bs.open_blob(blob_id).await?;
        blob.resize(size).await?;
        blob.sync_metadata().await?;
        Ok(EngineBlob{
            bl: blob,
        })
    }
}







