use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use lralloc_rs::{do_malloc, do_free, do_aligned_alloc};
use core::ops::RangeTo;
use std::thread;


struct Dummy;

#[global_allocator]
static allocator: Dummy = Dummy;

unsafe impl GlobalAlloc for Dummy {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // do_malloc(layout.size())
        let output = do_aligned_alloc(layout.align(), layout.size());
        output
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        do_free(ptr)
    }
}

#[test]
fn global_allocator() {

    let mut vec: Vec<_> = (0..100)
        .collect::<Vec<usize>>();



    for i in 0usize..100 {
        assert_eq!(i, vec[i])
    }


    let v : Vec<_> =vec.drain(0..100).collect();
    assert_eq!(vec.len(), 0);
    assert_eq!(v.len(), 100);

}

#[test]
fn mass_stress() {
    for j in 0..5000 {
        let mut vec = vec![];
        for i in 0..8 {
            vec.push(thread::spawn(move ||
                {
                    do_malloc(8);
                    println!("Thread {} says hello", j * 8 + i)
                }));
        }
        for join in vec {
            join.join();
        }
    }
}