use std::alloc::{GlobalAlloc, Layout};

unsafe extern "C" {
    fn rusteal_engine_malloc(size: usize, alignment: u32) -> *mut u8;
    fn rusteal_engine_malloc_zeroed(size: usize, alignment: u32) -> *mut u8;
    fn rusteal_engine_realloc(original: *mut u8, size: usize, alignment: u32) -> *mut u8;
    fn rusteal_engine_free(original: *mut u8);
}

pub struct EngineAllocator;

unsafe impl GlobalAlloc for EngineAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { rusteal_engine_malloc(layout.size(), layout.align() as u32) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { rusteal_engine_malloc_zeroed(layout.size(), layout.align() as u32) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { rusteal_engine_free(ptr) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        unsafe { rusteal_engine_realloc(ptr, new_size, layout.align() as u32) }
    }
}
