use std::env::args;
use std::process;

use expressa::parser::parse;
#[cfg(feature = "terminal")]
use expressa::runtime::run_repl_com;
use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, NumeroAritmetica, NumeroLocale, debug_source_args_com,
    run_exb_file_args_com, run_source_args_com, run_source_marcador_args_com,
};

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = args().skip(1).collect();
    // Flags choose this process only. `execute` must not inherit them: a child
    // expressa keeps its own defaults unless the script sets the environment.
    let mut options = ExecutionOptions::default();
    let option = 0;
    while option < args.len() {
        match args[option].as_str() {
            "--backend" => {
                let name = args
                    .get(option + 1)
                    .cloned()
                    .ok_or("Uso: expressa --backend ast|bytecode [arquivo.lep]")?;
                let backend = ExecutionBackend::from_name(&name)
                    .ok_or("Uso: expressa --backend ast|bytecode [arquivo.lep]")?;
                if !backend.is_available() {
                    return Err("backend AST indisponível neste build".into());
                }
                options.backend = backend;
                args.drain(option..=option + 1);
            }
            "--numeros" => {
                let name = args
                    .get(option + 1)
                    .cloned()
                    .ok_or("Uso: expressa --numeros pt|en")?;
                options.numero_locale = NumeroLocale::from_name(&name)
                    .ok_or_else(|| format!("padrão desconhecido `{name}` (use pt ou en)"))?;
                args.drain(option..=option + 1);
            }
            "--aritmetica" => {
                let name = args
                    .get(option + 1)
                    .cloned()
                    .ok_or("Uso: expressa --aritmetica decimal|float")?;
                options.numero_aritmetica =
                    NumeroAritmetica::from_name(&name).ok_or_else(|| {
                        format!("padrão desconhecido `{name}` (use decimal ou float)")
                    })?;
                args.drain(option..=option + 1);
            }
            _ => break,
        }
    }
    if args.is_empty() {
        if !options.backend.is_available() {
            return Err("backend AST indisponível neste build".into());
        }
        #[cfg(feature = "terminal")]
        run_repl_com(options)?;
        #[cfg(not(feature = "terminal"))]
        return Err("REPL indisponível neste build; informe um arquivo .lep ou .exb".into());
        #[cfg(feature = "terminal")]
        return Ok(());
    }
    if args[0] == "--help" || args[0] == "-h" {
        eprintln!("Uso:");
        #[cfg(feature = "terminal")]
        eprintln!("  expressa                         REPL");
        eprintln!("  expressa <arquivo.lep|arquivo.exb> [args…]");
        #[cfg(feature = "terminal")]
        eprintln!("  expressa debug <arquivo.lep|arquivo.exb> [args…]");
        eprintln!("  expressa --ast <arquivo.lep>     imprime a árvore sintática");
        eprintln!("  expressa compilar <arquivo.lep> [-o arquivo.exb]");
        eprintln!("  expressa --marcador-leia <arquivo.lep|arquivo.exb> [args…]");
        #[cfg(feature = "graphics")]
        eprintln!("  expressa --grafico <arquivo.cart>\n  expressa --ao-vivo");
        eprintln!("  expressa --instalar-exemplos");
        let backends = if cfg!(feature = "ast") {
            "ast|bytecode"
        } else {
            "bytecode"
        };
        eprintln!("  expressa --backend {backends} [arquivo.lep] [args…]");
        eprintln!("  expressa --numeros pt|en [arquivo.lep] [args…]");
        eprintln!("  expressa --aritmetica decimal|float [arquivo.lep] [args…]");
        return Ok(());
    }
    if args[0] == "--instalar-exemplos" {
        let dest = expressa::exemplos::instalar()?;
        println!("exemplos em {}", dest.display());
        return Ok(());
    }
    if args[0] == "--grafico" {
        if args.len() < 2 {
            return Err("Uso: expressa --grafico <arquivo.cart>".into());
        }
        #[cfg(feature = "graphics")]
        expressa::runtime::run_viewer(std::path::Path::new(&args[1]))?;
        #[cfg(not(feature = "graphics"))]
        return Err("gráficos indisponíveis neste build".into());
        #[cfg(feature = "graphics")]
        return Ok(());
    }
    if args[0] == "--ao-vivo" {
        #[cfg(feature = "graphics")]
        expressa::runtime::run_live_viewer()?;
        #[cfg(not(feature = "graphics"))]
        return Err("gráficos indisponíveis neste build".into());
        #[cfg(feature = "graphics")]
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
        if !cfg!(feature = "terminal") {
            return Err("depurador de terminal indisponível neste build".into());
        }
        if args.len() < 2 {
            return Err("Uso: expressa debug <arquivo.lep|arquivo.exb> [args…]".into());
        }
        ("debug", args[1].as_str(), &args[2..])
    } else if args[0] == "--ast" {
        if args.len() < 2 {
            return Err("Uso: expressa --ast <arquivo.lep>".into());
        }
        ("ast", args[1].as_str(), &args[2..])
    } else if args[0] == "--marcador-leia" {
        if args.len() < 2 {
            return Err("Uso: expressa --marcador-leia <arquivo.lep|arquivo.exb> [args…]".into());
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
        exit_if_nonzero(run_exb_file_args_com(file, script_args, mode, options)?)?;
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
        "debug" => exit_if_nonzero(debug_source_args_com(&source, file, script_args, options)?)?,
        "marcador-leia" => exit_if_nonzero(run_source_marcador_args_com(
            &source,
            file,
            script_args,
            options,
        )?)?,
        _ => exit_if_nonzero(run_source_args_com(&source, file, script_args, options)?)?,
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
