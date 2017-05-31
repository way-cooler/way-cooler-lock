extern crate gcc;

use gcc::Config;

fn main() {
    let mut config = Config::new();
    config.flag("-Wall");
    config.flag("-Wpedantic");
    config.flag("-Werror");
    config.file("src/pam/wrapper.c");
    config.compile("libpamwrapper.a");

    // Link against libpam
    println!("cargo:rustc-flags=-l pam")
}
