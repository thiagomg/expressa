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
    // Only global options before the command/file select the backend. Script
    // arguments named --backend must reach argumentos unchanged.
    let mut option = 0;
    while option < args.len() {
        match args[option].as_str() {
            "--backend" => {
                let name = args
                    .get(option + 1)
                    .filter(|s| expressa::runtime::ExecutionBackend::from_name(s).is_some())
                    .ok_or("Uso: expressa --backend ast|bytecode [arquivo.lep]")?
                    .clone();
                unsafe { std::env::set_var("EXPRESSA_BACKEND", &name) };
                args.drain(option..=option + 1);
            }
            "--numeros" | "--aritmetica" => option += 2,
            _ => break,
        }
    }
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
    if let Some(i) = args.iter().position(|a| a == "--aritmetica") {
        if i + 1 >= args.len() {
            eprintln!("Uso: expressa --aritmetica decimal|float");
            return Ok(());
        }
        let name = args[i + 1].clone();
        if expressa::runtime::NumeroAritmetica::from_name(&name).is_none() {
            eprintln!("padrão desconhecido `{name}` (use decimal ou float)");
            process::exit(1);
        }
        unsafe { std::env::set_var("EXPRESSA_ARITMETICA", &name) };
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
             expressa <arquivo.lep|arquivo.exb> [args…]\n  \
             expressa debug <arquivo.lep|arquivo.exb> [args…]\n  \
             expressa --ast <arquivo.lep>\n  \
             expressa compilar <arquivo.lep> [-o arquivo.exb]\n  \
             expressa --marcador-leia <arquivo.lep|arquivo.exb> [args…]\n  \
             expressa --grafico <arquivo.cart>\n  \
             expressa --ao-vivo\n  \
             expressa --instalar-exemplos\n  \
             expressa --backend ast|bytecode [arquivo.lep] [args…]\n  \
             expressa --numeros pt|en [arquivo.lep] [args…]\n  \
             expressa --aritmetica decimal|float [arquivo.lep] [args…]"
        );
        return Ok(());
    }
    if args[0] == "--instalar-exemplos" {
        let dest = expressa::exemplos::instalar()?;
        println!("exemplos em {}", dest.display());
        return Ok(());
    }
    if args[0] == "--grafico" {
        if args.len() < 2 {
            eprintln!("Uso: expressa --grafico <arquivo.cart>");
            return Ok(());
        }
        expressa::runtime::run_viewer(std::path::Path::new(&args[1]))?;
        return Ok(());
    }
    if args[0] == "--ao-vivo" {
        expressa::runtime::run_live_viewer()?;
        return Ok(());
    }

    if args[0] == "compilar" {
        if !(args.len() == 2 || (args.len() == 4 && args[2] == "-o")) {
            return Err("Uso: expressa compilar <arquivo.lep> [-o arquivo.exb]".into());
        }
        let input = std::path::Path::new(&args[1]);
        let output = if args.len() == 4 {
            std::path::PathBuf::from(&args[3])
        } else {
            input.with_extension("exb")
        };
        if input == output
            || std::fs::canonicalize(input)
                .ok()
                .zip(std::fs::canonicalize(&output).ok())
                .is_some_and(|(a, b)| a == b)
        {
            return Err("a saída compilada não pode substituir o arquivo fonte".into());
        }
        if !output
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("exb"))
        {
            return Err("a saída compilada deve ter extensão .exb".into());
        }
        use std::io::Read;
        let mut source = String::new();
        std::fs::File::open(input)?
            .take(expressa::runtime::EXB_MAX_BYTES as u64 + 1)
            .read_to_string(&mut source)?;
        let bytes = expressa::runtime::compile_exb(&source, &args[1])?;
        write_exb_output(&output, &bytes)?;
        return Ok(());
    }

    let (mode, file, script_args) = if args[0] == "debug" {
        if args.len() < 2 {
            eprintln!("Uso: expressa debug <arquivo.lep|arquivo.exb> [args…]");
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
            eprintln!("Uso: expressa --marcador-leia <arquivo.lep|arquivo.exb> [args…]");
            return Ok(());
        }
        ("marcador-leia", args[1].as_str(), &args[2..])
    } else {
        ("run", args[0].as_str(), &args[1..])
    };

    if std::path::Path::new(file)
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("exb"))
    {
        let mode = match mode {
            "ast" => {
                return Err(
                    "--ast requer o fonte .lep; o arquivo .exb não armazena uma AST".into(),
                );
            }
            "debug" => expressa::runtime::ExbRunMode::Debug,
            "marcador-leia" => expressa::runtime::ExbRunMode::Marker,
            _ => expressa::runtime::ExbRunMode::Run,
        };
        exit_if_nonzero(expressa::runtime::run_exb_file_args(
            file,
            script_args,
            mode,
        )?)?;
        return Ok(());
    }
    let source = std::fs::read_to_string(file)?;
    match mode {
        "ast" => match parse(&source) {
            Ok(program) => println!("{program:#?}"),
            Err(e) => {
                eprintln!("Erro: {e}");
                process::exit(1);
            }
        },
        "debug" => exit_if_nonzero(debug_source_args(&source, file, script_args)?)?,
        "marcador-leia" => exit_if_nonzero(run_source_marcador_args(&source, file, script_args)?)?,
        _ => exit_if_nonzero(run_source_args(&source, file, script_args)?)?,
    }
    Ok(())
}

fn exit_if_nonzero(code: i32) -> Result<(), Box<dyn std::error::Error>> {
    if code != 0 {
        process::exit(code);
    }
    Ok(())
}

fn write_exb_output(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    struct Temp(std::path::PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("nome de saída inválido"))?;
    for nonce in 0..100 {
        let tmp = path.with_file_name(format!(
            ".expressa-exb.tmp.{}.{}",
            std::process::id(),
            nonce
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(mut file) => {
                let temp = Temp(tmp);
                let written = file.write_all(bytes).and_then(|()| file.sync_all());
                drop(file);
                written?;
                return std::fs::rename(&temp.0, path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other(
        "não foi possível criar saída temporária",
    ))
}
