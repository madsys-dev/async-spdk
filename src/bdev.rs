use spdk_sys::*;
use std::{ffi::{CString, c_void}, mem::MaybeUninit};
use log::*;
use crate::{Result, SpdkError, blob::IoChannel};
use std::os::raw::c_int;
use crate::complete::LocalComplete;
use std::{
    ops::{Deref, DerefMut},
    slice::{from_raw_parts, from_raw_parts_mut},
};

#[derive(Debug)]
pub struct BdevIochannel{
    pub ptr: *mut spdk_io_channel,
}

impl Drop for BdevIochannel{
    fn drop(&mut self) {
        unsafe { spdk_put_io_channel(self.ptr) };
    }
}

/// SPDK block device.
/// TODO: Implement Drop
#[derive(Debug)]
pub struct BDev {
    ptr: *mut spdk_bdev,
}

impl BDev {
    /// Get block device by the block device name.
    pub fn get_by_name(name: &str) -> Option<Self> {
        let cname = CString::new(name).expect("Couldn't create a string");
        let ptr = unsafe { spdk_bdev_get_by_name(cname.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        Some(BDev { ptr })
    }

    pub fn get_block_size(&self) -> u32{
        let ret = unsafe{
            spdk_bdev_get_block_size(self.ptr)
        };
        ret
    }

    pub fn get_buf_align(&self) -> usize{
        let ret = unsafe{
            spdk_bdev_get_buf_align(self.ptr) as usize
        };
        ret
    }
}

#[derive(Debug)]
pub struct BDevDesc{
    ptr: *mut spdk_bdev_desc,
}

impl BDevDesc{
    pub fn create_desc(name: &str) -> Result<Self>{
        let cname = CString::new(name).expect("Could not parse to CString");
        let mut ptr = MaybeUninit::uninit();
        extern "C" fn callback(
            ty: spdk_bdev_event_type,
            bdev: *mut spdk_bdev,
            event_ctx: *mut c_void,
        ){
            warn!(
                "bdev callback: type = {:?}, bdev={:?}, ctx={:?}",
                ty, bdev, event_ctx
            );
        }
        let err = unsafe{
            spdk_bdev_open_ext(
                cname.as_ptr(),
                true,
                Some(callback),
                std::ptr::null_mut(),
                ptr.as_mut_ptr(),
            )
        };
        SpdkError::from_retval(err)?;
        Ok(BDevDesc{
            ptr: unsafe {
                ptr.assume_init()
            },
        })
    }

    pub fn get_bdev(&self) -> Result<BDev>{
        let ptr = unsafe{
            spdk_bdev_desc_get_bdev(self.ptr)
        };
        if ptr.is_null(){
            return Err(SpdkError::from(-1));
        }
        Ok(BDev{
            ptr,
        })
    }

    pub fn get_io_channel(&self) -> Result<IoChannel>{
        let ptr = unsafe{ spdk_bdev_get_io_channel(self.ptr) };
        if ptr.is_null(){
            return Err(SpdkError::from(-1));
        }
        Ok(IoChannel {ptr})
    }
    
    pub fn close(&self){
        unsafe{
            spdk_bdev_close(self.ptr);
        }
    }

    pub async fn write(
        &self, 
        io_channel: &IoChannel, 
        offset: u64,
        length: u64,
        buf: &[u8]
    )-> Result<()>{
        do_async(|arg| unsafe{
            spdk_bdev_write(
                self.ptr,
                io_channel.ptr,
                buf.as_ptr() as _,
                offset,
                length,
                Some(callback),
                arg,
            );
        })
        .await
    }

    pub async fn read(
        &self,
        io_channel: &IoChannel,
        offset: u64,
        length: u64,
        buf: &mut [u8],
    ) -> Result<()>{
        do_async(|arg| unsafe{
            spdk_bdev_read(
                self.ptr,
                io_channel.ptr,
                buf.as_mut_ptr() as _,
                offset,
                length,
                Some(callback),
                arg,
            );
        })
        .await
    }
}


#[warn(dead_code)]
#[derive(Debug)]
pub struct IoWaitEntry{
    wentry: spdk_bdev_io_wait_entry,
}

#[derive(Debug)]
pub struct BdevIo{
    ptr: *mut spdk_bdev_io,
}

impl BdevIo{
    pub fn free_io(&self){
        unsafe{
            spdk_bdev_free_io(self.ptr)
        };
    }
}

#[derive(Debug)]
pub struct dma_buf{
    buf: *mut c_void,
    length: usize,
}

impl dma_buf{
    pub fn as_slice(&self) -> &[u8] {
        unsafe { from_raw_parts(self.buf as *mut u8, self.length as usize) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { from_raw_parts_mut(self.buf as *mut u8, self.length as usize) }
    }

    pub fn fill(&mut self, val: u8) {
        unsafe {
            std::ptr::write_bytes(
                self.as_mut_slice().as_ptr() as *mut u8,
                val,
                self.length as usize,
            )
        }
    }

    pub fn new(size: u64, alignment: u64) -> Result<dma_buf> {
        let buf;
        unsafe {
            buf = spdk_zmalloc(
                size,
                alignment,
                std::ptr::null_mut(),
                SPDK_ENV_LCORE_ID_ANY as i32,
                SPDK_MALLOC_DMA,
            )
        };

        if buf.is_null() {
            Err(SpdkError::from(-1))
        } else {
            Ok(dma_buf {
                buf,
                length: size as usize,
            })
        }
    }

    pub fn len(&self) -> u64 {
        self.length as u64
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl Deref for dma_buf {
    type Target = *mut c_void;

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl DerefMut for dma_buf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}

impl Drop for dma_buf {
    fn drop(&mut self) {
        unsafe { spdk_dma_free(self.buf as *mut c_void) }
    }
}

extern "C" fn callback(
    bio: *mut spdk_bdev_io, 
    s: bool, 
    arg: *mut c_void,
){
    callback_with(arg, (), s, bio);
}

extern "C" fn callback_with<T>(
    arg: *mut c_void,
    bs: T,
    s: bool,
    bio: *mut spdk_bdev_io,
){
    let complete = unsafe{
        &mut *(arg as *mut LocalComplete<Result<T>>)
    };
    
    let result = if !s{
        Err(SpdkError::from(-1))
    }else{
        Ok(bs)
    };
    complete.complete(result);
    unsafe{
        spdk_bdev_free_io(bio);
    }
}

async fn do_async<T: Unpin>(f: impl FnOnce(*mut c_void)) -> Result<T> {
    let complete = LocalComplete::<Result<T>>::new();
    futures_lite::pin!(complete);
    f(complete.as_arg());
    complete.await
}


