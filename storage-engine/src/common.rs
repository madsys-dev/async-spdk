use std::sync::Arc;
use async_spdk::blob::*;
use async_spdk::SpdkError;

pub type Result<T> = std::result::Result<T, SpdkError>;

#[derive(Debug)]
pub struct DeviceEngine{
    pub bs: Arc<Blobstore>,
    pub name: String,
    pub io_size: u64,
    pub config_file: String,
}

/// supported operation list
pub enum Op{
    /// data write/read offset, bid, buf should be provided
    Write, 
    Read,
    /// create a new blob, blob_size should be provided
    CreateBlob,
}

#[derive(Debug, Copy, Clone)]
pub struct OpCtx{
    offset: u64,
    blob_id: BlobId,
    blob_size: u64,
}

impl OpCtx{
    pub fn init_io_ctx(offset: u64, blob_id: BlobId)-> Self{
        Self{
            offset: offset,
            blob_id: blob_id,
            blob_size: 0,
        }
    }

    pub fn init_blob_ctx(blob_size: u64) -> Self{
        Self{
            offset: 0,
            blob_id: BlobId::set_blob_id(0),
            blob_size: blob_size,
        }
    }
}


impl DeviceEngine{
    pub fn get_name(&self) ->Result<String>{
        Ok(self.name.clone())
    }

    pub fn get_io_size(&self) -> Result<u64>{
        Ok(self.io_size)
    }

    pub fn get_config_file(&self) -> Result<String>{
        Ok(self.config_file.clone())
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

