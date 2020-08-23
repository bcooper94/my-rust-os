#![no_std]

#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(llvm_asm)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate rlibc;
extern crate alloc;

use core::panic::PanicInfo;
use alloc::alloc::Layout;

pub mod serial;
pub mod vga_buffer;
pub mod qemu;
pub mod interrupts;
pub mod gdt;
pub mod memory;
pub mod allocator;

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());

    for test in tests {
        test.run();
    }

    qemu::exit_qemu(qemu::QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    qemu::exit_qemu(qemu::QemuExitCode::Failed);
    hlt_loop();
}

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

/// Loop over a HLT instruction to use less power while waiting for the next
/// interrupt
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[alloc_error_handler]
fn handle_alloc_error(layout: Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[cfg(test)]
use bootloader::{BootInfo, entry_point};

#[cfg(test)]
entry_point!(test_kernel_main);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
    init();
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
