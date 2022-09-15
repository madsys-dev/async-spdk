use crate::{cpuset::CpuSet, Result, SpdkError};
use spdk_sys::*;
use std::{
    ffi::{c_void, CString},
    os::raw::c_int,
};
use log::*;

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
        info!("poller registered");
        Ok(Poller { ptr, closure })
    }

    /// Pause a poller on the current thread.
    pub fn pause(&mut self) {
        unsafe {
            spdk_poller_pause(self.ptr);
        }
    }

    /// Resume a poller on the current thread.
    pub fn resume(&mut self) {
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

#[derive(Debug)]
pub struct Thread {
    ptr: *mut spdk_thread,
}

impl Thread {
    /// Creates a new SPDK thread object.
    pub fn create(name: &str, cpumask: &CpuSet) -> Result<Self> {
        let cname = CString::new(name).expect("Couldn't create a string");
        let ptr = unsafe { spdk_thread_create(cname.as_ptr(), cpumask.ptr) };
        if ptr.is_null() {
            // FIXME: proper error
            return Err(SpdkError::from(-1));
        }
        Ok(Thread { ptr })
    }

    /// Force the current system thread to act as if executing the given SPDK thread.
    pub fn set(&mut self) {
        unsafe { spdk_set_thread(self.ptr) };
    }

    /// Perform one iteration worth of processing on the thread.
    ///
    /// This includes both expired and continuous pollers as well as messages.
    /// If the thread has exited, return immediately.
    pub fn poll(&mut self, max_msgs: u32) -> bool {
        let done = unsafe { spdk_thread_poll(self.ptr, max_msgs, 0) };
        done != 0
    }

    /// Return the number of ticks until the next timed poller would expire.
    ///
    /// Timed pollers are pollers for which period_microseconds is greater than 0.
    pub fn next_poller_expiration(&self) -> u64 {
        unsafe { spdk_thread_next_poller_expiration(self.ptr) }
    }

    /// Mark the thread as exited.
    pub fn exit(&mut self) {
        unsafe { spdk_thread_exit(self.ptr) };
    }
}

impl Drop for Thread {
    /// Destroy a thread, releasing all of its resources.
    ///
    /// May only be called on a thread previously marked as exited.
    fn drop(&mut self) {
        unsafe { spdk_thread_destroy(self.ptr) };
    }
}

/// Initialize the threading library.
///
/// Must be called once prior to allocating any threads.
pub fn init() {
    unsafe { spdk_thread_lib_init(None, 0) };
}

/// Release all resources associated with this library.
pub fn fini() {
    unsafe { spdk_thread_lib_fini() };
}
