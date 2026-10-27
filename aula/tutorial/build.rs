fn main() {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let icon = manifest.join("../../res/expressa-2.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("windows") {
        return;
    }
    let path = icon.to_str().expect("caminho do icone");
    let mut res = winresource::WindowsResource::new();
    res.set_icon(path);
    if let Err(err) = res.compile() {
        panic!("icone do executavel: {err}");
    }
}
