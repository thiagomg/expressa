use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::lexer::Span;

use super::super::env::Env;
use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::numero::Numero;
use super::super::processo::{EtapaFuncao, Fluxo, Opcoes, Processo, Recebido, separe_comando};
use super::super::value::{MapKey, Value, format_numero};

/// How long one wait for the children lasts before the time limit and
/// Parar are checked again.
const FATIA: Duration = Duration::from_millis(50);

enum Etapa {
    Comandos(Vec<Vec<String>>),
    Funcao(EtapaFuncao),
}

impl Vm<'_, '_> {
    /// `execute(comando, etapa…, opções?)`: waits, returns
    /// `{:codigo, :ok, :saida, :erro}`. A non-zero code is not an error.
    pub(crate) fn bi_execute(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        let p = self.inicie_processo("execute", args, span, false)?;
        let mut saida = Vec::new();
        while let Some(l) = self.processo_proxima(&p, span, env)? {
            saida.push(Value::Texto(l));
        }
        let mut p = p.borrow_mut();
        let codigo = p.codigo.unwrap_or(0);
        let erro = std::mem::take(&mut p.erros)
            .into_iter()
            .map(Value::Texto)
            .collect();
        Ok(Value::mapa(vec![
            (
                chave("codigo"),
                Value::Numero(Numero::from_i64(codigo.into())),
            ),
            (chave("ok"), Value::Bool(codigo == 0)),
            (chave("saida"), Value::lista(saida)),
            (chave("erro"), Value::lista(erro)),
        ]))
    }

    /// `linhas_de(comando, etapa…, opções?)`: the running command, for
    /// `para linha em …`. stderr goes to `escreva_erro` as it arrives.
    pub(crate) fn bi_linhas_de(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let p = self.inicie_processo("linhas_de", args, span, true)?;
        Ok(Value::Processo(p))
    }

