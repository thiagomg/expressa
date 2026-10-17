use std::env;
use std::path::Path;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Uso: expressa-grafico <arquivo.cart>");
        process::exit(2);
    };
    if let Err(e) = expressa::runtime::run_viewer(Path::new(&path)) {
        eprintln!("{e}");
        process::exit(1);
    }
}
