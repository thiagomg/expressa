use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;
use super::{csv_cell, lines_to_text};

impl Vm<'_, '_> {
    pub(crate) fn bi_leia_arquivo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_salve_arquivo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_adicione_arquivo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_leia_csv(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_salve_csv(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let rows = self.expect_lista(&args[1], span)?;
        let mut out = String::new();
        for row in rows.borrow().iter() {
            let cells = self.expect_lista(row, span)?;
            let line: Vec<String> = cells.borrow().iter().map(csv_cell).collect();
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
    pub(crate) fn ensure_parent_dir(&self, path: &Path, span: Span) -> Result<(), EvalError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| self.io_err(e, span))?;
        }
        Ok(())
    }
}

/// Files and folders (`existe`, `liste`, `mova`…) and path text
/// (`junte_caminho`, `extensao`…). Paths go through `resolve_data_path`:
/// relative to the `.lep`, and confined in the Aula.
impl Vm<'_, '_> {
    fn caminho_arg(&self, v: &Value, span: Span) -> Result<PathBuf, EvalError> {
        self.resolve_data_path(&self.expect_texto(v, span)?, span)
    }

    pub(crate) fn bi_existe(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        Ok(Value::Bool(self.caminho_arg(&args[0], span)?.exists()))
    }

    pub(crate) fn bi_eh_pasta(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        Ok(Value::Bool(self.caminho_arg(&args[0], span)?.is_dir()))
    }

