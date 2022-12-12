#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
// #![feature(llvm_asm)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
// #![feature(const_fn)]
// #![feature(const_in_array_repeat_expressions)]
// #![feature(wake_trait)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate rlibc;

use alloc::alloc::Layout;
use bootloader_api::{config::Mapping, BootInfo, BootloaderConfig};
use bootloader_x86_64_common::logger::LockedLogger;
use core::panic::PanicInfo;
use log::{Level, Log, Record, RecordBuilder};
use x86_64::structures::paging::frame;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

pub mod allocator;
pub mod gdt;
pub mod interrupts;
// pub mod memory;
pub mod qemu;
pub mod serial;
pub mod task;
pub mod vga_buffer;

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

pub fn init(boot_info: &'static mut BootInfo) {
    gdt::init();
    vga_buffer::init(boot_info.framebuffer.as_mut().unwrap());
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();

    // init_heap(&boot_info);
}

// fn init_heap(boot_info: &'static BootInfo) {
//     use memory::BootInfoFrameAllocator;
//     use x86_64::VirtAddr;

//     let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
//     let mut mem_mapper = unsafe { memory::init(phys_mem_offset) };
//     let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
//     allocator::init_heap(&mut mem_mapper, &mut frame_allocator)
//         .expect("Heap initialization failed");
// }

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
use bootloader_api::entry_point;

#[cfg(test)]
entry_point!(test_kernel_main, config = &BOOTLOADER_CONFIG);

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(boot_info: &'static mut BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}
