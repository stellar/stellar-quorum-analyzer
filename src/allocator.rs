use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TrackingAllocator(AtomicUsize);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.fetch_add(layout.size(), Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout);
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator(AtomicUsize::new(0));

pub fn get_memory_usage() -> usize {
    ALLOCATOR.0.load(Ordering::SeqCst)
}