    /// Names (not paths) in the folder, sorted; `padrao` keeps those that
    /// match `*` / `?`.
    pub(crate) fn bi_liste(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.is_empty() || args.len() > 2 {
            return Err(self.err(
                format!("liste espera 1 ou 2 argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        let pasta = self.caminho_arg(&args[0], span)?;
        let padrao = match args.get(1) {
            Some(v) => Some(self.expect_texto(v, span)?.chars().collect::<Vec<_>>()),
            None => None,
        };
        let entradas = fs::read_dir(&pasta).map_err(|e| {
            self.err(
                format!("não foi possível listar '{}': {e}", pasta.display()),
                span,
            )
        })?;
        let mut nomes = Vec::new();
        for e in entradas {
            let e = e.map_err(|e| self.io_err(e, span))?;
            let nome = e.file_name().to_string_lossy().into_owned();
            let chars: Vec<char> = nome.chars().collect();
            if padrao.as_ref().is_none_or(|p| casa_padrao(p, &chars)) {
                nomes.push(nome);
            }
        }
        nomes.sort();
        Ok(Value::lista(nomes.into_iter().map(Value::Texto).collect()))
    }

    pub(crate) fn bi_crie_pasta(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let p = self.caminho_arg(&args[0], span)?;
        fs::create_dir_all(&p).map_err(|e| {
            self.err(
                format!("não foi possível criar a pasta '{}': {e}", p.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }

    pub(crate) fn bi_apague_arquivo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let p = self.caminho_arg(&args[0], span)?;
        if p.is_dir() {
            return Err(self.err(
                format!("'{}' é uma pasta; use apague_pasta", p.display()),
                span,
            ));
        }
        fs::remove_file(&p).map_err(|e| {
            self.err(
                format!("não foi possível apagar '{}': {e}", p.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }

    /// The folder and everything inside it. Refuses `/` and the home folder.
    pub(crate) fn bi_apague_pasta(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let p = self.caminho_arg(&args[0], span)?;
        if !p.is_dir() {
            return Err(self.err(
                format!("'{}' não é uma pasta; use apague_arquivo", p.display()),
                span,
            ));
        }
        let real = fs::canonicalize(&p).map_err(|e| self.io_err(e, span))?;
        let casa = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .and_then(|h| fs::canonicalize(h).ok());
        if real.parent().is_none() || Some(&real) == casa.as_ref() {
            return Err(self.err(format!("apague_pasta não apaga '{}'", real.display()), span));
        }
        fs::remove_dir_all(&p).map_err(|e| {
            self.err(
                format!("não foi possível apagar '{}': {e}", p.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }

    /// Like `mv`: into `destino` when it is a folder. Works across disks.
    pub(crate) fn bi_mova(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let (origem, destino) = self.origem_destino(args, span)?;
        if let Err(e) = fs::rename(&origem, &destino) {
            let copiou = e.kind() == io::ErrorKind::CrossesDevices
                && origem.is_file()
                && fs::copy(&origem, &destino).is_ok()
                && fs::remove_file(&origem).is_ok();
            if !copiou {
                return Err(self.err(
                    format!(
                        "não foi possível mover '{}' para '{}': {e}",
                        origem.display(),
                        destino.display()
                    ),
                    span,
                ));
            }
        }
        Ok(Value::Nada)
    }

    /// Copies a file (not folders); into `destino` when it is a folder.
    pub(crate) fn bi_copie(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let (origem, destino) = self.origem_destino(args, span)?;
        if origem.is_dir() {
            return Err(self.err(
                format!(
                    "copie só copia arquivos; '{}' é uma pasta",
                    origem.display()
                ),
                span,
            ));
        }
        fs::copy(&origem, &destino).map_err(|e| {
            self.err(
                format!(
                    "não foi possível copiar '{}' para '{}': {e}",
                    origem.display(),
                    destino.display()
                ),
                span,
            )
        })?;
        Ok(Value::Nada)
    }

    fn origem_destino(&self, args: &[Value], span: Span) -> Result<(PathBuf, PathBuf), EvalError> {
        let origem = self.caminho_arg(&args[0], span)?;
        let mut destino = self.caminho_arg(&args[1], span)?;
        if destino.is_dir()
            && let Some(nome) = origem.file_name()
        {
            destino.push(nome);
        }
        Ok((origem, destino))
    }

    pub(crate) fn bi_junte_caminho(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(self.err("junte_caminho espera pelo menos um texto", span));
        }
        let mut p = PathBuf::new();
        for a in args {
            p.push(self.expect_texto(a, span)?);
        }
        Ok(Value::Texto(p.to_string_lossy().into_owned()))
    }

    pub(crate) fn bi_nome_do_arquivo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let t = self.expect_texto(&args[0], span)?;
        Ok(texto_de(Path::new(&t).file_name()))
    }

    pub(crate) fn bi_pasta_de(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let t = self.expect_texto(&args[0], span)?;
        Ok(texto_de(Path::new(&t).parent().map(Path::as_os_str)))
    }

    pub(crate) fn bi_extensao(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let t = self.expect_texto(&args[0], span)?;
        Ok(texto_de(Path::new(&t).extension()))
    }

    /// Folder `expressa` was started from (relative paths use the `.lep`'s).
    pub(crate) fn bi_pasta_atual(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        let p = std::env::current_dir().map_err(|e| self.io_err(e, span))?;
        Ok(Value::Texto(p.to_string_lossy().into_owned()))
    }
}

fn texto_de(s: Option<&std::ffi::OsStr>) -> Value {
    Value::Texto(
        s.map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    )
}

/// `*` any run of characters, `?` exactly one; everything else literal.
fn casa_padrao(p: &[char], nome: &[char]) -> bool {
    let (mut i, mut j) = (0, 0);
    let mut estrela: Option<(usize, usize)> = None;
    while j < nome.len() {
        if i < p.len() && (p[i] == '?' || p[i] == nome[j]) {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == '*' {
            estrela = Some((i, j));
            i += 1;
        } else if let Some((ei, ej)) = estrela {
            i = ei + 1;
            j = ej + 1;
            estrela = Some((ei, ej + 1));
        } else {
            return false;
        }
    }
    p[i..].iter().all(|&c| c == '*')
}

#[cfg(test)]
mod tests {
    use super::casa_padrao;
    use crate::runtime::run_to_string;

    #[test]
    fn pastas_e_arquivos() {
        let dir = std::env::temp_dir().join(format!("expressa-fs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = "importe \"arquivo\"\n\
            crie_pasta(\"t/a/b\")\n\
            salve_arquivo(\"t/a/x.txt\", [\"oi\"])\n\
            salve_arquivo(\"t/a/y.md\", [\"oi\"])\n\
            escreva(existe(\"t/a/x.txt\"), existe(\"t/nada\"), eh_pasta(\"t/a\"), é_pasta(\"t/a/x.txt\"))\n\
            escreva(liste(\"t/a\"), liste(\"t/a\", \"*.txt\"))\n\
            copie(\"t/a/x.txt\", \"t/a/b\")\n\
            copie(\"t/a/x.txt\", \"t/a/c.txt\")\n\
            mova(\"t/a/y.md\", \"t/z.md\")\n\
            escreva(liste(\"t/a/b\"), leia_arquivo(\"t/a/c.txt\"), existe(\"t/z.md\"), existe(\"t/a/y.md\"))\n\
            escreva(apague_arquivo(\"t/a\") se_falhar \"é pasta\", apague_pasta(\"t/z.md\") se_falhar \"é arquivo\")\n\
            escreva(copie(\"t/a\", \"t/q\") se_falhar \"copie pasta\", liste(\"t/nada\") se_falhar \"sem pasta\")\n\
            apague_arquivo(\"t/z.md\")\n\
            apague_pasta(\"t\")\n\
            escreva(existe(\"t\"), apague_pasta(\"/\") se_falhar \"raiz não\")\n";
        let out = run_to_string(src, &dir.join("t.lep").to_string_lossy()).unwrap();
        assert_eq!(
            out,
            "verdadeiro falso verdadeiro falso\n\
             [\"b\", \"x.txt\", \"y.md\"] [\"x.txt\"]\n\
             [\"x.txt\"] [\"oi\"] verdadeiro falso\n\
             é pasta é arquivo\n\
             copie pasta sem pasta\n\
             falso raiz não\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn caminhos_em_texto() {
        let out = run_to_string(
            "importe \"arquivo\"\n\
             escreva(nome_do_arquivo(\"x/y/z.png\"), pasta_de(\"x/y/z.png\"), \
             pasta_de(\"z.png\") == \"\", extensao(\"a.tar.gz\"), extensão(\"sem\") == \"\")\n\
             escreva(nome_do_arquivo(junte_caminho(\"a\", \"b\", \"c.txt\")), \
             pasta_atual() != \"\")",
            "t.lep",
        )
        .unwrap();
        assert_eq!(
            out,
            "z.png x/y verdadeiro gz verdadeiro\nc.txt verdadeiro\n"
        );
    }

    #[test]
    fn pasta_confinada_vale_para_as_novas() {
        let dir = std::env::temp_dir().join(format!("expressa-fsc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "existe(\"/etc\")",
            "liste(\"..\")",
            "apague_pasta(\"../x\")",
            "mova(\"a\", \"/tmp/b\")",
        ] {
            let e = crate::runtime::run_to_string_with(
                &format!("importe \"arquivo\"\n{f}"),
                &dir.join("t.lep").to_string_lossy(),
                "",
                Some(dir.clone()),
                None,
            )
            .unwrap_err();
            assert!(e.message.contains("fora da pasta"), "{f}: {}", e.message);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn casa(p: &str, n: &str) -> bool {
        casa_padrao(
            &p.chars().collect::<Vec<_>>(),
            &n.chars().collect::<Vec<_>>(),
        )
    }

    #[test]
    fn padroes() {
        assert!(casa("*.txt", "a.txt"));
        assert!(casa("*.txt", ".txt"));
        assert!(!casa("*.txt", "a.txt.bak"));
        assert!(casa("a?c", "abc"));
        assert!(!casa("a?c", "ac"));
        assert!(casa("*", ""));
        assert!(casa("*a*b*", "xxaYYbzz"));
        assert!(!casa("*a*b", "xxaYYbzz"));
        assert!(casa("relatório-*.csv", "relatório-2024.csv"));
        assert!(casa("abc", "abc"));
        assert!(!casa("abc", "abd"));
    }
}
