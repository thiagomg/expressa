use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::lexer::Span;

use super::error::EvalError;
use super::eval::Vm;
use super::value::{Value, format_numero};

impl Vm<'_> {
    pub(crate) fn call_builtin(
        &mut self,
        name: &str,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        match name {
            "escreva" => self.bi_escreva(args, span),
            "leia" => self.bi_leia(args, span),
            "raiz" => self.bi_raiz(args, span),
            "tamanho" => self.bi_tamanho(args, span),
            "primeiro" => self.bi_primeiro(args, span),
            "ultimo" => self.bi_ultimo(args, span),
            "maiuscula" => self.bi_maiuscula(args, span),
            "minuscula" => self.bi_minuscula(args, span),
            "substitua" => self.bi_substitua(args, span),
            "separe" => self.bi_separe(args, span),
            "junte" => self.bi_junte(args, span),
            "limpe" => self.bi_limpe(args, span),
            "leia_arquivo" => self.bi_leia_arquivo(args, span),
            "salve_arquivo" => self.bi_salve_arquivo(args, span),
            "adicione_arquivo" => self.bi_adicione_arquivo(args, span),
            "leia_csv" => self.bi_leia_csv(args, span),
            "salve_csv" => self.bi_salve_csv(args, span),
            other => Err(self.err(format!("função nativa desconhecida: {other}"), span)),
        }
    }

    fn bi_escreva(&mut self, args: &[Value], _span: Span) -> Result<Value, EvalError> {
        let mut first = true;
        for arg in args {
            if !first {
                write!(self.out, " ").map_err(|e| self.io_err(e, _span))?;
            }
            first = false;
            write!(self.out, "{arg}").map_err(|e| self.io_err(e, _span))?;
        }
        writeln!(self.out).map_err(|e| self.io_err(e, _span))?;
        self.out.flush().map_err(|e| self.io_err(e, _span))?;
        Ok(Value::Nada)
    }

    fn bi_leia(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() > 1 {
            return Err(self.err(
                format!("leia() espera 0 ou 1 argumento(s), recebeu {}", args.len()),
                span,
            ));
        }
        let prompt = match args.first() {
            Some(v) => self.expect_texto(v, span)?,
            None => String::new(),
        };
        if let Some(host) = self.leia_host.as_mut() {
            return host
                .ask(&prompt)
                .map(Value::Texto)
                .map_err(|e| self.err(e, span));
        }
        if !prompt.is_empty() {
            write!(self.out, "{prompt}").map_err(|e| self.io_err(e, span))?;
            self.out.flush().map_err(|e| self.io_err(e, span))?;
        }
        let mut line = String::new();
        let n = self
            .input
            .read_line(&mut line)
            .map_err(|e| self.io_err(e, span))?;
        if n == 0 {
            return Err(self.err("fim da entrada", span));
        }
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        Ok(Value::Texto(line))
    }

    fn bi_raiz(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_numero(&args[0], span)?;
        if n < 0.0 {
            return Err(self.err("raiz de número negativo", span));
        }
        Ok(Value::Numero(n.sqrt()))
    }

    fn bi_tamanho(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = match &args[0] {
            Value::Texto(s) => s.chars().count() as f64,
            Value::Lista(xs) => xs.borrow().len() as f64,
            Value::Mapa(xs) => xs.borrow().len() as f64,
            other => {
                return Err(self.err(
                    format!(
                        "tamanho() espera texto, lista ou mapa, encontrado {}",
                        other.type_name()
                    ),
                    span,
                ));
            }
        };
        Ok(Value::Numero(n))
    }

    fn bi_primeiro(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let xs = self.expect_lista(&args[0], span)?;
        xs.borrow()
            .first()
            .cloned()
            .ok_or_else(|| self.err("primeiro() de lista vazia", span))
    }

    fn bi_ultimo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let xs = self.expect_lista(&args[0], span)?;
        xs.borrow()
            .last()
            .cloned()
            .ok_or_else(|| self.err("ultimo() de lista vazia", span))
    }

    fn bi_maiuscula(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(s.to_uppercase()))
    }

    fn bi_minuscula(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(s.to_lowercase()))
    }

    fn bi_substitua(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let s = self.expect_texto(&args[0], span)?;
        let from = self.expect_texto(&args[1], span)?;
        let to = self.expect_texto(&args[2], span)?;
        Ok(Value::Texto(s.replace(&from, &to)))
    }

    fn bi_separe(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let s = self.expect_texto(&args[0], span)?;
        let sep = self.expect_texto(&args[1], span)?;
        let parts = if sep.is_empty() {
            s.chars().map(|c| Value::Texto(c.to_string())).collect()
        } else {
            s.split(&sep).map(|p| Value::Texto(p.to_string())).collect()
        };
        Ok(Value::lista(parts))
    }

    fn bi_junte(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let xs = self.expect_lista(&args[0], span)?;
        let sep = self.expect_texto(&args[1], span)?;
        let mut parts = Vec::new();
        for v in xs.borrow().iter() {
            parts.push(value_as_texto(v));
        }
        Ok(Value::Texto(parts.join(&sep)))
    }

    fn bi_limpe(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(s.trim().to_string()))
    }

    fn bi_leia_arquivo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let contents = fs::read_to_string(&path).map_err(|e| {
            self.err(
                format!("não foi possível ler '{}': {e}", path.display()),
                span,
            )
        })?;
        let lines = contents
            .lines()
            .map(|l| Value::Texto(l.to_string()))
            .collect();
        Ok(Value::lista(lines))
    }

    fn bi_salve_arquivo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let linhas = self.expect_lista(&args[1], span)?;
        let body = lines_to_text(&linhas.borrow(), span, self)?;
        self.ensure_parent_dir(&path, span)?;
        fs::write(&path, body).map_err(|e| {
            self.err(
                format!("não foi possível salvar '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }

    fn bi_adicione_arquivo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let linhas = self.expect_lista(&args[1], span)?;
        let extra = lines_to_text(&linhas.borrow(), span, self)?;
        self.ensure_parent_dir(&path, span)?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                self.err(
                    format!("não foi possível abrir '{}': {e}", path.display()),
                    span,
                )
            })?;
        f.write_all(extra.as_bytes())
            .map_err(|e| self.io_err(e, span))?;
        Ok(Value::Nada)
    }

    fn bi_leia_csv(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let contents = fs::read_to_string(&path).map_err(|e| {
            self.err(
                format!("não foi possível ler '{}': {e}", path.display()),
                span,
            )
        })?;
        let rows = contents
            .lines()
            .map(|line| {
                Value::lista(
                    line.split(',')
                        .map(|cell| Value::Texto(cell.to_string()))
                        .collect(),
                )
            })
            .collect();
        Ok(Value::lista(rows))
    }

    fn bi_salve_csv(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let rows = self.expect_lista(&args[1], span)?;
        let mut out = String::new();
        for row in rows.borrow().iter() {
            let cells = self.expect_lista(row, span)?;
            let line: Vec<String> = cells.borrow().iter().map(value_as_texto).collect();
            out.push_str(&line.join(","));
            out.push('\n');
        }
        self.ensure_parent_dir(&path, span)?;
        fs::write(&path, out).map_err(|e| {
            self.err(
                format!("não foi possível salvar '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }

    pub(crate) fn resolve_data_path(
        &self,
        path: &str,
        span: Span,
    ) -> Result<std::path::PathBuf, EvalError> {
        let p = Path::new(path);
        let joined = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.base_dir.join(p)
        };
        self.confine(joined, span)
    }

    fn ensure_parent_dir(&self, path: &Path, span: Span) -> Result<(), EvalError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| self.io_err(e, span))?;
        }
        Ok(())
    }
}

pub(crate) const BUILTINS: &[&str] = &[
    "escreva",
    "leia",
    "raiz",
    "tamanho",
    "primeiro",
    "ultimo",
    "maiuscula",
    "minuscula",
    "substitua",
    "separe",
    "junte",
    "limpe",
    "leia_arquivo",
    "salve_arquivo",
    "adicione_arquivo",
    "leia_csv",
    "salve_csv",
];

fn value_as_texto(v: &Value) -> String {
    match v {
        Value::Texto(s) => s.clone(),
        Value::Numero(n) => format_numero(*n),
        other => other.to_string(),
    }
}

fn lines_to_text(lines: &[Value], span: Span, vm: &Vm<'_>) -> Result<String, EvalError> {
    let mut body = String::new();
    for line in lines {
        match line {
            Value::Texto(s) => {
                body.push_str(s);
                body.push('\n');
            }
            other => {
                return Err(vm.err(
                    format!("esperado lista de textos, encontrado {}", other.type_name()),
                    span,
                ));
            }
        }
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::BUILTINS;
    use crate::runtime::run_to_string;

    fn run(src: &str) -> String {
        run_to_string(src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err(src: &str) -> String {
        match run_to_string(src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("expressa-bi-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run_in(dir: &Path, src: &str) -> String {
        let file = dir.join("teste.lep");
        run_to_string(src, file.to_str().unwrap())
            .unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    #[test]
    fn builtins_are_registered() {
        assert!(BUILTINS.contains(&"escreva"));
        assert!(BUILTINS.contains(&"leia"));
        assert!(BUILTINS.contains(&"raiz"));
        assert_eq!(BUILTINS.len(), 17);
    }

    #[test]
    fn raiz_quadrada() {
        assert_eq!(run(r#"escreva(raiz(0))"#), "0\n");
        assert_eq!(run(r#"escreva(raiz(9))"#), "3\n");
        assert_eq!(run(r#"escreva(raiz(2.25))"#), "1.5\n");
        assert_eq!(run(r#"escreva(raiz(-1) se_falhar 0)"#), "0\n");
        assert!(run_err("raiz(-4)").contains("raiz de número negativo"));
        assert!(run_err(r#"raiz("9")"#).contains("esperado numero"));
    }

    #[test]
    fn escreva_joins_args_with_space() {
        assert_eq!(run(r#"escreva("a", 1, verdadeiro)"#), "a 1 verdadeiro\n");
        assert_eq!(run(r#"escreva()"#), "\n");
    }

    #[test]
    fn tamanho_on_text_list_and_map() {
        assert_eq!(run(r#"escreva(tamanho("olá"))"#), "3\n");
        assert_eq!(run(r#"escreva(tamanho([1, 2, 3]))"#), "3\n");
        assert_eq!(
            run(r#"escreva(tamanho(mapa inicio
    "a" -> 1
    "b" -> 2
fim))"#),
            "2\n"
        );
        assert!(run_err("tamanho(10)").contains("espera texto, lista ou mapa"));
    }

    #[test]
    fn primeiro_e_ultimo() {
        assert_eq!(run(r#"escreva(primeiro([10, 20, 30]))"#), "10\n");
        assert_eq!(run(r#"escreva(ultimo([10, 20, 30]))"#), "30\n");
        assert!(run_err("primeiro([])").contains("lista vazia"));
        assert!(run_err("ultimo([])").contains("lista vazia"));
    }

    #[test]
    fn text_transforms() {
        assert_eq!(run(r#"escreva(maiuscula("olá"))"#), "OLÁ\n");
        assert_eq!(run(r#"escreva(minuscula("Olá"))"#), "olá\n");
        assert_eq!(
            run(r#"escreva(substitua("Maria Silva", "Maria", "Ana"))"#),
            "Ana Silva\n"
        );
        assert_eq!(run(r#"escreva(substitua("aaa", "a", "b"))"#), "bbb\n");
        assert_eq!(run(r#"escreva(limpe("  x  "))"#), "x\n");
    }

    #[test]
    fn separe_and_junte() {
        assert_eq!(
            run(r#"escreva(separe("a,b,c", ","))"#),
            "[\"a\", \"b\", \"c\"]\n"
        );
        assert_eq!(
            run(r#"escreva(separe("a,,b", ","))"#),
            "[\"a\", \"\", \"b\"]\n"
        );
        assert_eq!(
            run(r#"escreva(separe("ola", ""))"#),
            "[\"o\", \"l\", \"a\"]\n"
        );
        assert_eq!(run(r#"escreva(junte(["a", "b"], " - "))"#), "a - b\n");
        assert_eq!(run(r#"escreva(junte([1, 2], ","))"#), "1,2\n");
        assert_eq!(run(r#"escreva(junte([], ","))"#), "\n");
    }

    #[test]
    fn file_roundtrip_relative_to_script_dir() {
        let dir = temp_dir();
        let out = run_in(
            &dir,
            r#"
salve_arquivo("notas.txt", ["7.5", "8"])
adicione_arquivo("notas.txt", ["9"])
linhas = leia_arquivo("notas.txt")
escreva(junte(linhas, "|"))
"#,
        );
        assert_eq!(out, "7.5|8|9\n");
        assert_eq!(
            fs::read_to_string(dir.join("notas.txt")).unwrap(),
            "7.5\n8\n9\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_a_runtime_error() {
        let msg = run_err(r#"leia_arquivo("nao-existe-expressa.txt")"#);
        assert!(msg.contains("não foi possível ler"), "{msg}");
    }

    #[test]
    fn csv_roundtrip() {
        let dir = temp_dir();
        let out = run_in(
            &dir,
            r#"
salve_csv("pessoas.csv", [["Ana", 25], ["Bruno", 30]])
dados = leia_csv("pessoas.csv")
escreva(dados[1][1])
escreva(dados[2][2])
"#,
        );
        assert_eq!(out, "Ana\n30\n");
        assert_eq!(
            fs::read_to_string(dir.join("pessoas.csv")).unwrap(),
            "Ana,25\nBruno,30\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
