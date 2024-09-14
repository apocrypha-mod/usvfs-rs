use std::env;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from(env::current_dir().unwrap());
    println!(
        "cargo:rustc-link-search=native={}",
        path.as_os_str().to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=usvfs_x64");
}
