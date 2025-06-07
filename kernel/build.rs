use std::{env, fs, io::Result, path::Path};

const LOGLEVEL: &str = "error";

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");
    gen_linker_script()
}

fn gen_linker_script() -> Result<()> {
    let arch = env::var("CARGO_CFG_TARGET_ARCH")
        .expect("Failed to get CARGO_CFG_TARGET_ARCH environment variable.");

    let (output_arch_str, kernel_base_addr) = if arch == "x86_64" {
        ("i386:x86-64", "0xffff800000200000")
    } else if arch.starts_with("riscv64") {
        ("riscv", "0xffffffc080200000")
    } else if arch == "aarch64" {
        ("aarch64", "0xffff000040080000")
    } else if arch == "loongarch64" {
        ("loongarch64", "0x9000000080000000")
    } else {
        panic!("Unsupported target architecture: {}", arch);
    };

    let linker_script_name = format!("linker_{}.lds", arch);

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker_template_path = Path::new(&manifest_dir).join("linker.ld");

    let ld_content_template = fs::read_to_string(&linker_template_path).map_err(|e| {
        eprintln!("Failed to read linker script template at {:?}: {}", linker_template_path, e);
        e
    })?;

    let final_ld_content = ld_content_template
        .replace("%ARCH%", output_arch_str)
        .replace("%KERNEL_BASE%", kernel_base_addr);

    let out_dir = env::var("OUT_DIR").expect("Failed to get OUT_DIR environment variable.");
    let final_script_path = Path::new(&out_dir).join(linker_script_name);

    fs::write(&final_script_path, final_ld_content).map_err(|e| {
        eprintln!("Failed to write final linker script to {:?}: {}", final_script_path, e);
        e
    })?;

    println!("cargo:rustc-link-arg=-T{}", final_script_path.display());
    // println!("cargo:rustc-link-arg=-nostartfiles"); // Temporarily commented out for diagnostics

    // The LOGLEVEL constant's purpose was unclear in the original script for build-time config.
    // If it's for conditional compilation in the main kernel code based on log level,
    // it should be set via `rustc-cfg`.
    // Example: println!("cargo:rustc-cfg=kernel_loglevel=\"{}\"", LOGLEVEL);

    Ok(())
}