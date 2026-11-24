use std::env;
use std::path::Path;
use std::process;

fn main() {
    let mut args = env::args().skip(1);
    let Some(arg) = args.next() else {
        eprintln!(
            "Uso: expressa-grafico <arquivo.cart|arquivo.png>\n       expressa-grafico --ao-vivo"
        );
        process::exit(2);
    };
    let r = if arg == "--ao-vivo" {
        expressa::runtime::run_live_viewer()
    } else {
        expressa::runtime::run_viewer(Path::new(&arg))
    };
    if let Err(e) = r {
        eprintln!("{e}");
        process::exit(1);
    }
}
