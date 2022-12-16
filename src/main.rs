#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use my_rust_os::println;
use my_rust_os::task::{executor::Executor, keyboard, Task};

extern crate alloc;
extern crate rlibc;

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
    unsafe { core::arch::asm!("mov dx, 0; div dx", out("ax") _, out("dx") _) }
}

async fn example() -> usize {
    42
}

async fn call_example() {
    let num = example().await;
    println!("async number: {}", num);
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};

    my_rust_os::init(&boot_info);

    println!("Hello world{}", "!");

    let test_box = Box::new("testing 1234");
    println!("Box at {:p}", test_box);

    let mut vector = Vec::new();
    for i in 0..500 {
        vector.push(i);
    }
    println!("Vec at {:p}", vector.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    );
    core::mem::drop(reference_counted);
    println!(
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    );

    let mut executor = Executor::new();
    executor.spawn(Task::new(call_example()));
    executor.spawn(Task::new(keyboard::print_keypresses()));

    #[cfg(not(test))]
    executor.run();

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
