use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Jogo(PathBuf);
impl Jogo {
    fn novo() -> Self {
        static PROXIMO: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "expressa-entrada-{}-{}",
            std::process::id(),
            PROXIMO.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn execute(&self, args: &[&str]) -> std::process::Output {
        std::process::Command::new(env!("CARGO_BIN_EXE_expressa"))
            .env("EXPRESSA_BACKEND", "bytecode")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Jogo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn directory_entry_keeps_arguments_relative_imports_and_file_io() {
    let jogo = Jogo::novo();
    std::fs::write(
        jogo.0.join("principal.lep"),
        r#"
define_opcao(:formato_numero, :en)
modulo = importe "apoio"
a = importe "arquivo"
escreva(modulo::resposta(), argumentos)
escreva(a::leia_arquivo("dados.txt"))
"#,
    )
    .unwrap();
    std::fs::write(jogo.0.join("apoio.lep"), "resposta = funcao() { 42 }").unwrap();
    std::fs::write(jogo.0.join("dados.txt"), "local\n").unwrap();
    std::fs::write(
        jogo.0.join("outro.lep"),
        "falhe(\"não deve ser executado\")",
    )
    .unwrap();
    let out = jogo.execute(&["Ana", "--teste"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "42 [\"Ana\", \"--teste\"]\n[\"local\"]\n"
    );
}

#[test]
fn missing_entry_reports_an_error_without_scanning_other_files() {
    let jogo = Jogo::novo();
    std::fs::write(
        jogo.0.join("outro.lep"),
        "principal = funcao() { escreva(\"errado\") }",
    )
    .unwrap();
    std::fs::write(jogo.0.join("principal.lepc"), "não é um candidato").unwrap();
    let out = jogo.execute(&[]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("não contém principal.lep"));
    assert!(out.stdout.is_empty());
}
