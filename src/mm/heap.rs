use crate::sync::Spinlock;
use core::alloc::{GlobalAlloc, Layout};

pub const HEAP_START_ADDR: usize = 0x100000; // 1 MB mark
pub const HEAP_INITIAL_SIZE: usize = 2 * 1024 * 1024; // 2 MB heap
const HEAP_MAGIC: u32 = 0xCAFEBABE;

#[repr(C)]
struct HeapBlock {
    magic: u32,
    size: usize,
    is_free: bool,
    next: *mut HeapBlock,
    prev: *mut HeapBlock,
}

const BLOCK_HEADER_SIZE: usize = core::mem::size_of::<HeapBlock>();

#[inline]
const fn align8(x: usize) -> usize {
    (x + 7) & !7
}

#[derive(Debug, Copy, Clone)]
pub struct HeapStats {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
    pub num_allocations: usize,
    pub num_blocks: usize,
}

struct Heap {
    head: *mut HeapBlock,
    total_size: usize,
}

unsafe impl Send for Heap {}

impl Heap {
    const fn new() -> Self {
        Self {
            head: core::ptr::null_mut(),
            total_size: 0,
        }
    }

    fn init(&mut self, start: usize, size: usize) {
        self.total_size = size;
        self.head = start as *mut HeapBlock;
        unsafe {
            (*self.head).magic = HEAP_MAGIC;
            (*self.head).size = size - BLOCK_HEADER_SIZE;
            (*self.head).is_free = true;
            (*self.head).next = core::ptr::null_mut();
            (*self.head).prev = core::ptr::null_mut();
        }
    }

    fn kmalloc(&mut self, size: usize) -> *mut u8 {
        if size == 0 || self.head.is_null() {
            return core::ptr::null_mut();
        }

        let aligned_size = align8(size);
        let mut curr = self.head;

        while !curr.is_null() {
            unsafe {
                if (*curr).magic != HEAP_MAGIC {
                    return core::ptr::null_mut();
                }

                if (*curr).is_free && (*curr).size >= aligned_size {
                    // Check if block can be split
                    if (*curr).size >= aligned_size + BLOCK_HEADER_SIZE + 16 {
                        let split_addr = (curr as *mut u8)
                            .add(BLOCK_HEADER_SIZE + aligned_size)
                            as *mut HeapBlock;
                        (*split_addr).magic = HEAP_MAGIC;
                        (*split_addr).size = (*curr).size - aligned_size - BLOCK_HEADER_SIZE;
                        (*split_addr).is_free = true;
                        (*split_addr).next = (*curr).next;
                        (*split_addr).prev = curr;

                        if !(*curr).next.is_null() {
                            (*(*curr).next).prev = split_addr;
                        }
                        (*curr).next = split_addr;
                        (*curr).size = aligned_size;
                    }

                    (*curr).is_free = false;
                    return (curr as *mut u8).add(BLOCK_HEADER_SIZE);
                }

                curr = (*curr).next;
            }
        }

        core::ptr::null_mut()
    }

    fn kfree(&mut self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        unsafe {
            let block = (ptr.sub(BLOCK_HEADER_SIZE)) as *mut HeapBlock;
            if (*block).magic != HEAP_MAGIC {
                return;
            }

            (*block).is_free = true;

            // Merge with next block if free (coalescing)
            if !(*block).next.is_null() && (*(*block).next).is_free {
                let next = (*block).next;
                (*block).size += BLOCK_HEADER_SIZE + (*next).size;
                (*block).next = (*next).next;
                if !(*block).next.is_null() {
                    (*(*block).next).prev = block;
                }
            }

            // Merge with previous block if free
            if !(*block).prev.is_null() && (*(*block).prev).is_free {
                let prev = (*block).prev;
                (*prev).size += BLOCK_HEADER_SIZE + (*block).size;
                (*prev).next = (*block).next;
                if !(*block).next.is_null() {
                    (*(*block).next).prev = prev;
                }
            }
        }
    }

    fn krealloc(&mut self, ptr: *mut u8, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return self.kmalloc(new_size);
        }
        if new_size == 0 {
            self.kfree(ptr);
            return core::ptr::null_mut();
        }

        unsafe {
            let block = (ptr.sub(BLOCK_HEADER_SIZE)) as *mut HeapBlock;
            if (*block).magic != HEAP_MAGIC {
                return core::ptr::null_mut();
            }

            let aligned_new = align8(new_size);
            if (*block).size >= aligned_new {
                return ptr;
            }

            let old_size = (*block).size;
            let new_ptr = self.kmalloc(new_size);
            if !new_ptr.is_null() {
                core::ptr::copy_nonoverlapping(ptr, new_ptr, old_size);
                self.kfree(ptr);
            }
            new_ptr
        }
    }

    fn stats(&self) -> HeapStats {
        let mut stats = HeapStats {
            total_bytes: self.total_size,
            used_bytes: 0,
            free_bytes: 0,
            num_allocations: 0,
            num_blocks: 0,
        };

        let mut curr = self.head;
        while !curr.is_null() {
            unsafe {
                stats.num_blocks += 1;
                if (*curr).is_free {
                    stats.free_bytes += (*curr).size;
                } else {
                    stats.used_bytes += (*curr).size;
                    stats.num_allocations += 1;
                }
                curr = (*curr).next;
            }
        }

        stats
    }
}

static HEAP: Spinlock<Heap> = Spinlock::new(Heap::new());

pub fn heap_init(start: usize, size: usize) {
    HEAP.lock().init(start, size);
}

pub fn kmalloc(size: usize) -> *mut u8 {
    HEAP.lock().kmalloc(size)
}

pub fn kfree(ptr: *mut u8) {
    HEAP.lock().kfree(ptr);
}

pub fn krealloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    HEAP.lock().krealloc(ptr, new_size)
}

pub fn heap_get_stats() -> HeapStats {
    HEAP.lock().stats()
}

struct PweOsGlobalAlloc;

unsafe impl GlobalAlloc for PweOsGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        kmalloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        kfree(ptr);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        krealloc(ptr, new_size)
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: PweOsGlobalAlloc = PweOsGlobalAlloc;
