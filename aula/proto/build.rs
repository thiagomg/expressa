fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = tonic_build::Config::new();
    println!("cargo:rerun-if-env-changed=PROTOC");
    if cfg!(windows) && std::env::var_os("PROTOC").is_none() {
        let bundled = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../deps/protoc/bin/protoc.exe");
        println!("cargo:rerun-if-changed={}", bundled.display());
        if bundled.is_file() {
            config.protoc_executable(bundled);
        }
    }
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos_with_config(config, &["proto/aula.proto"], &["proto"])?;
    Ok(())
}
