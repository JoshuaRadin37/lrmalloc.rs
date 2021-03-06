//! A package that creates FFI bindings for LRMalloc-rs to C programs. It also includes an Rust Allocator that, which the user can either
//! disable being set as the global allocator, or not include entirely.

extern crate apfmalloc_lib;

use std::ffi::c_void;
use std::ptr::null_mut;

pub use apfmalloc_lib::{do_aligned_alloc, do_free, do_malloc, do_realloc};
#[cfg(not(feature = "no-rust"))]
pub use rust_global::*;

/// Checks if a call to `malloc` use the lrmalloc-rs implementation.
///
/// Only works after `malloc` has been called at least once.
pub static mut OVERRIDE_MALLOC: bool = false;
/// Checks if a call to `calloc` use the lrmalloc-rs implementation.
///
/// Only works after `calloc` has been called at least once.
pub static mut OVERRIDE_CALLOC: bool = false;
/// Checks if a call to `realloc` use the lrmalloc-rs implementation.
///
/// Only works after `realloc` has been called at least once.
pub static mut OVERRIDE_REALLOC: bool = false;
/// Checks if a call to `free` use the lrmalloc-rs implementation.
///
/// Only works after `free` has been called at least once.
pub static mut OVERRIDE_FREE: bool = false;
/// Checks if a call to `aligned_alloc` use the lrmalloc-rs implementation.
///
/// Only works after `aligned_alloc` has been called at least once.
pub static mut OVERRIDE_ALIGNED_ALLOC: bool = false;

/// Allocates size bytes of uninitialized storage.
///
/// If allocation succeeds, returns a pointer that is suitably aligned for any object type with fundamental alignment.
///
/// If size is zero, a pointer to the minimum sized allocation is created.
///
/// Memory in malloc is aligned to the minimum on the current OS.
#[no_mangle]
pub extern "C" fn malloc(size: usize) -> *mut c_void {
    unsafe {
        OVERRIDE_MALLOC = true;
    }
    /*

    #[cfg(not(target_os = "macos"))]
    {
        do_malloc(size) as *mut c_void
    }
    #[cfg(target_os = "macos")]
    {
        // MacOS requires that memory is aligned to atleast 16 bytes

        do_aligned_alloc(16, size) as *mut c_void
    }

     */
    do_malloc(size) as *mut c_void
}

/// Allocates memory for an array of num objects of size and initializes all bytes in the allocated storage to zero.
///
/// If allocation succeeds, returns a pointer to the lowest (first) byte in the allocated memory block that is suitably aligned for any object type.
///
/// If size is zero, a pointer to the minimum sized allocation is created
#[no_mangle]
pub extern "C" fn calloc(num: usize, size: usize) -> *mut c_void {
    unsafe {
        OVERRIDE_CALLOC = true;
    }
    if num.checked_mul(size).is_none() {
        return null_mut();
    }
    let ret = malloc(num * size) as *mut u8;
    unsafe {
        for i in 0..(num * size) {
            *ret.offset(i as isize) = 0;
        }
    }
    ret as *mut c_void
}
/// Reallocates the given area of memory. It must be previously allocated by malloc(), calloc() or realloc() and not yet freed with a call to free or realloc. Otherwise, the results are undefined.
///
/// The reallocation is done by either:
///
/// 1. Allocating a new memory block of size new_size bytes, copying memory area with size equal the lesser of the new and the old sizes, and freeing the old block.
/// If there is not enough memory, the old memory block is not freed and null pointer is returned.
/// 2. Keeping the block in the same space, if the size class of the new size is the same
///
/// If ptr is NULL, the behavior is the same as calling malloc(new_size).
///
/// If size is zero, a pointer to the minimum sized allocation is created
#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    OVERRIDE_REALLOC = true;
    do_realloc(ptr, new_size)
}

