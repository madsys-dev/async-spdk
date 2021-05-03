use crate::{Result, SpdkError};
use spdk_sys::*;
use std::{
    ffi::{c_void, CString},
    future::Future,
    mem::MaybeUninit,
    os::raw::c_int,
    pin::Pin,
    task::{Context, RawWakerVTable, Waker},
};
use std::{os::raw::c_char, task::RawWaker};

pub struct AppOpts(spdk_app_opts);

impl AppOpts {
    pub fn new() -> Self {
        let mut opts = MaybeUninit::uninit();
        extern "C" {
            fn spdk_app_opts_init(opts: *mut spdk_app_opts, size: usize);
        }
        unsafe {
            spdk_app_opts_init(opts.as_mut_ptr(), std::mem::size_of::<spdk_app_opts>());
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
    pub fn block_on<F: Future>(mut self, future: F) -> Result<()> {
        extern "C" fn start_fn<F: Future>(future: *mut c_void) {
            let future = unsafe { *Box::from_raw(future as *mut F) };
            spawn_internal(future, true);
        }
        let err = unsafe {
            spdk_app_start(
                &mut self.0,
                Some(start_fn::<F>),
                Box::into_raw(Box::new(future)) as *mut c_void,
            )
        };
        unsafe {
            spdk_app_fini();
        }
        SpdkError::from_retval(err)?;
        Ok(())
    }
}

pub fn spawn<F: Future>(future: F) {
    spawn_internal(future, false);
}

fn spawn_internal<F: Future>(future: F, main: bool) {
    struct Task<F> {
        future: F,
        poller: *mut spdk_poller,
        waker: Waker,
        main: bool,
    }
    extern "C" fn poller_wrapper<F: Future>(task: *mut c_void) -> c_int {
        let task = unsafe { &mut *(task as *mut Task<F>) };
        let mut context = Context::from_waker(&task.waker);
        let future = unsafe { Pin::new_unchecked(&mut task.future) };
        if future.poll(&mut context).is_pending() {
            unsafe { spdk_poller_pause(task.poller) };
            // return positive to indicate that polling took place and some events were processed.
            return 1;
        }
        // ready
        unsafe {
            let main = task.main;
            spdk_poller_unregister(&mut task.poller);
            Box::from_raw(task);
            if main {
                spdk_app_stop(0);
            }
        }
        return 1;
    }
    let task = Box::leak(Box::new(Task {
        future,
        poller: std::ptr::null_mut(),
        waker: unsafe { poller_waker(std::ptr::null_mut()) },
        main,
    }));
    let poller =
        unsafe { spdk_poller_register(Some(poller_wrapper::<F>), task as *mut Task<F> as _, 0) };
    assert!(!poller.is_null());
    task.poller = poller;
    task.waker = unsafe { poller_waker(poller) };
}

unsafe fn poller_waker(poller: *mut spdk_poller) -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |data| RawWaker::new(data, &VTABLE),                 // clone
        |poller| unsafe { spdk_poller_resume(poller as _) }, // wake
        |poller| unsafe { spdk_poller_resume(poller as _) }, // wake_by_ref
        |_| {},                                              // drop
    );
    Waker::from_raw(RawWaker::new(poller as _, &VTABLE))
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
