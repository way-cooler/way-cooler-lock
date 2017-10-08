extern crate gcc;
extern crate wayland_scanner;

use wayland_scanner::{Side, generate_code, generate_interfaces};
use gcc::Config;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    add_pam_build_flags();
    generate_wayland_protocols();
}

fn add_pam_build_flags() {
    let mut config = Config::new();
    config.flag("-Wall");
    config.flag("-Wpedantic");
    config.flag("-Werror");
    config.file("src/pam/wrapper.c");
    config.compile("libpamwrapper.a");

    // Link against libpam
    println!("cargo:rustc-flags=-l pam")
}

fn generate_wayland_protocols() {
    let protocols = fs::read_dir("./protocols")
        .expect("No <Way Cooler>/protocols/ directory");
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    for protocol_path in protocols {
        let protocol_path: fs::DirEntry = protocol_path.unwrap();
        let path: PathBuf = protocol_path.path().into();
        let mut file_name: String = protocol_path.file_name().into_string().unwrap();
        if let Some(extension) = file_name.find(".xml") {
            file_name.truncate(extension);
        }
        generate_code(
            path.clone(),
            out_dir.join(file_name.clone() + "_api.rs"),
            Side::Client
        );
        generate_interfaces(
            path,
            out_dir.join(file_name + "_interface.rs")
        );
    }
}
