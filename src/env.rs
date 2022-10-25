use spdk_sys::*;

#[derive(Debug)]
pub struct DmaBuf {
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for DmaBuf {}
unsafe impl Sync for DmaBuf {}

impl DmaBuf {
    /// Allocate a pinned memory buffer with the given size and alignment.
    pub fn alloc(size: usize, align: usize) -> DmaBuf {
        let ptr = unsafe { spdk_dma_malloc(size as u64, align as u64, std::ptr::null_mut()) };
        assert!(!ptr.is_null(), "Failed to malloc");
        DmaBuf {
            ptr: ptr as _,
            len: size as usize,
        }
    }

    /// Allocate a pinned memory buffer with the given size and alignment.
    /// The buffer will be zeroed.
    pub fn alloc_zeroed(size: usize, align: usize) -> DmaBuf {
        let ptr = unsafe { spdk_dma_zmalloc(size as u64, align as u64, std::ptr::null_mut()) };
        assert!(!ptr.is_null(), "Failed to malloc");
        DmaBuf {
            ptr: ptr as _,
            len: size as usize,
        }
    }

    /// Converts to a raw pointer.
    pub const fn as_ptr(&self) -> *const u8 {
        self.ptr as _
    }
}

impl AsRef<[u8]> for DmaBuf {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl AsMut<[u8]> for DmaBuf {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for DmaBuf {
    fn drop(&mut self) {
        unsafe { spdk_dma_free(self.ptr as _) }
    }
}
