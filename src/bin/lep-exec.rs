use expressa::runtime::{
    ExbRunMode, ExecutionBackend, ExecutionOptions, NumeroAritmetica, NumeroLocale,
    run_exb_file_args_com,
};

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut options = ExecutionOptions {
        backend: ExecutionBackend::Bytecode,
        ..ExecutionOptions::default()
    };
    let mut mode = ExbRunMode::Run;
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("Uso: lep-exec [opções] <arquivo.lepc> [args…]");
                println!("  --numeros pt|en");
                println!("  --aritmetica decimal|float");
                println!("  --marcador-leia");
                #[cfg(feature = "terminal")]
                println!("  debug");
                return Ok(0);
            }
            "--numeros" => {
                index += 1;
                options.numero_locale = args
                    .get(index)
                    .and_then(|name| NumeroLocale::from_name(name))
                    .ok_or("Uso: lep-exec --numeros pt|en <arquivo.lepc> [args…]")?;
            }
            "--aritmetica" => {
                index += 1;
                options.numero_aritmetica = args
                    .get(index)
                    .and_then(|name| NumeroAritmetica::from_name(name))
                    .ok_or("Uso: lep-exec --aritmetica decimal|float <arquivo.lepc> [args…]")?;
            }
            "--marcador-leia" => mode = ExbRunMode::Marker,
            "debug" => {
                if !cfg!(feature = "terminal") {
                    return Err("depurador de terminal indisponível neste build".into());
                }
                mode = ExbRunMode::Debug;
            }
            "--" => {
                index += 1;
                break;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("opção desconhecida: {arg}").into());
            }
            _ => break,
        }
        index += 1;
    }
    let file = args
        .get(index)
        .ok_or("Uso: lep-exec [opções] <arquivo.lepc> [args…]")?;
    if !std::path::Path::new(file)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lepc"))
    {
        return Err("o runner lep-exec exige um arquivo .lepc".into());
    }
    Ok(run_exb_file_args_com(
        file,
        &args[index + 1..],
        mode,
        options,
    )?)
}
