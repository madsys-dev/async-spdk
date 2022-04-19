use std::sync::Arc;
use async_spdk::blob::*;
use async_spdk::SpdkError;

pub type Result<T> = std::result::Result<T, SpdkError>;



/// supported operation list
pub enum Op{
    /// data write/read offset, bid, buf should be provided
    Write, 
    Read,
    /// create a new blob, blob_size should be provided
    CreateBlob,
    /// finish IO tasks, call spdk_app_fini
    Fini,
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




#[derive(Debug)]
pub struct EngineBlob{
    pub bl: BlobId,
}

impl EngineBlob{
    pub fn get_id(&self) -> Result<BlobId>{
        Ok(self.bl)
    }
}

