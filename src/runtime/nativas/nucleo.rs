use std::cell::RefCell;
use std::rc::Rc;

use crate::lexer::Span;
use crate::parser::parse;

use super::super::env::Env;
use super::super::error::{CallFrame, EvalError};
use super::super::eval::Vm;
use super::super::value::{MapKey, NumeroLocale, Value, conjunto_insert};
use super::{sem_acento, value_as_texto};

impl Vm<'_> {
    pub(crate) fn bi_escreva(&mut self, args: &[Value], _span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_escreva_erro(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_durma(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let secs = self.expect_numero(&args[0], span)?;
        self.sleep_secs(secs, span)?;
        Ok(Value::Nada)
    }
    pub(crate) fn bi_sair(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_leia(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
        super::super::leia::strip_newline(&mut line);
        Ok(Value::Texto(line))
    }
    pub(crate) fn bi_leia_linhas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
            super::super::leia::strip_newline(&mut line);
            lines.push(Value::Texto(line));
        }
        Ok(Value::lista(lines))
    }
    pub(crate) fn bi_numero(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_formato(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_formate(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.is_empty() {
            return Err(self.err(
                "formate() espera um modelo (texto) e os valores",
                span,
            ));
        }
        let modelo = self.expect_texto(&args[0], span)?;
        let text = super::super::formate::formate(&modelo, &args[1..], self.numero_locale)
            .map_err(|msg| self.err(msg, span))?;
        Ok(Value::Texto(text))
    }
    pub(crate) fn bi_tamanho(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = match &args[0] {
            Value::Texto(s) => s.chars().count() as f64,
            Value::Lista(xs) => xs.borrow().len() as f64,
            Value::Mapa(xs) => xs.borrow().len() as f64,
            Value::Conjunto(xs) => xs.borrow().len() as f64,
            Value::Matriz(m) => m.borrow().len() as f64,
            other => {
                return Err(self.err(
                    format!(
                        "tamanho() espera texto, lista, mapa, conjunto ou matriz, encontrado {}",
                        other.type_name()
                    ),
                    span,
                ));
            }
        };
        Ok(Value::Numero(n))
    }
    pub(crate) fn bi_primeiro(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let xs = self.expect_lista(&args[0], span)?;
        xs.borrow()
            .first()
            .cloned()
            .ok_or_else(|| self.err("primeiro() de lista vazia", span))
    }
    pub(crate) fn bi_ultimo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let xs = self.expect_lista(&args[0], span)?;
        xs.borrow()
            .last()
            .cloned()
            .ok_or_else(|| self.err("ultimo() de lista vazia", span))
    }
    pub(crate) fn bi_maiuscula(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(s.to_uppercase()))
    }
    pub(crate) fn bi_minuscula(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(s.to_lowercase()))
    }
    pub(crate) fn bi_sem_acento(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(sem_acento(&s)))
    }
    pub(crate) fn bi_remova(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() < 2 || args.len() > 3 {
            return Err(self.err(
                format!("remova espera 2 ou 3 argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        match &args[0] {
            Value::Mapa(_) => {
                if args.len() != 2 {
                    return Err(self.err(
                        "remova em mapa espera a chave (não uma faixa de índices)",
                        span,
                    ));
                }
                self.remova_mapa(&args[0], &args[1], span)
            }
            Value::Conjunto(_) => {
                if args.len() != 2 {
                    return Err(self.err(
                        "remova em conjunto espera o elemento (não uma faixa de índices)",
                        span,
                    ));
                }
                self.remova_conjunto(&args[0], &args[1], span)
            }
            Value::Lista(_) | Value::Texto(_) => {
                let end = if args.len() == 3 { &args[2] } else { &args[1] };
                self.remova_faixa(&args[0], &args[1], end, span)
            }
            other => Err(self.err(
                format!(
                    "remova espera lista, texto, mapa ou conjunto, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }
    pub(crate) fn remova_mapa(&self, map: &Value, key_v: &Value, span: Span) -> Result<Value, EvalError> {
        let Value::Mapa(xs) = map else {
            unreachable!();
        };
        let key = MapKey::from_value(key_v).ok_or_else(|| {
            self.err(
                format!("chave de mapa inválida ({})", key_v.type_name()),
                span,
            )
        })?;
        let mut entries = xs.borrow().clone();
        let before = entries.len();
        entries.retain(|(k, _)| *k != key);
        if entries.len() == before {
            return Err(self.err(format!("chave {key} não existe no mapa"), span));
        }
        Ok(Value::mapa(entries))
    }
    pub(crate) fn remova_conjunto(&self, set: &Value, elem: &Value, span: Span) -> Result<Value, EvalError> {
        let Value::Conjunto(xs) = set else {
            unreachable!();
        };
        let key = MapKey::from_value(elem).ok_or_else(|| {
            self.err(
                format!("elemento de conjunto inválido ({})", elem.type_name()),
                span,
            )
        })?;
        let mut items = xs.borrow().clone();
        let before = items.len();
        items.retain(|k| *k != key);
        if items.len() == before {
            return Err(self.err(format!("elemento {key} não existe no conjunto"), span));
        }
        Ok(Value::conjunto(items))
    }
    pub(crate) fn remova_faixa(
        &self,
        coll: &Value,
        start: &Value,
        end: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        let s = self.expect_int(start, span)?;
        let e = self.expect_int(end, span)?;
        if s > e {
            return Err(self.err(
                format!("faixa de remova: início {s} maior que o fim {e}"),
                span,
            ));
        }
        match coll {
            Value::Lista(xs) => {
                let mut v = xs.borrow().clone();
                let (a, b) = self.to_slice_bounds(start, Some(end), v.len(), span)?;
                v.drain(a..b);
                Ok(Value::lista(v))
            }
            Value::Texto(t) => {
                let mut chars: Vec<char> = t.chars().collect();
                let (a, b) = self.to_slice_bounds(start, Some(end), chars.len(), span)?;
                chars.drain(a..b);
                Ok(Value::Texto(chars.into_iter().collect()))
            }
            _ => unreachable!(),
        }
    }
    pub(crate) fn bi_substitua(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let s = self.expect_texto(&args[0], span)?;
        let from = self.expect_texto(&args[1], span)?;
        let to = self.expect_texto(&args[2], span)?;
        Ok(Value::Texto(s.replace(&from, &to)))
    }
    pub(crate) fn bi_separe(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_junte(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let xs = self.expect_lista(&args[0], span)?;
        let sep = self.expect_texto(&args[1], span)?;
        let mut parts = Vec::new();
        for v in xs.borrow().iter() {
            parts.push(value_as_texto(v, self.numero_locale));
        }
        Ok(Value::Texto(parts.join(&sep)))
    }
    pub(crate) fn bi_limpe(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let s = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(s.trim().to_string()))
    }
    pub(crate) fn bi_mapa(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        match args {
            [] => Ok(Value::mapa(vec![])),
            [v] => {
                let xs = self.expect_lista(v, span)?;
                let mut map = Vec::new();
                for item in xs.borrow().iter() {
                    let Value::Par(k, val) = item else {
                        return Err(self.err(
                            format!(
                                "mapa() espera lista de pares, encontrado {}",
                                item.type_name()
                            ),
                            span,
                        ));
                    };
                    if map.iter().any(|(mk, _)| mk == k) {
                        return Err(self.err(format!("chave duplicada {k} no mapa"), span));
                    }
                    map.push((k.clone(), val.as_ref().clone()));
                }
                Ok(Value::mapa(map))
            }
            _ => Err(self.err(
                format!("mapa() espera 0 ou 1 argumento(s), recebeu {}", args.len()),
                span,
            )),
        }
    }

    pub(crate) fn bi_conjunto(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        match args {
            [] => Ok(Value::conjunto(vec![])),
            [v] => {
                let xs = self.expect_lista(v, span)?;
                let mut out = Vec::new();
                for item in xs.borrow().iter() {
                    let k = MapKey::from_value(item).ok_or_else(|| {
                        self.err(
                            format!("elemento de conjunto inválido ({})", item.type_name()),
                            span,
                        )
                    })?;
                    conjunto_insert(&mut out, k);
                }
                Ok(Value::conjunto(out))
            }
            _ => Err(self.err(
                format!(
                    "conjunto() espera 0 ou 1 argumento(s), recebeu {}",
                    args.len()
                ),
                span,
            )),
        }
    }

    pub(crate) fn bi_avaliar(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let src = self.expect_texto(&args[0], span)?;
        let program = parse(&src).map_err(|e| self.err(format!("avaliar: {}", e.message), span))?;
        if program.items.is_empty() {
            return Err(self.err("avaliar espera uma expressão", span));
        }
        self.stack.push(CallFrame {
            name: "avaliar".into(),
            file: self.file.clone(),
            span,
        });
        let prev_file = std::mem::replace(&mut self.file, "<avaliar>".into());
        let prev_source = std::mem::replace(&mut self.source, src);
        let result = self.eval_items_value(&program.items, env);
        self.source = prev_source;
        self.file = prev_file;
        self.stack.pop();
        result
    }
}
