use spdk_sys::*;
use std::{ffi::{CString, c_void}, mem::MaybeUninit};
use log::*;
use crate::{Result, SpdkError, blob::IoChannel};
use std::os::raw::c_int;
use crate::complete::LocalComplete;


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
        unsafe{
            spdk_bdev_get_block_size(self.ptr)
        }
    }

    pub fn get_buf_align(&self) -> usize{
        unsafe{
            spdk_bdev_get_buf_align(self.ptr) as usize
        }
    }

    pub fn release_io_channel(&self, ioc: IoChannel) {
        unsafe{
            spdk_put_io_channel(ioc.ptr);
        }
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
    
    pub async fn close(&self){
        unsafe{
            spdk_bdev_close(self.ptr);
        }
    }

    pub async fn write(
        &self, 
        io_channel: &IoChannel, 
        offset: u64,
        buf: &[u8]
    )-> Result<i32>{
        let l = buf.len() as u64;
        do_async(|arg| unsafe{
            spdk_bdev_write(
                self.ptr,
                io_channel.ptr,
                buf.as_ptr() as _,
                offset,
                l,
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
        buf: &mut [u8],
    ) -> Result<()>{
        let l = buf.len() as u64;
        do_async(|arg| unsafe{
            spdk_bdev_read(
                self.ptr,
                io_channel.ptr,
                buf.as_ptr() as _,
                offset,
                l,
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

extern "C" fn callback(
    bio: *mut spdk_bdev_io, 
    s: bool, 
    arg: *mut c_void,
){
    callback_with(arg, (), s);
}

extern "C" fn callback_with<T>(
    arg: *mut c_void,
    bs: T,
    s: bool,
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
}

async fn do_async<T: Unpin>(f: impl FnOnce(*mut c_void)) -> Result<T> {
    let complete = LocalComplete::<Result<T>>::new();
    futures_lite::pin!(complete);
    f(complete.as_arg());
    complete.await
}


