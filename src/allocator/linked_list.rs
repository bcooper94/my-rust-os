use super::{align_up, Locked};
use alloc::alloc::{GlobalAlloc, Layout};
use core::{mem, ptr};

#[derive(Debug)]
struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode {
            size,
            next: None,
        }
    }

    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }

    /// Merge this ListNode with `self.next`, setting `self.next` to point to
    /// the following ListNode.
    /// 
    /// Panics if combining the size of this ListNode with `self.next` results
    /// in an integer overflow.
    fn merge_with_next(&mut self) {
        let next = self.next.as_mut().unwrap();
        self.size = self.size.checked_add(next.size)
            .expect("Overflow while merging ListNode with next ListNode");
        self.next = next.next.take();
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    /// Creates an empty LinkedListAllocator
    pub const fn new() -> Self {
        LinkedListAllocator {
            head: ListNode::new(0),
        }
    }

    /// Initialize the allocator with the given heap bounds.
    ///
    /// This function is unsafe because the caller must guarantee that the given
    /// heap bounds are valid and that the heap is unused. This method must be
    /// called only once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }

    pub unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
        let (size, align) = Self::size_align(layout);

        if let Some((region, alloc_start)) = self.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            // region is larger than needed: split region up into a used and a
            // free segment, and add free segment to the free list
            if excess_size > 0 {
                self.add_free_region(alloc_end, excess_size);
            }

            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let (size, _) = Self::size_align(layout);
        self.add_free_region(ptr as usize, size);
    }

    /// Adds the given memory region to the free list.
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // ensure that the freed memory region is capable of holding ListNode
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        let mut node = ListNode::new(size);
        let node_ptr = addr as *mut ListNode;

        let mut prev_region = self.find_region_preceding_addr(addr);

        // prev_region is just before addr, and prev_region.next is immediately
        // after addr, so insert node in between prev_region and prev_region.next
        if prev_region.next.is_some() {
            node.next = prev_region.next.take();
        }

        // prev_region.next should always point to the new free node
        prev_region.next = Some(&mut *node_ptr);
        node_ptr.write(node);

        let mut was_prev_region_merged = false;
        if prev_region.size > 0 {
            was_prev_region_merged = Self::try_merge_region_with_next(
                &mut prev_region
            );
        }

        // If prev_region was merged with prev_region.next, we need to try to
        // merge prev_region with prev_region.next again
        if was_prev_region_merged {
            Self::try_merge_region_with_next(&mut prev_region);
        } else {
            Self::try_merge_region_with_next(prev_region.next.as_mut().unwrap());
        }
    }

    /// Find the last region that starts just before the given address
    fn find_region_preceding_addr(&mut self, addr: usize) -> &mut ListNode {
        let mut current = &mut self.head;

        // Find the last region that starts just before addr
        while let Some(ref mut next_region) = current.next {
            if next_region.start_addr() < addr {
                current = current.next.as_mut().unwrap();
            } else {
                break;
            }
        }

        current
    }

    /// Try to merge the given `ListNode` with the next region in the free list
    /// Returns `true` if `region` is merged with `region.next`, and `false`
    /// otherwise.
    fn try_merge_region_with_next(region: &mut ListNode) -> bool {
        let region_end_addr = region.end_addr();

        if let Some(ref mut next_region) = region.next {
            if region_end_addr == next_region.start_addr() {
                region.merge_with_next();
                return true;
            }
        }

        false
    }

    /// Looks for a free region of the given size and alignment, and removes it
    /// from the list.
    /// 
    /// Returns a tuple of the list node and the start address of the allocation.
    fn find_region(&mut self, size: usize, align: usize)
        -> Option<(&'static mut ListNode, usize)>
    {
        let mut current = &mut self.head;

        // look for a large enough memory region in the linked list
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // region suitable for allocation -> remove node from list
                let next = region.next.take();
                let returned_region = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return returned_region;
            } else {
                // region not suitable -> look at next region
                current = current.next.as_mut().unwrap();
            }
        }

        // no suitable region found
        None
    }

    /// Try to use the given region for an allocation with a given size and
    /// alignment.
    /// 
    /// Returns the allocation start address on success.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize)
        -> Result<usize, ()>
    {
        let alloc_start = align_up(region.start_addr(), align);

        let bytes_lost = alloc_start - region.start_addr();
        if bytes_lost > 0 {
            crate::serial_println!("{} bytes lost in allocation", bytes_lost);
        }

        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // region too small
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // rest of the region is too small to fit another ListNode, which
            // is required because the allocation splits the region into a used
            // and a free part
            return Err(());
        }

        // suitable region for allocation
        Ok(alloc_start)
    }

    /// Adjust the given layout so that the resulting allocated memory region
    /// is also capable of storing a `ListNode`.
    /// 
    /// Returns the adjusted size and layout as a `(size, layout)` tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout.align_to(mem::size_of::<ListNode>())
            .expect("alignment adjustment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().allocate(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().deallocate(ptr, layout);
    }
}

