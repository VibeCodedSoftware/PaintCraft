// Embeds the PaintCraft cheetah icon and version info into the Windows executable.
fn main() {
    println!("cargo:rerun-if-changed=assets/branding/paintcraft.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/branding/paintcraft.ico");
        res.set("ProductName", "PaintCraft");
        res.set("FileDescription", "PaintCraft - 3D texture painting studio");
        res.set("LegalCopyright", "PaintCraft contributors; engine by Ras-Maciej (3D Paintcraft). MIT OR Apache-2.0");
        if let Err(err) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon: {err}");
        }
    }
}
