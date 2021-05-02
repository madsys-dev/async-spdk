use crate::{Result, SpdkError};
use spdk_sys::*;
use std::os::raw::c_char;
use std::{
    ffi::{c_void, CString},
    mem::MaybeUninit,
};

pub struct AppOpts(spdk_app_opts);

impl AppOpts {
    pub fn new() -> Self {
        let mut opts = MaybeUninit::uninit();
        unsafe {
            spdk_app_opts_init(opts.as_mut_ptr());
            AppOpts(opts.assume_init())
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.0.name = CString::new(name)
            .expect("Couldn't create a string")
            .into_raw();
        self
    }

    pub fn config_file(mut self, config_file: &str) -> Self {
        self.0.config_file = CString::new(config_file)
            .expect("Couldn't create a string")
            .into_raw();
        self
    }

    /// Start the framework.
    pub fn start<F: FnOnce()>(mut self, f: F) -> Result<()> {
        extern "C" fn start_fn<F: FnOnce()>(closure: *mut c_void) {
            let f = unsafe { Box::from_raw(closure as *mut F) };
            f();
        }
        let err = unsafe {
            spdk_app_start(
                &mut self.0,
                Some(start_fn::<F>),
                Box::into_raw(Box::new(f)) as *mut c_void,
            )
        };
        unsafe {
            spdk_app_fini();
        }
        SpdkError::from_retval(err)?;
        Ok(())
    }
}

/// Stop the framework.
pub fn app_stop(rc: i32) {
    unsafe {
        spdk_app_stop(rc);
    }
}

impl Drop for AppOpts {
    fn drop(&mut self) {
        drop_if_not_null(self.0.name);
        drop_if_not_null(self.0.config_file);
    }
}

fn drop_if_not_null(string: *const c_char) {
    if !string.is_null() {
        unsafe { CString::from_raw(string as *mut c_char) };
    }
}
