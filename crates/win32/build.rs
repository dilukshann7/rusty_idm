fn main() {
    println!("cargo:rerun-if-changed=resources/app.rc");
    println!("cargo:rerun-if-changed=resources/app.manifest");
    println!("cargo:rerun-if-changed=resources/icons");
    println!("cargo:rerun-if-changed=../../assets/branding/rusty.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/app.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
        // The resource script embeds the single application manifest.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            println!("cargo:rustc-link-arg-bin=rusty-win32=/MANIFEST:NO");
        }
    }
}
