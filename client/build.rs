//! Embeds the application icon into the Windows executable, and lets the macOS binary find the
//! Swift runtime that ScreenCaptureKit's bridge links against.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // A dependency's link arguments do not reach the final binary, so the rpath goes here.
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    println!("cargo:rerun-if-changed=../packaging/icon/moqspeak.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../packaging/icon/moqspeak.ico");
        res.set("ProductName", "moqspeak");
        res.set("FileDescription", "moqspeak");
        res.compile().expect("compiling the Windows resources");
    }
}
