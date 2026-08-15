//! Embeds assets/icon.ico into the Windows executable as icon resource 1,
//! which Explorer shows for the file and main.rs loads for the window and
//! taskbar. Regenerate the .ico after editing assets/icon.svg:
//!
//! ```sh
//! for s in 16 20 24 32 48 64 128 256; do
//!   inkscape -w $s -h $s assets/icon.svg -o icon-$s.png
//! done
//! magick icon-{16,20,24,32,48,64,128,256}.png assets/icon.ico
//! ```

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}
