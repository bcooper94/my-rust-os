use super::{linked_list, Locked};
use alloc::alloc::{Layout, GlobalAlloc};
use core::mem;

struct ListNode {
    next: Option<&'static mut ListNode>,
}

/// The block sizes to use.
///
/// The sizes must each be power of 2 because they are also used as
/// the block alignment (alignments must be always powers of 2).
const BLOCK_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

pub struct FixedSizeBlockAllocator {
    list_heads: [Option<&'static mut ListNode>; BLOCK_SIZES.len()],
    fallback_allocator: linked_list::LinkedListAllocator,
}

impl FixedSizeBlockAllocator {
    /// Create an empty FixedSizeBlockAllocator
    pub const fn new() -> Self {
        FixedSizeBlockAllocator {
            list_heads: [None; BLOCK_SIZES.len()],
            fallback_allocator: linked_list::LinkedListAllocator::new(),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.fallback_allocator.init(heap_start, heap_size);
    }

    fn fallback_alloc(&mut self, layout: Layout) -> *mut u8 {
        unsafe { self.fallback_allocator.allocate(layout) }
    }

    /// Choose an appropriate block size for the given Layout.alloc
    /// 
    /// Returns an index into the `BLOCK_SIZES` array.
    fn list_index(layout: &Layout) -> Option<usize> {
        let required_block_size = layout.size().max(layout.align());
        BLOCK_SIZES.iter().position(|&size| size >= required_block_size)
    }
}

unsafe impl GlobalAlloc for Locked<FixedSizeBlockAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();
        
        match FixedSizeBlockAllocator::list_index(&layout) {
            Some(index) => {
                match allocator.list_heads[index].take() {
                    Some(node) => {
                        allocator.list_heads[index] = node.next.take();
                        node as *mut ListNode as *mut u8
                    },
                    None => {
                        // No block exists in list => allocate new block
                        let block_size = BLOCK_SIZES[index];
                        // Only works if all block sizes are powers of 2
                        let block_align = block_size;
                        let layout = Layout::from_size_align(block_size, block_align)
                            .unwrap();
                        allocator.fallback_alloc(layout)
                    }
                }
            },
            None => allocator.fallback_alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut allocator = self.lock();

        match FixedSizeBlockAllocator::list_index(&layout) {
            Some(index) => {
                let new_node = ListNode {
                    next: allocator.list_heads[index].take(),
                };

                // Verify that block has size and alignment required for storing node
                assert!(mem::size_of::<ListNode>() <= BLOCK_SIZES[index]);
                assert!(mem::align_of::<ListNode>() <= BLOCK_SIZES[index]);

                let new_node_ptr = ptr as *mut ListNode;
                new_node_ptr.write(new_node);
                allocator.list_heads[index] = Some(&mut *new_node_ptr);
            },
            None => {
                allocator.fallback_allocator.deallocate(ptr, layout);
            }
        }
    }
}
