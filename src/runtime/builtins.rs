use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::lexer::Span;

use super::error::EvalError;
use super::eval::Vm;
use super::value::{NumeroLocale, Value, format_numero};

impl Vm<'_> {
    pub(crate) fn call_builtin(
        &mut self,
        name: &str,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        match name {
            "escreva" => self.bi_escreva(args, span),
            "escreva_erro" => self.bi_escreva_erro(args, span),
            "sair" => self.bi_sair(args, span),
            "leia" => self.bi_leia(args, span),
            "leia_linhas" => self.bi_leia_linhas(args, span),
            "raiz" => self.bi_raiz(args, span),
            "transposta" => self.bi_transposta(args, span),
            "det" => self.bi_det(args, span),
            "identidade" => self.bi_identidade(args, span),
            "numero" => self.bi_numero(args, span),
            "formato" => self.bi_formato(args, span),
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
            write!(self.out, "{}", arg.format_with(self.numero_locale))
                .map_err(|e| self.io_err(e, _span))?;
        }
        writeln!(self.out).map_err(|e| self.io_err(e, _span))?;
        self.out.flush().map_err(|e| self.io_err(e, _span))?;
        Ok(Value::Nada)
    }

    fn bi_escreva_erro(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let mut first = true;
        for arg in args {
            if !first {
                write!(self.err, " ").map_err(|e| self.io_err(e, span))?;
            }
            first = false;
            write!(self.err, "{}", arg.format_with(self.numero_locale))
                .map_err(|e| self.io_err(e, span))?;
        }
        writeln!(self.err).map_err(|e| self.io_err(e, span))?;
        self.err.flush().map_err(|e| self.io_err(e, span))?;
        Ok(Value::Nada)
    }

    fn bi_sair(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let code = match args {
            [] => 0,
            [v] => {
                let n = self.expect_int(v, span)?;
                i32::try_from(n).map_err(|_| self.err("código de saída fora do intervalo", span))?
            }
            _ => {
                return Err(self.err(
                    format!("sair() espera 0 ou 1 argumento(s), recebeu {}", args.len()),
                    span,
                ));
            }
        };
        let _ = self.out.flush();
        let _ = self.err.flush();
        Err(EvalError::Quit(code))
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
        super::leia::strip_newline(&mut line);
        Ok(Value::Texto(line))
    }

    fn bi_leia_linhas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if !args.is_empty() {
            return Err(self.err(
                format!(
                    "leia_linhas() não espera argumentos, recebeu {}",
                    args.len()
                ),
                span,
            ));
        }
        if let Some(host) = self.leia_host.as_mut() {
            let lines = host.read_all_lines().map_err(|e| self.err(e, span))?;
            return Ok(Value::lista(lines.into_iter().map(Value::Texto).collect()));
        }
        let mut lines = Vec::new();
        loop {
            self.check_deadline(span)?;
            let mut line = String::new();
            let n = self
                .input
                .read_line(&mut line)
                .map_err(|e| self.io_err(e, span))?;
            if n == 0 {
                break;
            }
            super::leia::strip_newline(&mut line);
            lines.push(Value::Texto(line));
        }
        Ok(Value::lista(lines))
    }

    fn bi_numero(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        match &args[0] {
            Value::Numero(n) => Ok(Value::Numero(*n)),
            Value::Texto(s) => match self.numero_locale.parse_texto(s) {
                Some(n) => Ok(Value::Numero(n)),
                None => Err(self.err(
                    format!("não foi possível transformar `{s}` em número"),
                    span,
                )),
            },
            other => Err(self.err(
                format!(
                    "numero() espera texto ou numero, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }

    fn bi_formato(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let name = self.expect_texto(&args[0], span)?;
        match NumeroLocale::from_name(&name) {
            Some(loc) => {
                self.numero_locale = loc;
                Ok(Value::Nada)
            }
            None => Err(self.err(
                format!("padrão desconhecido `{name}` (use \"pt\" ou \"en\")"),
                span,
            )),
        }
    }

    fn bi_raiz(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_numero(&args[0], span)?;
        if n < 0.0 {
            return Err(self.err("raiz de número negativo", span));
        }
        Ok(Value::Numero(n.sqrt()))
    }

    fn expect_matriz(
        &self,
        v: &Value,
        span: Span,
    ) -> Result<std::rc::Rc<std::cell::RefCell<Vec<Vec<f64>>>>, EvalError> {
        match v {
            Value::Matriz(m) => Ok(std::rc::Rc::clone(m)),
            _ => Err(self.err(
                format!("esperado matriz, encontrado {}", v.type_name()),
                span,
            )),
        }
    }

    fn bi_transposta(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        let rows = m.borrow();
        let h = rows.len();
        let w = rows.first().map(|r| r.len()).unwrap_or(0);
        let mut out = vec![vec![0.0; h]; w];
        for i in 0..h {
            for j in 0..w {
                out[j][i] = rows[i][j];
            }
        }
        Ok(Value::matriz(out))
    }

    fn bi_identidade(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_int(&args[0], span)?;
        if n < 1 {
            return Err(self.err("identidade() espera um inteiro >= 1", span));
        }
        let n = n as usize;
        let mut out = vec![vec![0.0; n]; n];
        for i in 0..n {
            out[i][i] = 1.0;
        }
        Ok(Value::matriz(out))
    }

    fn bi_det(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        let a = m.borrow();
        let n = a.len();
        if n == 0 || a.iter().any(|r| r.len() != n) {
            return Err(self.err("det() exige matriz quadrada", span));
        }
        let d = match n {
            1 => a[0][0],
            2 => a[0][0] * a[1][1] - a[0][1] * a[1][0],
            3 => {
                a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                    - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                    + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
            }
            _ => {
                return Err(self.err("det() só para matrizes 1×1, 2×2 ou 3×3", span));
            }
        };
        Ok(Value::Numero(d))
    }

    fn bi_tamanho(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = match &args[0] {
            Value::Texto(s) => s.chars().count() as f64,
            Value::Lista(xs) => xs.borrow().len() as f64,
            Value::Mapa(xs) => xs.borrow().len() as f64,
            Value::Matriz(m) => m.borrow().len() as f64,
            other => {
                return Err(self.err(
                    format!(
                        "tamanho() espera texto, lista, mapa ou matriz, encontrado {}",
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
            parts.push(value_as_texto(v, self.numero_locale));
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

    fn ensure_parent_dir(&self, path: &Path, span: Span) -> Result<(), EvalError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| self.io_err(e, span))?;
        }
        Ok(())
    }
}

pub(crate) const BUILTINS: &[&str] = &[
    "escreva",
    "escreva_erro",
    "sair",
    "leia",
    "leia_linhas",
    "numero",
    "formato",
    "raiz",
    "transposta",
    "det",
    "identidade",
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

pub(crate) struct BuiltinDoc {
    pub name: &'static str,
    pub sig: &'static str,
    pub summary: &'static str,
    pub example: &'static str,
}

pub(crate) const BUILTIN_DOCS: &[BuiltinDoc] = &[
    BuiltinDoc {
        name: "escreva",
        sig: "escreva(valor, ...)",
        summary: "Imprime os argumentos separados por espaço e quebra a linha.",
        example: r#"escreva("Olá", 10)"#,
    },
    BuiltinDoc {
        name: "escreva_erro",
        sig: "escreva_erro(valor, ...)",
        summary: "Como escreva, mas na saída de erro (stderr). Não vai para arquivos com >.",
        example: r#"escreva_erro("nome vazio")"#,
    },
    BuiltinDoc {
        name: "sair",
        sig: "sair()  ou  sair(codigo)",
        summary: "Encerra o programa. Sem argumento, código 0; sair(1) indica falha.",
        example: r#"se nome == "" { sair(1) }"#,
    },
    BuiltinDoc {
        name: "leia",
        sig: "leia()  ou  leia(prompt)",
        summary: "Lê uma linha do teclado (sem o Enter). Prompt opcional.",
        example: r#"nome = leia("Seu nome:")"#,
    },
    BuiltinDoc {
        name: "leia_linhas",
        sig: "leia_linhas() -> lista",
        summary: "Lê todas as linhas da entrada padrão (pipe/arquivo) até o fim.",
        example: r#"para linha em leia_linhas() { escreva(linha) }"#,
    },
    BuiltinDoc {
        name: "numero",
        sig: "numero(texto|numero) -> numero",
        summary: "Transforma texto em número (aceita 3.14 ou 3,14). Erro se não for número.",
        example: r#"idade = numero(leia("Idade: ")) se_falhar 0"#,
    },
    BuiltinDoc {
        name: "formato",
        sig: r#"formato("pt")  ou  formato("en")"#,
        summary: "Escolhe o padrão de texto de números: pt-BR (1.000,5) ou en-US (1,000.5).",
        example: r#"formato("en")"#,
    },
    BuiltinDoc {
        name: "transposta",
        sig: "transposta(matriz) -> matriz",
        summary: "Troca linhas por colunas.",
        example: "transposta(A)",
    },
    BuiltinDoc {
        name: "det",
        sig: "det(matriz) -> numero",
        summary: "Determinante (1×1, 2×2 ou 3×3).",
        example: "det(A)",
    },
    BuiltinDoc {
        name: "identidade",
        sig: "identidade(n) -> matriz",
        summary: "Matriz identidade n×n.",
        example: "identidade(3)",
    },
    BuiltinDoc {
        name: "raiz",
        sig: "raiz(numero) -> numero",
        summary: "Raiz quadrada. Erro se o número for negativo (use se_falhar).",
        example: "raiz(9)    // 3",
    },
    BuiltinDoc {
        name: "tamanho",
        sig: "tamanho(texto|lista|mapa) -> numero",
        summary: "Quantidade de caracteres, itens ou pares.",
        example: r#"tamanho("olá")    // 3"#,
    },
    BuiltinDoc {
        name: "primeiro",
        sig: "primeiro(lista) -> valor",
        summary: "Primeiro elemento da lista (índice 1). Erro se vazia.",
        example: "primeiro([10, 20])    // 10",
    },
    BuiltinDoc {
        name: "ultimo",
        sig: "ultimo(lista) -> valor",
        summary: "Último elemento da lista. Erro se vazia.",
        example: "ultimo([10, 20])    // 20",
    },
    BuiltinDoc {
        name: "maiuscula",
        sig: "maiuscula(texto) -> texto",
        summary: "Copia o texto em letras maiúsculas.",
        example: r#"maiuscula("olá")    // "OLÁ""#,
    },
    BuiltinDoc {
        name: "minuscula",
        sig: "minuscula(texto) -> texto",
        summary: "Copia o texto em letras minúsculas.",
        example: r#"minuscula("Olá")    // "olá""#,
    },
    BuiltinDoc {
        name: "substitua",
        sig: "substitua(texto, antigo, novo) -> texto",
        summary: "Troca todas as ocorrências de antigo por novo.",
        example: r#"substitua("aa", "a", "b")    // "bb""#,
    },
    BuiltinDoc {
        name: "separe",
        sig: "separe(texto, separador) -> lista",
        summary: "Parte o texto. Separador \"\" gera um item por caractere.",
        example: r#"separe("a,b", ",")    // ["a", "b"]"#,
    },
    BuiltinDoc {
        name: "junte",
        sig: "junte(lista, separador) -> texto",
        summary: "Junta os itens da lista com o separador no meio.",
        example: r#"junte(["a", "b"], "-")    // "a-b""#,
    },
    BuiltinDoc {
        name: "limpe",
        sig: "limpe(texto) -> texto",
        summary: "Remove espaços do começo e do fim.",
        example: r#"limpe("  x  ")    // "x""#,
    },
    BuiltinDoc {
        name: "leia_arquivo",
        sig: "leia_arquivo(caminho) -> lista",
        summary: "Lê o arquivo; cada linha vira um texto (sem \\n).",
        example: r#"leia_arquivo("dados.txt") se_falhar []"#,
    },
    BuiltinDoc {
        name: "salve_arquivo",
        sig: "salve_arquivo(caminho, linhas)",
        summary: "Grava a lista de textos, substituindo o arquivo.",
        example: r#"salve_arquivo("saida.txt", ["a", "b"])"#,
    },
    BuiltinDoc {
        name: "adicione_arquivo",
        sig: "adicione_arquivo(caminho, linhas)",
        summary: "Acrescenta linhas no fim do arquivo (cria se não existir).",
        example: r#"adicione_arquivo("saida.txt", ["c"])"#,
    },
    BuiltinDoc {
        name: "leia_csv",
        sig: "leia_csv(caminho) -> lista",
        summary: "Lê CSV simples (vírgula, sem aspas). Lista de listas de textos.",
        example: r#"leia_csv("notas.csv")"#,
    },
    BuiltinDoc {
        name: "salve_csv",
        sig: "salve_csv(caminho, dados)",
        summary: "Grava uma lista de listas como CSV.",
        example: r#"salve_csv("saida.csv", [["Ana", 25]])"#,
    },
];

pub(crate) fn lookup_builtin_doc(name: &str) -> Option<&'static BuiltinDoc> {
    BUILTIN_DOCS.iter().find(|d| d.name == name)
}

fn value_as_texto(v: &Value, loc: NumeroLocale) -> String {
    match v {
        Value::Texto(s) => s.clone(),
        Value::Numero(n) => loc.format(*n),
        other => other.to_string(),
    }
}

fn csv_cell(v: &Value) -> String {
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

    use super::{BUILTIN_DOCS, BUILTINS, lookup_builtin_doc};
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
        assert!(BUILTINS.contains(&"leia_linhas"));
        assert!(BUILTINS.contains(&"raiz"));
        assert!(BUILTINS.contains(&"numero"));
        assert_eq!(BUILTINS.len(), 25);
        assert_eq!(BUILTIN_DOCS.len(), BUILTINS.len());
        for name in BUILTINS {
            assert!(
                lookup_builtin_doc(name).is_some(),
                "falta ajuda para {name}"
            );
        }
    }

    #[test]
    fn numero_from_text() {
        assert_eq!(run(r#"escreva(numero("10"))"#), "10\n");
        assert_eq!(run(r#"escreva(numero("  3,14  "))"#), "3,14\n");
        assert_eq!(run(r#"escreva(numero("1_000"))"#), "1.000\n");
        assert_eq!(run(r#"escreva(numero(-8))"#), "-8\n");
        assert_eq!(run(r#"escreva(numero("abc") se_falhar 0)"#), "0\n");
        assert!(run_err(r#"numero("xyz")"#).contains("transformar"));
        assert_eq!(run(r#"escreva(numero("1.000"))"#), "1.000\n");
        assert_eq!(
            run(r#"formato("en")
escreva(numero("1,000.5"))"#),
            "1,000.5\n"
        );
    }

    #[test]
    fn raiz_quadrada() {
        assert_eq!(run(r#"escreva(raiz(0))"#), "0\n");
        assert_eq!(run(r#"escreva(raiz(9))"#), "3\n");
        assert_eq!(run(r#"escreva(raiz(2.25))"#), "1,5\n");
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
    fn sair_rejects_bad_args() {
        assert!(run_err(r#"sair("x")"#).contains("esperado numero"));
        assert!(run_err("sair(1, 2)").contains("espera 0 ou 1"));
        assert!(run_err("sair(1.5)").contains("inteiro"));
    }

    #[test]
    fn leia_linhas_rejects_args() {
        assert!(run_err(r#"leia_linhas(1)"#).contains("não espera argumentos"));
    }

    #[test]
    fn escreva_erro_goes_to_stderr() {
        use crate::runtime::run_to_strings_with;
        let (out, err) = run_to_strings_with(
            r#"escreva("ok")
escreva_erro("falhou")"#,
            "t.lep",
            "",
        )
        .unwrap();
        assert_eq!(out, "ok\n");
        assert_eq!(err, "falhou\n");
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
        assert!(run_err("tamanho(10)").contains("espera texto, lista, mapa ou matriz"));
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
