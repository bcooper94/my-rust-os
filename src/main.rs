#![no_std]
#![no_main]

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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Hello world{}", "!");

    #[cfg(test)]
    test_main();

    loop {}
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn simple_test() {
        assert_eq!(1, 1);
    }
}

