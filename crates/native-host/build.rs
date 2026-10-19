fn main() {
    println!("cargo:rerun-if-changed=resources/host.rc");
    println!("cargo:rerun-if-changed=../../assets/branding/rusty.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources/host.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
