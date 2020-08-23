#![no_std]
#![no_main]

#![feature(llvm_asm)]
#![feature(custom_test_frameworks)]
#![test_runner(my_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use my_rust_os::println;
use bootloader::{BootInfo, entry_point};

extern crate rlibc;
extern crate alloc;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    my_rust_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_rust_os::test_panic_handler(info)
}

#[allow(dead_code)]
fn divide_by_zero() {
    unsafe {
        llvm_asm!("mov dx, 0; div dx" ::: "ax", "dx" : "volatile", "intel")
    }
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use my_rust_os::memory::{self, BootInfoFrameAllocator};
    use my_rust_os::allocator;
    use x86_64::VirtAddr;
    use alloc::{rc::Rc, vec, vec::Vec, boxed::Box};

    my_rust_os::init();

    println!("Hello world{}", "!");

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mem_mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mem_mapper, &mut frame_allocator)
        .expect("Heap initialization failed");

    let test_box = Box::new("testing 1234");
    println!("Box at {:p}", test_box);

    let mut vector = Vec::new();
    for i in 0..500 {
        vector.push(i);
    }
    println!("Vec at {:p}", vector.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("current reference count is {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("reference count is {} now", Rc::strong_count(&cloned_reference));

    #[cfg(test)]
    test_main();

    println!("It didn't crash");

    my_rust_os::hlt_loop();
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn simple_test() {
        assert_eq!(1, 1);
    }
}

