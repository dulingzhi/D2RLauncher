fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=kernel32");
    }
    tauri_build::build()
}
