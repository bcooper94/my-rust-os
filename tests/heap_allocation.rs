#![no_std]
#![no_main]

#![feature(custom_test_frameworks)]
#![test_runner(my_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use alloc::{boxed::Box, vec::Vec};
use my_rust_os::allocator::HEAP_SIZE;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use my_rust_os::allocator;
    use my_rust_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mem_mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mem_mapper, &mut frame_allocator)
        .expect("Heap initialization failed");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_rust_os::test_panic_handler(&info)
}

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(42);
    let heap_value_2 = Box::new(9000);

    assert_eq!(42, *heap_value_1);
    assert_eq!(9000, *heap_value_2);
}

#[test_case]
fn large_vec_many_allocations() {
    let count = 1000;
    let mut vec = Vec::new();

    for i in 0..count {
        vec.push(i);
    }

    for i in 0..count {
        assert_eq!(i, vec[i]);
    }
}

#[test_case]
fn large_vec_one_bulk_allocation() {
    let count = 10000;
    let mut vec = Vec::with_capacity(count);

    for i in 0..count {
        vec.push(i);
    }

    for i in 0..count {
        assert_eq!(i, vec[i]);
    }
}

#[test_case]
fn many_boxes_reuses_memory() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(i, *x);
    }
}

#[test_case]
fn many_boxes_long_lived_reuses_memory() {
    let long_lived = Box::new(1);

    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(i, *x);
    }

    assert_eq!(1, *long_lived);
}

#[test_case]
fn fragment_heap_then_large_alloc() {
    // Fragment heap into roughly quarters of the heap by consuming ~75% of the
    // heap, then allow these segments to be freed
    {
        let quarter_heap = HEAP_SIZE / 4 as usize;
        let _vec: Vec<u8> = Vec::with_capacity(quarter_heap);
        let _second_vec: Vec<u8> = Vec::with_capacity(quarter_heap);
        let _third_vec: Vec<u8> = Vec::with_capacity(quarter_heap);
    }

    // If we've defragmented the heap, we should still be able to allocate
    // a region that is roughly half the size of the heap. Otherwise, we won't
    // find a large enough region and this allocation will fail
    let _vec: Vec<u8> = Vec::with_capacity(HEAP_SIZE / 2 as usize);
}