/// Deallocates the space previously allocated by malloc(), calloc(), aligned_alloc() or realloc().
///
/// If ptr is a null pointer, the function does nothing.
///
/// The behavior is undefined if the value of ptr does not equal a value returned earlier by malloc(), calloc(), realloc(), or aligned_alloc() (since C11).
///
/// The behavior is undefined if the memory area referred to by ptr has already been deallocated, that is, free() or realloc() has already been called with ptr as the argument and no calls to malloc(), calloc() or realloc() resulted in a pointer equal to ptr afterwards.
///
/// The behavior is undefined if after free() returns, an access is made through the pointer ptr (unless another allocation function happened to result in a pointer value equal to ptr)
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    OVERRIDE_FREE = true;
    do_free(ptr)
}

/// Has similar behavior to malloc, but also ensures that all memory allocated is also properly aligned to the specified
/// alignment
#[no_mangle]
pub extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void {
    unsafe {
        OVERRIDE_ALIGNED_ALLOC = true;
    }
    if size % alignment != 0 {
        return null_mut();
    }
    /*
    if cfg!(target_os = "macos") {
        let alignment = alignment.max(16); // macos alignment is 16 bytes (?)
        let size = size.max(alignment); // rewrite size if alignment is larger
        do_aligned_alloc(alignment, size) as *mut c_void
    } else {
        do_aligned_alloc(alignment, size) as *mut c_void
    }

     */
    do_aligned_alloc(alignment, size) as *mut c_void
}


/// Uses the posix mem align. This is equivalent to aligned_alloc
///
/// Returns EINVAL if alignment is not a multiple of sizeof(void*)
/// Returns ENOMEM is no more memory is available
#[no_mangle]
pub extern "C" fn posix_memalign(ptr: *mut *mut std::ffi::c_void, alignment: usize, size: usize) -> i32 {
    unsafe {
        OVERRIDE_ALIGNED_ALLOC = true;
    }
    if alignment % std::mem::size_of::<usize>() != 0 {
        return libc::EINVAL;
    }
    let out_ptr = if cfg!(target_os = "macos") {
        let alignment = alignment.max(16);
        aligned_alloc(alignment, size) as *mut c_void
    } else {
        aligned_alloc(alignment, size) as *mut c_void
    };
    if ptr.is_null() {
        return libc::ENOMEM;
    }

    unsafe {
        ptr.write(out_ptr);
    }
    0
}

#[no_mangle]
pub extern "C" fn check_override() -> u8 {
    unsafe {
        let ptr = malloc(8);
        if !OVERRIDE_MALLOC {
            return 0;
        }
        let new_ptr = realloc(ptr, 64);
        assert_ne!(new_ptr, ptr);
        if !OVERRIDE_REALLOC {
            return 0;
        }
        let calloced = calloc(8, 8);
        assert_ne!(new_ptr, calloced);
        if !OVERRIDE_CALLOC {
            return 0;
        }
        free(new_ptr);
        free(calloced);
        if !OVERRIDE_FREE {
            return 0;
        }
    }
    1
}

#[cfg(not(feature = "no-rust"))]
mod rust_global {
    use std::alloc::{GlobalAlloc, Layout};

    use apfmalloc_lib::mem_info::align_val;

    use super::*;

    /// Allows Rust to use aligned allocation instead of using malloc when calling alloc, as alignment data would be lost. This is important
        /// for creating the internal structures of the allocator
    pub struct RustAllocator;

    /// The global allocator structure
    #[cfg(not(feature = "no-rust-global"))]
    #[global_allocator]
    pub static ALLOCATOR: RustAllocator = RustAllocator;

    unsafe impl GlobalAlloc for RustAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            OVERRIDE_ALIGNED_ALLOC = true;
            do_aligned_alloc(layout.align(), layout.size()) as *mut u8
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let _ = layout;
            OVERRIDE_FREE = true;
            do_free(ptr as *mut c_void)
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            calloc(1, align_val(layout.size(), layout.align())) as *mut u8
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            OVERRIDE_REALLOC = true;
            do_realloc(ptr as *mut c_void, align_val(new_size, layout.align())) as *mut u8
        }
    }
}


#[no_mangle]
#[doc(hidden)]
#[cfg(feature = "no-rust")]
pub fn __rust_alloc(size: usize, align: usize) -> *mut u8 {
    aligned_alloc(align, size) as *mut u8
}

