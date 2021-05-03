use crate::{Result, SpdkError};
use spdk_sys::*;
use std::{ffi::c_void, os::raw::c_int};

pub struct Poller {
    ptr: *mut spdk_poller,
    #[allow(dead_code)]
    closure: Box<dyn Fn() -> bool>,
}

impl Poller {
    /// Registers a poller with spdk.
    ///
    /// `f` should return true if any work was done.
    pub fn register<F: Fn() -> bool + 'static>(f: F) -> Result<Self> {
        extern "C" fn poller_wrapper<F: Fn() -> bool + 'static>(closure: *mut c_void) -> c_int {
            let f = unsafe { &*(closure as *const F) };
            f() as _
        }
        let closure = Box::new(f);
        let ptr = unsafe {
            spdk_poller_register(Some(poller_wrapper::<F>), &*closure as *const F as _, 0)
        };
        if ptr.is_null() {
            // FIXME: proper error
            return Err(SpdkError::from(-1));
        }
        Ok(Poller { ptr, closure })
    }

    /// Pause a poller on the current thread.
    pub fn pause(&self) {
        unsafe {
            spdk_poller_pause(self.ptr);
        }
    }

    /// Resume a poller on the current thread.
    pub fn resume(&self) {
        unsafe {
            spdk_poller_resume(self.ptr);
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        unsafe {
            spdk_poller_unregister(&mut self.ptr);
        }
    }
}
