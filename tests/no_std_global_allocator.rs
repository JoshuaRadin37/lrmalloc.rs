#![no_std]


use core::alloc::{GlobalAlloc, Layout};
use lrmalloc_rs::{do_free, do_aligned_alloc};

extern crate alloc;

use alloc::vec::Vec;

struct Dummy;

#[global_allocator]
static ALLOCATOR: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        do_aligned_alloc(layout.align(), layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = layout;
        do_free(ptr)
    }
}

#[test]
fn no_std_global_allocator() {
    let _vec = Vec::<usize>::with_capacity(8);
}