use crate::{Result, SpdkError};
use log::*;
use spdk_sys::*;
use std::{
    ffi::{c_void, CString},
    mem::MaybeUninit,
};

/// SPDK blob store block device.
///
/// This is a virtual representation of a block device that is exported by the backend.
/// TODO: Implement Drop
#[derive(Debug)]
pub struct BlobStoreBDev {
    pub(crate) ptr: *mut spdk_bs_dev,
}

impl BlobStoreBDev {
    /// Create a blobstore block device from a bdev.
    pub fn create(name: &str) -> Result<Self> {
        let cname = CString::new(name).expect("Couldn't create a string");
        let mut ptr = MaybeUninit::uninit();
        extern "C" fn callback(
            ty: spdk_bdev_event_type,
            bdev: *mut spdk_bdev,
            event_ctx: *mut c_void,
        ) {
            warn!(
                "bdev callback: type={:?}, bdev={:?}, ctx={:?}",
                ty, bdev, event_ctx
            );
        }
        let err = unsafe {
            spdk_bdev_create_bs_dev_ext(
                cname.as_ptr(),
                Some(callback),
                std::ptr::null_mut(),
                ptr.as_mut_ptr(),
            )
        };
        SpdkError::from_retval(err)?;
        Ok(BlobStoreBDev {
            ptr: unsafe { ptr.assume_init() },
        })
    }
}