/*
TODO: figure out how to get these tests working with a LinkedListAllocator

#[cfg(test)]
mod tests {
    use crate::allocator;
    use alloc::vec::Vec;

    #[test_case]
    fn first_free_region_is_full_heap() {
        let head = &allocator::ALLOCATOR.lock().head;
        assert_eq!(0, head.size);

        let next = head.next.as_ref().unwrap();
        assert_eq!(allocator::HEAP_SIZE, next.size);
    }

    #[test_case]
    fn free_list_is_sorted_by_address() {
        let vec: Vec<i32> = Vec::with_capacity(1000);
        let _vec_2: Vec<i32> = Vec::with_capacity(1000);
        let vec_3: Vec<i32> = Vec::with_capacity(100);
        let _vec_4: Vec<i32> = Vec::with_capacity(10000);
        let vec_5: Vec<i32> = Vec::with_capacity(100);

        // Drop vectors in different order from allocation order so we can check
        // that free list still maintains nodes in ascending order by address
        drop(vec_3);
        drop(vec);
        drop(vec_5);

        let allocator = allocator::ALLOCATOR.lock();

        let mut prev_node = &allocator.head;
        let mut prev_end_addr = prev_node.end_addr();
        let mut region_count = 0;

        while let Some(ref region) = prev_node.next {
            region_count += 1;

            assert!(
                region.end_addr() > prev_end_addr,
                "Current region ending at {:x} expected to be greater than region ending at {:x}",
                region.end_addr(),
                prev_end_addr
            );
            prev_node = &region;
            prev_end_addr = prev_node.end_addr();
        }

        // 3 free regions = 1 region per dropped vector above
        assert!(region_count >= 3, "Expected to inspect at least 3 free regions");
    }

    #[test_case]
    fn multiple_freed_allocs_are_merged() {
        let vec: Vec<i32> = Vec::with_capacity(1000);
        let vec_2: Vec<i32> = Vec::with_capacity(1000);
        let vec_3: Vec<i32> = Vec::with_capacity(100);
        let vec_4: Vec<i32> = Vec::with_capacity(10000);
        let vec_5: Vec<i32> = Vec::with_capacity(100);

        // Drop vectors in different order from allocation order so we can check
        // that free list still maintains nodes in ascending order by address
        drop(vec_3);
        drop(vec);
        drop(vec_5);
        drop(vec_2);
        drop(vec_4);

        // TODO: this may not be reliable if we've allocated memory during test setup
        assert_eq!(1, count_free_regions());
    }

    fn count_free_regions() -> usize {
        let mut region_count = 0;
        // TODO: this doesn't work with different allocators
        let mut prev_region = &allocator::ALLOCATOR.lock().head;
        while let Some(ref region) = prev_region.next {
            region_count += 1;
            prev_region = region;
        }

        region_count
    }
}
*/
