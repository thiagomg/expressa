use std::env::args;
use std::process;

use expressa::parser::parse;
use expressa::runtime::{debug_source_args, run_repl, run_source_args, run_source_marcador_args};

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--numeros") {
        if i + 1 >= args.len() {
            eprintln!("Uso: expressa --numeros pt|en");
            return Ok(());
        }
        let name = args[i + 1].clone();
        if expressa::runtime::NumeroLocale::from_name(&name).is_none() {
            eprintln!("padrão desconhecido `{name}` (use pt ou en)");
            process::exit(1);
        }
        unsafe { std::env::set_var("EXPRESSA_NUMEROS", &name) };
        args.drain(i..=i + 1);
    }
    if args.is_empty() {
        run_repl()?;
        return Ok(());
    }
    if args[0] == "--help" || args[0] == "-h" {
        eprintln!(
            "Uso:\n  \
             expressa                         REPL\n  \
             expressa <arquivo.lep> [args…]\n  \
             expressa debug <arquivo.lep> [args…]\n  \
             expressa --ast <arquivo.lep>\n  \
             expressa --marcador-leia <arquivo.lep> [args…]\n  \
             expressa --numeros pt|en [arquivo.lep] [args…]"
        );
        return Ok(());
    }

    let (mode, file, script_args) = if args[0] == "debug" {
        if args.len() < 2 {
            eprintln!("Uso: expressa debug <arquivo.lep> [args…]");
            return Ok(());
        }
        ("debug", args[1].as_str(), &args[2..])
    } else if args[0] == "--ast" {
        if args.len() < 2 {
            eprintln!("Uso: expressa --ast <arquivo.lep>");
            return Ok(());
        }
        ("ast", args[1].as_str(), &args[2..])
    } else if args[0] == "--marcador-leia" {
        if args.len() < 2 {
            eprintln!("Uso: expressa --marcador-leia <arquivo.lep> [args…]");
            return Ok(());
        }
        ("marcador-leia", args[1].as_str(), &args[2..])
    } else {
        ("run", args[0].as_str(), &args[1..])
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
        "debug" => debug_source_args(&source, file, script_args)?,
        "marcador-leia" => run_source_marcador_args(&source, file, script_args)?,
        _ => run_source_args(&source, file, script_args)?,
    }
    Ok(())
}
