//! Embeds the application icon into the Windows executable.

fn main() {
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