    /// Next line that passed every stage; `None` when the commands ended.
    pub(crate) fn processo_proxima(
        &mut self,
        p: &Rc<RefCell<Processo>>,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Option<String>, EvalError> {
        loop {
            {
                let mut pb = p.borrow_mut();
                if let Some(l) = pb.saida.pop_front() {
                    return Ok(Some(l));
                }
                if pb.codigo.is_some() {
                    return Ok(None);
                }
            }
            self.check_espera(span)?;
            let estourou = p.borrow().limite.is_some_and(|t| Instant::now() >= t);
            if estourou {
                let mut pb = p.borrow_mut();
                pb.encerre();
                return Err(self.err(
                    format!("tempo esgotado (:tempo) em '{}'", pb.descricao),
                    span,
                ));
            }
            // The borrow ends before any Expressa function runs: a stage
            // may look at this same value.
            let recebido = p.borrow_mut().fluxo.receba(FATIA);
            match recebido {
                Recebido::Nada => {}
                Recebido::Acabou => {
                    let mut pb = p.borrow_mut();
                    let codigo = pb.fluxo.codigo();
                    pb.codigo = Some(codigo);
                    if pb.exija && codigo != 0 {
                        return Err(self.err(falhou(&pb, codigo), span));
                    }
                }
                Recebido::Erro(l) => {
                    if p.borrow().erro_ao_vivo {
                        writeln!(self.err, "{l}").map_err(|e| self.io_err(e, span))?;
                    } else {
                        p.borrow_mut().erros.push(l);
                    }
                }
                Recebido::Linha(g, l) => {
                    let etapas = p.borrow().funcoes[g].clone();
                    let linhas = self.aplique_etapas(&etapas, l, span, env)?;
                    let mut pb = p.borrow_mut();
                    if g + 1 < pb.fluxo.grupos() {
                        for l in linhas {
                            pb.fluxo.envie(g + 1, l);
                        }
                    } else {
                        pb.saida.extend(linhas);
                    }
                }
            }
        }
    }

    fn inicie_processo(
        &mut self,
        nome: &str,
        args: &[Value],
        span: Span,
        erro_ao_vivo: bool,
    ) -> Result<Rc<RefCell<Processo>>, EvalError> {
        if !self.processos_permitidos() {
            return Err(self.err(
                format!("{nome}: programas externos não rodam aqui (só na linha de comando)"),
                span,
            ));
        }
        let (etapas, opcoes) = match args.split_last() {
            Some((Value::Mapa(_), resto)) => (resto, args.last()),
            _ => (args, None),
        };
        if etapas.is_empty() {
            return Err(self.err(format!("{nome}() espera um comando"), span));
        }
        let mut grupos: Vec<Vec<Vec<String>>> = Vec::new();
        let mut funcoes: Vec<Vec<EtapaFuncao>> = Vec::new();
        let mut descricao: Vec<String> = Vec::new();
        for (i, v) in etapas.iter().enumerate() {
            match self.etapa_de(v, i + 1, span)? {
                Etapa::Comandos(cmds) => {
                    descricao.extend(cmds.iter().map(|c| c.join(" ")));
                    // Commands next to each other share one OS pipe chain.
                    if funcoes.last().is_none_or(|f| !f.is_empty()) {
                        grupos.push(Vec::new());
                        funcoes.push(Vec::new());
                    }
                    grupos.last_mut().unwrap().extend(cmds);
                }
                Etapa::Funcao(f) => {
                    let Some(depois_de) = funcoes.last_mut() else {
                        return Err(self.err(
                            format!(
                                "{nome}: o primeiro argumento tem que ser um comando \
                                 (texto ou lista de textos), não uma função"
                            ),
                            span,
                        ));
                    };
                    descricao.push(f.nome.clone());
                    depois_de.push(f);
                }
            }
        }
        let opcoes = match opcoes {
            Some(m) => self.opcoes_de(m, span)?,
            None => Opcoes::default(),
        };
        if opcoes.terminal {
            let problema = if erro_ao_vivo {
                Some(format!("{nome} não aceita :terminal; use execute"))
            } else if funcoes.iter().any(|f| !f.is_empty()) {
                Some("com :terminal as etapas só podem ser comandos".to_string())
            } else if opcoes.entrada.is_some() {
                Some("com :terminal a entrada é o teclado; tire :entrada".to_string())
            } else {
                None
            };
            if let Some(p) = problema {
                return Err(self.err(p, span));
            }
            // What the program printed must come before the command's output.
            self.out.flush().map_err(|e| self.io_err(e, span))?;
            self.err.flush().map_err(|e| self.io_err(e, span))?;
        }
        let fluxo = Fluxo::inicie(grupos, &opcoes).map_err(|e| self.err(e, span))?;
        Ok(Rc::new(RefCell::new(Processo {
            descricao: descricao.join(" | "),
            fluxo,
            funcoes,
            saida: VecDeque::new(),
            erros: Vec::new(),
            erro_ao_vivo,
            limite: opcoes.tempo.map(|d| Instant::now() + d),
            codigo: None,
            exija: opcoes.exija,
        })))
    }

    /// Argument `n` (1-based): command text, argv list, function, or
    /// `[função, extras…]`.
    fn etapa_de(&self, v: &Value, n: usize, span: Span) -> Result<Etapa, EvalError> {
        let etapa = self.etapa_de_valor(v, n, span)?;
        // Checked up front: the plain arity error would not mention the line.
        // A function first is refused by the caller, with a better message.
        if n > 1
            && let Etapa::Funcao(EtapaFuncao {
                f: Value::Funcao(c),
                extras,
                ..
            }) = &etapa
            && c.params.len() != 1 + extras.len()
        {
            return Err(self.err(
                format!(
                    "etapa {n}: a função tem {} parâmetro(s), mas recebe a linha e mais {} \
                     argumento(s); passe os outros numa lista: [funcao, …]",
                    c.params.len(),
                    extras.len()
                ),
                span,
            ));
        }
        Ok(etapa)
    }

    fn etapa_de_valor(&self, v: &Value, n: usize, span: Span) -> Result<Etapa, EvalError> {
        match v {
            Value::Texto(t) => separe_comando(t)
                .map(Etapa::Comandos)
                .map_err(|e| self.err(e, span)),
            Value::Funcao(_) | Value::Builtin(_) => Ok(Etapa::Funcao(EtapaFuncao {
                f: v.clone(),
                extras: Vec::new(),
                nome: nome_da_etapa(v, n),
            })),
            Value::Lista(xs) => {
                let xs = xs.borrow();
                match xs.first() {
                    None => Err(self.err(format!("etapa {n}: lista vazia não é comando"), span)),
                    Some(f @ (Value::Funcao(_) | Value::Builtin(_))) => {
                        Ok(Etapa::Funcao(EtapaFuncao {
                            f: f.clone(),
                            extras: xs[1..].to_vec(),
                            nome: nome_da_etapa(f, n),
                        }))
                    }
                    Some(_) => {
                        let argv = xs
                            .iter()
                            .map(|x| match x {
                                Value::Texto(s) => Ok(s.clone()),
                                Value::Numero(num) => Ok(format_numero(*num)),
                                other => Err(self.err(
                                    format!(
                                        "etapa {n}: a lista do comando só pode ter textos, \
                                         encontrado {}",
                                        other.type_name()
                                    ),
                                    span,
                                )),
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        if argv[0].is_empty() {
                            return Err(self.err(format!("etapa {n}: comando vazio"), span));
                        }
                        Ok(Etapa::Comandos(vec![argv]))
                    }
                }
            }
            Value::Mapa(_) => {
                Err(self.err("o mapa de opções tem que ser o último argumento", span))
            }
            other => Err(self.err(
                format!(
                    "etapa {n}: esperado comando (texto ou lista) ou função, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }

    fn opcoes_de(&self, m: &Value, span: Span) -> Result<Opcoes, EvalError> {
        let Value::Mapa(m) = m else {
            unreachable!("only maps are options");
        };
        let mut opcoes = Opcoes::default();
        for (k, v) in m.borrow().iter() {
            let chave = match k {
                MapKey::Texto(s) => s.as_str(),
                other => {
                    return Err(self.err(format!("opção desconhecida: {other}"), span));
                }
            };
            match chave {
                "pasta" => {
                    let pasta = self.resolve_data_path(&self.expect_texto(v, span)?, span)?;
                    if !pasta.is_dir() {
                        return Err(
                            self.err(format!("pasta não existe: {}", pasta.display()), span)
                        );
                    }
                    opcoes.pasta = Some(pasta);
                }
                "ambiente" => {
                    let Value::Mapa(vars) = v else {
                        return Err(self.err(
                            format!(":ambiente espera um mapa, encontrado {}", v.type_name()),
                            span,
                        ));
                    };
                    for (k, v) in vars.borrow().iter() {
                        let nome = match k {
                            MapKey::Texto(s) => s.clone(),
                            other => other.to_string(),
                        };
                        let valor = self.texto_de_linha(v, ":ambiente", span)?;
                        opcoes.ambiente.push((nome, valor));
                    }
                }
                "tempo" => {
                    let s = self.expect_f64(v, span)?;
                    let d = Duration::try_from_secs_f64(s)
                        .ok()
                        .filter(|d| !d.is_zero())
                        .ok_or_else(|| self.err(":tempo espera segundos > 0", span))?;
                    opcoes.tempo = Some(d);
                }
                "entrada" => {
                    let mut texto = String::new();
                    match v {
                        Value::Lista(xs) => {
                            for x in xs.borrow().iter() {
                                texto.push_str(&self.texto_de_linha(x, ":entrada", span)?);
                                texto.push('\n');
                            }
                        }
                        other => {
                            texto = self.texto_de_linha(other, ":entrada", span)?;
                            if !texto.is_empty() && !texto.ends_with('\n') {
                                texto.push('\n');
                            }
                        }
                    }
                    opcoes.entrada = Some(texto);
                }
                "exija" => opcoes.exija = self.expect_bool(v, span)?,
                "terminal" => opcoes.terminal = self.expect_bool(v, span)?,
                outra => {
                    return Err(self.err(
                        format!(
                            "opção desconhecida :{outra} (use :pasta, :ambiente, :tempo, \
                             :entrada, :exija ou :terminal)"
                        ),
                        span,
                    ));
                }
            }
        }
        Ok(opcoes)
    }

    /// Runs the stages on one line. `nada` drops it, a list gives several.
    fn aplique_etapas(
        &mut self,
        etapas: &[EtapaFuncao],
        linha: String,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Vec<String>, EvalError> {
        let mut linhas = vec![linha];
        for etapa in etapas {
            let mut proximas = Vec::new();
            for l in linhas {
                let mut args = Vec::with_capacity(1 + etapa.extras.len());
                args.push(Value::Texto(l));
                args.extend(etapa.extras.iter().cloned());
                match self.call_value(&etapa.f, &args, span, &etapa.nome, env)? {
                    Value::Nada => {}
                    Value::Lista(xs) => {
                        for x in xs.borrow().iter() {
                            proximas.push(self.texto_de_linha(x, &etapa.nome, span)?);
                        }
                    }
                    v => proximas.push(self.texto_de_linha(&v, &etapa.nome, span)?),
                }
            }
            linhas = proximas;
            if linhas.is_empty() {
                break;
            }
        }
        Ok(linhas)
    }

    fn texto_de_linha(&self, v: &Value, onde: &str, span: Span) -> Result<String, EvalError> {
        match v {
            Value::Texto(s) => Ok(s.clone()),
            Value::Numero(n) => Ok(format_numero(*n)),
            Value::Bool(_) => Ok(v.to_string()),
            other => Err(self.err(
                format!(
                    "{onde} devolveu {}; esperado texto, lista de textos ou nada",
                    other.type_name()
                ),
                span,
            )),
        }
    }
}

/// `:exija` message: the command, its code and the end of its stderr.
fn falhou(p: &Processo, codigo: i32) -> String {
    let mut msg = format!("'{}' falhou (código {codigo})", p.descricao);
    let fim: Vec<&str> = p
        .erros
        .iter()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .take(3)
        .map(String::as_str)
        .collect();
    if !fim.is_empty() {
        let fim: Vec<&str> = fim.into_iter().rev().collect();
        msg.push_str(": ");
        msg.push_str(&fim.join(" / "));
    }
    msg
}

fn chave(s: &str) -> MapKey {
    MapKey::Texto(s.to_string())
}

fn nome_da_etapa(f: &Value, n: usize) -> String {
    match f {
        Value::Builtin(nome) => (*nome).to_string(),
        _ => format!("etapa {n}"),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::time::{Duration, Instant};

    use crate::runtime::{run_to_string, run_to_string_with, run_to_strings_with};

    const FILTRO: &str = "importe \"processo\"\n\
        filtro = funcao(linha, oque) { se linha contem oque { linha } }\n";

    fn run(src: &str) -> String {
        let src = format!("{FILTRO}{src}");
        run_to_string(&src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err(src: &str) -> String {
        let src = format!("{FILTRO}{src}");
        match run_to_string(&src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    #[test]
    fn execute_devolve_codigo_saida_e_erro() {
        assert_eq!(
            run("escreva(execute(\"printf 'a\\nb\\n'\"))"),
            "mapa{\"codigo\": 0, \"ok\": verdadeiro, \"saida\": [\"a\", \"b\"], \"erro\": []}\n"
        );
        assert_eq!(
            run("r = execute([\"sh\", \"-c\", \"echo x >&2; exit 3\"])\n\
                 escreva(r:codigo, r:ok, r:erro)"),
            "3 falso [\"x\"]\n"
        );
    }

    #[test]
    fn funcoes_entre_comandos() {
        let src = "linhas = [\"a.txt\", \"b.md\", \"c.txt\"]\n\
                   r = execute(\"cat\", [filtro, \".txt\"], maiuscula, [\"wc\", \"-l\"], \
                   mapa([:entrada -> linhas]))\n\
                   escreva(r:saida)\n\
                   escreva(execute(\"cat\", [filtro, \".txt\"], maiuscula, \
                   mapa([:entrada -> linhas])):saida)";
        let out = run(src);
        let mut it = out.lines();
        assert_eq!(it.next().unwrap().replace(' ', ""), "[\"2\"]");
        assert_eq!(it.next().unwrap(), "[\"A.TXT\", \"C.TXT\"]");
    }

    #[test]
    fn lista_de_uma_funcao_vira_varias_linhas() {
        assert_eq!(
            run("dobra = funcao(l) { [l, l + l] }\n\
                 escreva(execute(\"printf 'a\\nb\\n'\", dobra, \"sort -r\"):saida)"),
            "[\"bb\", \"b\", \"aa\", \"a\"]\n"
        );
    }

    #[test]
    fn pipe_no_texto_e_sigpipe_nao_e_falha() {
        assert_eq!(
            run("escreva(execute(\"printf 'b\\na\\nb\\n' | sort | uniq\"):saida)"),
            "[\"a\", \"b\"]\n"
        );
        // `yes` never ends: `head` closing the pipe must stop it, with no failure.
        assert_eq!(
            run("r = execute(\"yes\", [filtro, \"y\"], \"head -2\")\nescreva(r:saida, r:codigo)"),
            "[\"y\", \"y\"] 0\n"
        );
        assert_eq!(run("escreva(execute(\"yes\", \"head -1\"):codigo)"), "0\n");
    }

    #[test]
    fn linhas_de_para_no_pare_e_encerra() {
        let inicio = Instant::now();
        assert_eq!(
            run("para l em linhas_de(\"yes\") {\n\
                     se l != \"y\" { sair(1) }\n\
                     escreva(l)\n\
                     pare\n\
                 }\n\
                 p = linhas_de(\"seq 3\")\n\
                 escreva(tipo(p))\n\
                 para l em p { escreva(l) }\n\
                 para l em p { escreva(\"outra volta\") }"),
            "y\nprocesso\n1\n2\n3\n"
        );
        assert!(inicio.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn linhas_de_manda_stderr_para_escreva_erro() {
        let (out, err) = run_to_strings_with(
            "importe \"processo\"\n\
             para l em linhas_de([\"sh\", \"-c\", \"echo a; echo b >&2\"]) { escreva(l) }",
            "teste.lep",
            "",
        )
        .unwrap();
        assert_eq!(out, "a\n");
        assert_eq!(err, "b\n");
    }

    #[test]
    fn opcoes() {
        let dir = std::env::temp_dir().join(format!("expressa-proc-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/x.txt"), "").unwrap();
        let src = format!(
            "{FILTRO}escreva(execute(\"ls\", mapa([:pasta -> \"sub\"])):saida)\n\
             escreva(execute([\"sh\", \"-c\", \"echo $NOME\"], mapa([:ambiente -> mapa([:NOME -> 7])])):saida)\n\
             escreva(execute(\"cat\", mapa([:entrada -> \"um texto\"])):saida)"
        );
        let out = run_to_string(&src, &dir.join("t.lep").to_string_lossy()).unwrap();
        assert_eq!(out, "[\"x.txt\"]\n[\"7\"]\n[\"um texto\"]\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tempo_esgotado_encerra() {
        let inicio = Instant::now();
        let e = run_err("execute(\"sleep 10\", mapa([:tempo -> 0.2]))");
        assert!(e.contains("tempo esgotado"), "{e}");
        assert!(inicio.elapsed() < Duration::from_secs(5));
        assert_eq!(
            run("escreva(execute(\"sleep 10\", mapa([:tempo -> 0.1])) se_falhar \"desisti\")"),
            "desisti\n"
        );
    }

    #[test]
    fn erros_de_etapa() {
        assert!(run_err("execute(\"nao_existe_xyz\")").contains("comando não encontrado"));
        assert!(run_err("execute(filtro)").contains("primeiro argumento"));
        assert!(run_err("execute()").contains("espera um comando"));
        assert!(run_err("execute(\"ls *.txt\")").contains("não há shell"));
        assert!(run_err("execute(\"ls\", mapa([]), \"wc\")").contains("último argumento"));
        assert!(run_err("execute(\"ls\", mapa([:pastaa -> 1]))").contains("opção desconhecida"));
        assert!(
            run_err("execute(\"ls\", mapa([:pasta -> \"nao/existe\"]))").contains("não existe")
        );
        assert!(run_err("execute(\"ls\", [])").contains("lista vazia"));
        assert!(run_err("execute(\"ls\", 3)").contains("encontrado numero"));
        assert!(run_err("execute([\"ls\", verdadeiro])").contains("só pode ter textos"));
        assert!(run_err("execute(\"echo a\", filtro)").contains("a função tem 2 parâmetro(s)"));
        assert!(
            run_err("execute(\"echo a\", funcao(l) { mapa() })").contains("etapa 2 devolveu mapa")
        );
        assert!(run_err("tamanho(linhas_de(\"echo a\"))").contains("encontrado processo"));
    }

    #[test]
    fn erro_numa_etapa_sobe_e_encerra() {
        let inicio = Instant::now();
        let e = run_err("execute(\"yes\", funcao(l) { 1 / 0 })");
        assert!(
            e.contains("divisão por zero") || e.contains("dividir"),
            "{e}"
        );
        assert!(inicio.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn exija_transforma_falha_em_erro() {
        let e = run_err(
            "execute([\"sh\", \"-c\", \"echo a >&2; echo 'fatal: x' >&2; exit 2\"], \
             mapa([:exija -> verdadeiro]))",
        );
        assert!(e.contains("falhou (código 2): a / fatal: x"), "{e}");
        assert_eq!(
            run(
                "escreva(execute(\"false\", mapa([:exija -> verdadeiro])) se_falhar \"pego\")\n\
                 escreva(execute(\"true\", mapa([:exija -> verdadeiro])):ok)"
            ),
            "pego\nverdadeiro\n"
        );
        let e = run_err(
            "para l em linhas_de(\"sh -c 'echo um; exit 5'\", mapa([:exija -> verdadeiro])) { \
             escreva(l) }",
        );
        assert!(e.contains("código 5"), "{e}");
        // Leaving the loop early is not a failure.
        assert_eq!(
            run("para l em linhas_de(\"yes\", mapa([:exija -> verdadeiro])) { pare }\nescreva(1)"),
            "1\n"
        );
    }

    #[test]
    fn terminal() {
        assert_eq!(
            run(
                "r = execute(\"sh -c 'exit 3'\", mapa([:terminal -> verdadeiro]))\n\
                 escreva(r:codigo, r:saida, r:erro)"
            ),
            "3 [] []\n"
        );
        assert!(
            run_err("linhas_de(\"true\", mapa([:terminal -> verdadeiro]))").contains("use execute")
        );
        assert!(
            run_err("execute(\"true\", maiuscula, mapa([:terminal -> verdadeiro]))")
                .contains("só podem ser comandos")
        );
        assert!(
            run_err("execute(\"cat\", mapa([:terminal -> verdadeiro, :entrada -> \"x\"]))")
                .contains("tire :entrada")
        );
        assert!(run_err("execute(\"true\", mapa([:terminal -> 1]))").contains("esperado bool"));
    }

    #[test]
    fn pasta_confinada_nao_roda_processos() {
        let dir = std::env::temp_dir();
        let e = run_to_string_with(
            "importe \"processo\"\nexecute(\"ls\")",
            &dir.join("t.lep").to_string_lossy(),
            "",
            Some(dir.clone()),
            None,
        )
        .unwrap_err();
        assert!(
            e.message.contains("só na linha de comando"),
            "{}",
            e.message
        );
    }
}
