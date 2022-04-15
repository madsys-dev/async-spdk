use std::sync::Arc;
use async_spdk::blob::*;

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

#[derive(Debug)]
pub struct EngineBlob{
    pub bl: Arc<Blob>,
}

impl EngineBlob{
    pub fn get_id(&self) -> Result<BlobId>{
        Ok(self.bl.blob_id())
    }
}


// unsafe impl Send for EngineBlob {}
// unsafe impl Sync for EngineBlob {}

// unsafe impl Send for DeviceEngine {}
// unsafe impl Sync for DeviceEngine {}