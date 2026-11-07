fn main() {
    let src = std::path::Path::new("src");
    let parser = src.join("parser.c");
    println!("cargo:rerun-if-changed={}", parser.display());
    cc::Build::new()
        .std("c11")
        .include(src)
        .file(&parser)
        .compile("tree-sitter-expressa");
}