#[no_mangle]
#[doc(hidden)]
#[cfg(feature = "no-rust")]
pub fn __rust_alloc_zeroed(size: usize, align: usize) -> *mut u8 {
    unsafe {
        OVERRIDE_CALLOC = true;
    }
    let ret = aligned_alloc(align, size) as *mut u8;
    unsafe {
        for i in 0..size {
            *ret.offset(i as isize) = 0;
        }
    }
    ret
}

#[no_mangle]
#[doc(hidden)]
#[cfg(feature = "no-rust")]
pub fn __rust_dealloc(ptr: *mut u8, _size: usize, _align: usize) {
    unsafe { free(ptr as *mut c_void) }
}

#[no_mangle]
#[doc(hidden)]
#[cfg(feature = "no-rust")]
pub fn __rust_realloc(ptr: *mut u8, _old_size: usize, _align: usize, new_size: usize) -> *mut u8 {
    unsafe { realloc(ptr as *mut c_void, new_size) as *mut u8 }
}



#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn overrides_malloc() {
        unsafe {
            OVERRIDE_MALLOC = false;
            let _ret = libc::malloc(8);
            assert!(OVERRIDE_MALLOC, "Malloc wasn't overwritten!")
        }
    }
    #[test]
    fn overrides_calloc() {
        unsafe {
            OVERRIDE_CALLOC = false;
            let _ret = libc::calloc(1, 8);
            assert!(OVERRIDE_CALLOC, "Calloc wasn't overwritten!")
        }
    }
    #[test]
    fn overrides_realloc() {
        unsafe {
            OVERRIDE_REALLOC = false;
            let first = libc::malloc(8);
            let _ret = libc::realloc(first, 8);
            assert!(OVERRIDE_REALLOC, "Realloc wasn't overwritten!")
        }
    }
    #[test]
    fn overrides_free() {
        unsafe {
            let ret = libc::malloc(8);
            OVERRIDE_FREE = false;
            libc::free(ret);
            assert!(OVERRIDE_FREE, "Free wasn't overwritten!")
        }
    }
}

#[cfg(test)]
mod behavior {
    use super::*;

    #[test]
    fn allocates() {
        let ptr = malloc(std::mem::size_of::<usize>());
        assert!(!ptr.is_null());
        unsafe {
            do_free(ptr);
        }
    }

    #[test]
    fn zero_bytes_malloc_no_fail() {
        let ptr = malloc(0);
        assert!(!ptr.is_null());
        unsafe {
            do_free(ptr);
        }
    }

    #[test]
    fn calloc_fails_on_overflow() {
        assert!(calloc(std::usize::MAX, 2).is_null())
    }

    #[test]
    fn calloc_zeroes() {
        let ptr = calloc(std::mem::size_of::<u8>(), 16) as *mut u8;
        assert!(!ptr.is_null());
        unsafe {
            for i in 0..16 {
                assert_eq!(*ptr.add(i), 0)
            }

            do_free(ptr);
        }
    }

    #[test]
    fn aligned_alloc_checks_consistency() {
        assert!(aligned_alloc(std::mem::size_of::<usize>() + 1, 8).is_null(), "Alignment required to be power of 2");
        assert!(aligned_alloc(std::mem::size_of::<usize>(), std::mem::size_of::<usize>() * 3 / 2).is_null(), "Size must be a multiple of alignment");
    }

    #[test]
    fn realloc_on_null() {
        let ptr = unsafe {
            realloc(null_mut(), 16)
        };

        assert!(!ptr.is_null());
        unsafe {
            do_free(ptr);
        }
    }


    #[test]
    fn realloc_moves_data() {

        let ptr1 = malloc(std::mem::size_of::<usize>()) as *mut usize;
        assert!(!ptr1.is_null());

        unsafe {
            *ptr1 = 16;
            let ptr2 = realloc(ptr1 as *mut c_void, std::mem::size_of::<usize>() * 4) as *mut usize;
            assert_ne!(ptr1, ptr2);
            assert_eq!(16, *ptr2);

            do_free(ptr2);
        }


    }




}

