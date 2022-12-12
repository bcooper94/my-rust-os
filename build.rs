use std::path::Path;

fn main() {
    // set by cargo, build scripts should use this directory for output files
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    // set by cargo's artifact dependency feature, see
    // https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#artifact-dependencies
    let kernel_base_dir = std::env::var_os("CARGO_BIN_FILE_MY_RUST_OS_my-rust-os").unwrap();
    let kernel_path = Path::new(&kernel_base_dir);

    // create an UEFI disk image (optional)
    let binding = Path::new(&out_dir).join("uefi.img");
    let uefi_path = binding.as_path();
    bootloader::UefiBoot::new(&kernel_path)
        .create_disk_image(uefi_path)
        .unwrap();

    // create a BIOS disk image (optional)
    let binding = Path::new(&out_dir).join("bios.img");
    let bios_path = binding.as_path();
    bootloader::BiosBoot::new(&kernel_path)
        .create_disk_image(bios_path)
        .unwrap();

    // pass the disk image paths as env variables to the `main.rs`
    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}
