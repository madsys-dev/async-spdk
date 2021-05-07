use crate::{Result, SpdkError};
use spdk_sys::*;

/// List of CPUs.
pub struct CpuSet {
    pub(crate) ptr: *mut spdk_cpuset,
}

impl CpuSet {
    /// Allocate CPU set object.
    pub fn new() -> Result<Self> {
        let ptr = unsafe { spdk_cpuset_alloc() };
        if ptr.is_null() {
            // FIXME: proper error
            return Err(SpdkError::from(-1));
        }
        Ok(CpuSet { ptr })
    }

    /// Set or clear CPU state in CPU set.
    pub fn set(&mut self, cpu: u32, state: bool) {
        unsafe { spdk_cpuset_set_cpu(self.ptr, cpu, state) };
    }

    /// Get the state of CPU in CPU set.
    pub fn get(&mut self, cpu: u32) -> bool {
        unsafe { spdk_cpuset_get_cpu(self.ptr, cpu) }
    }

    /// Get the number of CPUs that are set in CPU set.
    pub fn count(&self) -> u32 {
        unsafe { spdk_cpuset_count(self.ptr) }
    }
}

impl Drop for CpuSet {
    fn drop(&mut self) {
        unsafe { spdk_cpuset_free(self.ptr) };
    }
}
