use crate::page_map::PageInfo;
use crate::independent_collections::Array;
use std::ptr::null_mut;
use std::ops::RangeInclusive;
use crate::allocation_data::{ProcHeap, Descriptor};
use crate::mem_info::PAGE_MASK;

struct Node {
    range: RangeInclusive<usize>,
    inner: NodeInner
}

impl Node {

    fn create_parent_node(&self, other: &Self) -> Self {
        let new_range = *self.range.start().min(other.range.start())..=*self.range.end().max(other.range.end());
        let (less, more) = if self.range.end() < other.range.start() {
            (self as *const Node as *mut Node, other as *const Node as *mut Node)
        } else {
            (other as *const Node as *mut Node, self as *const Node as *mut Node)
        };

        Node {
            range: new_range,
            inner: NodeInner::Children { less, more }
        }
    }

    pub fn find_info_for_ptr<T : ?Sized>(&self, ptr: * const T) -> Option<&PageInfo> {
        self.find_info(ptr as *mut u8 as usize)
    }

    fn find_info(&self, address: usize) -> Option<&PageInfo> {
        if self.range.contains(&address) {
            match &self.inner {
                NodeInner::Info(info) => {
                    Some(info)
                },
                NodeInner::Children { less, more } => {
                    unsafe {
                        let less = & **less;
                        let more = & **more;

                        if less.range.contains(&address) {
                            less.find_info(address)
                        } else if more.range.contains(&address) {
                            more.find_info(address)
                        } else {
                            None
                        }
                    }
                },
            }
        } else {
            None
        }
    }
}

enum NodeInner {
    Info(PageInfo),
    Children { less: *mut Node, more: *mut Node }
}

pub struct PageRangeMapping {
    head: *mut Node,
    memory_array: Array<Node>
}

impl PageRangeMapping {

    pub const fn new() -> Self {
        PageRangeMapping {
            head: null_mut(),
            memory_array: Array::new()
        }
    }

    pub fn init_with_capacity(&mut self, capacity: usize) {
        self.memory_array = Array::with_capacity(capacity);
    }

    fn create_range_node(&mut self, range: RangeInclusive<usize>, info: PageInfo) -> *mut Node {
        let node = Node {
            range,
            inner: NodeInner::Info(info)
        };
        self.memory_array.push(node);
        self.memory_array.last_mut().unwrap() as *mut Node
    }

    pub fn get_page_info<T : ?Sized>(&self, ptr: *const T) -> Option<&PageInfo> {
        if self.head.is_null() {
           return None;
        }
        let head = unsafe { & *self.head };
        head.find_info_for_ptr(ptr)
    }

    unsafe fn insert_node(&mut self, node: *mut Node) {
        if self.head.is_null() {
            self.head = node;
            return;
        }

        //let mut last = null_mut();
        let mut ptr = self.head;
        let main_range = & (*node).range;
        while let NodeInner::Children { less, more } = &(*ptr).inner {
            let less_range = &(**less).range;
            let more_range = &(**more).range;
            if main_range.end() < less_range.start() {
                ptr = *less;
                break;
            } else if less_range.start() < main_range.start() &&
                    less_range.end() > main_range.end() {
                ptr = *less;
            } else if main_range.start() > more_range.end() {
                ptr = *more;
                break;
            } else if more_range.start() < main_range.start() &&
                    main_range.end() > main_range.end() {
                ptr = *more;
            } else {
                break;
            }
        }

        let combined = (*node).create_parent_node(&*ptr);
        /*
        self.memory_array.push(combined);
        let created_ptr = self.memory_array.last().unwrap() as *const Node;

         */
        ptr.write(combined);
    }




    fn set_page_info_for_ptr(&mut self, ptr: *mut u8, info: PageInfo) {
        let range = (ptr as usize)..=(ptr as usize);
        let node = self.create_range_node(range, info);
        unsafe {
            self.insert_node(node);
        }
    }

    pub fn update_page_map(
        &mut self,
        heap: Option<&mut ProcHeap>,
        ptr: *mut u8,
        desc: Option<&mut Descriptor>,
        size_class_index: usize,
    ) {
        let mut info = PageInfo::default();
        info.set_ptr(desc.map_or(null_mut(), |d| d as *mut Descriptor),
                     size_class_index,);

        if heap.is_none() {
            self.set_page_info_for_ptr(ptr, info);
            return;
        }

        let heap = heap.unwrap();
        let sb_size = heap.get_size_class().sb_size as usize;
        assert_eq!(
            sb_size & PAGE_MASK,
            0,
            "sb_size must be a multiple of a page"
        );
        let range = (ptr as usize)..=(ptr as usize + sb_size - 1);
        let node = self.create_range_node(range, info);
        unsafe {
            self.insert_node(node);
        }
    }


}


