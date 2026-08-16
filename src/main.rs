use std::env::args;
use std::fs;
use std::io::ErrorKind;
use crate::lexer::tokenize;

mod lexer;

fn main() -> std::io::Result<()> {

    let args: Vec<String> = args().skip(1).collect();
    if args.is_empty() {
        println!("Uso: expressa <arquivo>");
        return Ok(());
    }

    let content = fs::read_to_string(&args[0])?;
    let tokens = match tokenize(content.as_str()) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprint!("Error: {:?}", e);
            return Err(std::io::Error::new(ErrorKind::InvalidInput, e.message))
        },
    };

    for tok in tokens.iter() {
        println!("{:?}", tok);
    }

    Ok(())
}
