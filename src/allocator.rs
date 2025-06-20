use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct LimitedAllocator {
    limit: AtomicUsize,
    allocated: AtomicUsize,
}

unsafe impl GlobalAlloc for LimitedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let new_size = self.allocated.fetch_add(layout.size(), Ordering::SeqCst);
        if new_size > self.limit.load(Ordering::SeqCst) {
            self.allocated.fetch_sub(layout.size(), Ordering::SeqCst);
            std::ptr::null_mut()
        } else {
            System.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.allocated.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout);
    }
}

impl LimitedAllocator {
    pub fn set_limit(&self, bytes: usize) {
        self.limit.store(bytes, Ordering::SeqCst);
    }
}

// provide a sensible default; can be overwritten by calling set_limit
const DEFAULT_LIMIT: usize = 2 * 1024 * 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LimitedAllocator = LimitedAllocator {
    limit: AtomicUsize::new(DEFAULT_LIMIT),
    allocated: AtomicUsize::new(0),
};

pub fn get_memory_usage() -> usize {
    ALLOCATOR.allocated.load(Ordering::SeqCst)
}

pub fn set_memory_limit(bytes: usize) {
    ALLOCATOR.set_limit(bytes);
}
