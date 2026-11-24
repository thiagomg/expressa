//! Any program must finish (or hit the time limit) with a result or a
//! `RuntimeError`, never panic or overflow the stack.
//!
//! Runs with no graphics windows (null hosts), no input, file I/O confined
//! to a temporary folder, and a short time limit.
#![no_main]

use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use expressa::runtime::{LeiaHost, NoopHook, NullCartesiano, NullGraf, run_with_hosts};
use libfuzzer_sys::fuzz_target;

struct SemEntrada;

impl LeiaHost for SemEntrada {
    fn ask(&mut self, _prompt: &str) -> Result<String, String> {
        Err("sem entrada no fuzz".into())
    }
}

fn pasta() -> PathBuf {
    static PASTA: OnceLock<PathBuf> = OnceLock::new();
    PASTA
        .get_or_init(|| {
            let dir = std::env::temp_dir().join(format!("expressa-fuzz-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("pasta do fuzz");
            dir
        })
        .clone()
}

fuzz_target!(|src: &str| {
    let dir = pasta();
    let file = dir.join("fuzz.lep");
    let _ = run_with_hosts(
        src,
        &file.to_string_lossy(),
        Box::new(NoopHook),
        Some(dir),
        Some(Duration::from_millis(200)),
        &mut io::sink(),
        &mut io::sink(),
        &mut SemEntrada,
        Some(&mut NullCartesiano),
        Some(&mut NullGraf),
        &[],
    );
});
