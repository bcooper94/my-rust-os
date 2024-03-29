#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(my_rust_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use my_rust_os::{hlt_loop, println};

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();

    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    my_rust_os::test_panic_handler(info)
}

#[test_case]
fn test_println() {
    println!("Hello world!");
}
