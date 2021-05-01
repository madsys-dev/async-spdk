use spdk_sys::*;
use std::ffi::CString;

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
}
