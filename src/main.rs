use std::env::args;
use std::process;

use expressa::parser::parse;
use expressa::runtime::{debug_source, run_source};

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "Uso:\n  \
             expressa <arquivo.lep>\n  \
             expressa debug <arquivo.lep>\n  \
             expressa --ast <arquivo.lep>"
        );
        return Ok(());
    }

    let (mode, file) = if args[0] == "debug" {
        if args.len() < 2 {
            eprintln!("Uso: expressa debug <arquivo.lep>");
            return Ok(());
        }
        ("debug", args[1].as_str())
    } else if args[0] == "--ast" {
        if args.len() < 2 {
            eprintln!("Uso: expressa --ast <arquivo.lep>");
            return Ok(());
        }
        ("ast", args[1].as_str())
    } else {
        ("run", args[0].as_str())
    };

    let source = std::fs::read_to_string(file)?;
    match mode {
        "ast" => match parse(&source) {
            Ok(program) => println!("{program:#?}"),
            Err(e) => {
                eprintln!("Erro: {e}");
                process::exit(1);
            }
        },
        "debug" => debug_source(&source, file)?,
        _ => run_source(&source, file)?,
    }
    Ok(())
}
