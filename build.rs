#[cfg(windows)]
fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let obj_path = format!("{}\\lib\\YY_Thunks_for_Win7.obj", manifest_dir);
    println!("cargo:rustc-link-arg={}", obj_path);
    println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS,6.0");
    println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    println!("cargo:rustc-link-search=native={}\\lib", manifest_dir);
    println!("cargo:rustc-link-lib=dylib=SecureEngineSDK64");
    println!("cargo:rustc-link-lib=delayimp");

    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.set("FileDescription", "Fe PC - e听说答案提取器");
    res.set("ProductName", "Fe PC");
    res.set("OriginalFilename", "fepc.exe");
    res.set("LegalCopyright", "Copyright (C) 2026 code-leitianshuo(leitianshuo1337)");
    res.set("ProductVersion", "v0.1.6");
    res.set("FileVersion", "0.1.6.0");
    if let Err(e) = res.compile() {
        eprintln!("Warning: Failed to compile Windows resource: {}", e);
    }
}

#[cfg(not(windows))]
fn main() {}
