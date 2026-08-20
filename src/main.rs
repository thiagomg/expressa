use std::env::args;
use std::fs;
use std::io::ErrorKind;
use crate::parser::parse;

mod lexer;
pub mod parser;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = args().skip(1).collect();
    if args.is_empty() {
        println!("Uso: expressa <arquivo.lep>");
        return Ok(());
    }

    let content = fs::read_to_string(&args[0])?;
    match parse(content.as_str()) {
        Ok(program) => {
            println!("{program:#?}");
        }
        Err(e) => {
            eprintln!("Erro: {e}");
            return Err(std::io::Error::new(ErrorKind::InvalidInput, e.message));
        }
    }

    Ok(())
}
