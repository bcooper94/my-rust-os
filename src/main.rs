#![no_std]
#![no_main]

#![feature(llvm_asm)]
#![feature(custom_test_frameworks)]
#![test_runner(my_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use my_rust_os::println;

extern crate rlibc;

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    my_rust_os::init();

    println!("Hello world{}", "!");

    #[cfg(test)]
    test_main();

    println!("It didn't crash");

    loop {}
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn simple_test() {
        assert_eq!(1, 1);
    }
}

