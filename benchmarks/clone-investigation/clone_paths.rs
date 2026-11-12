//! Diagnostic host microbenchmark, not an interpreter workload. Black-box the
//! enum input to exercise dynamic dispatch in Value::clone, including drops.
use expressa::runtime::Value;
use std::hint::black_box;

fn main() {
    let case = std::env::args().nth(1).expect("case file");
    let case = std::path::Path::new(&case).file_stem().unwrap().to_str().unwrap();
    let (value, repeats) = match case {
        "number" => (Value::Numero(7i64.into()), 10_000_000),
        "list" => (Value::lista(vec![Value::Numero(7i64.into())]), 10_000_000),
        "text" => (Value::Texto("um pequeno texto para copiar".into()), 3_000_000),
        _ => panic!("unknown case"),
    };
    for _ in 0..repeats {
        black_box(black_box(&value).clone());
    }
    println!("verdadeiro");
}
