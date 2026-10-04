use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::lexer::Span;
use crate::parser::parse;

use super::super::env::Env;
use super::super::error::{CallFrame, EvalError};
use super::super::eval::Vm;
use super::super::value::{MapKey, NumeroLocale, Value, conjunto_insert};
use super::{builtin_args, builtin_modulo, sem_acento, value_as_texto};

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
    pub(crate) fn bi_escreva_erro(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_leia_linhas(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
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
            return Err(self.err("formate() espera um modelo (texto) e os valores", span));
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
    pub(crate) fn remova_mapa(
        &self,
        map: &Value,
        key_v: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
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
    pub(crate) fn remova_conjunto(
        &self,
        set: &Value,
        elem: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
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
    pub(crate) fn bi_procurar(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        match &args[0] {
            Value::Texto(s) => {
                let needle = self.expect_texto(&args[1], span)?;
                if needle.is_empty() {
                    return Err(self.err("procurar espera um trecho não vazio", span));
                }
                let pos = match s.find(&needle) {
                    Some(byte_i) => s[..byte_i].chars().count() as f64 + 1.0,
                    None => 0.0,
                };
                Ok(Value::Numero(pos))
            }
            Value::Lista(xs) => {
                let pos = xs
                    .borrow()
                    .iter()
                    .position(|x| x == &args[1])
                    .map(|i| (i + 1) as f64)
                    .unwrap_or(0.0);
                Ok(Value::Numero(pos))
            }
            other => Err(self.err(
                format!(
                    "procurar espera texto ou lista, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
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
        let prev_source = std::mem::replace(&mut self.source, std::rc::Rc::from(src));
        let result = self.eval_items_value(&program.items, env);
        self.source = prev_source;
        self.file = prev_file;
        self.stack.pop();
        result
    }

    pub(crate) fn bi_catalogo(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        match args {
            [] => self.catalog_scope(env),
            [v] => self.catalog_value(v, span),
            _ => Err(self.err(
                format!(
                    "catalogo() espera 0 ou 1 argumento(s), recebeu {}",
                    args.len()
                ),
                span,
            )),
        }
    }

    fn catalog_scope(&self, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        let mut rows: Vec<(String, Value)> = Vec::new();
        let mut names = HashSet::new();
        for (name, value) in Env::visible_bindings(env) {
            if let Some(entry) = catalog_entry(&name, &value) {
                names.insert(name.clone());
                rows.push((name, entry));
            }
        }
        let mut mods: Vec<_> = self.native_modules.keys().cloned().collect();
        mods.sort();
        for m in mods {
            if names.contains(&m) {
                continue;
            }
            let membros = self
                .native_modules
                .get(&m)
                .map(|e| module_member_names(&e.borrow()))
                .unwrap_or_default();
            rows.push((m.clone(), catalog_row(&m, "modulo", &membros, &m)));
            names.insert(m);
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(Value::lista(rows.into_iter().map(|(_, v)| v).collect()))
    }

    fn catalog_value(&self, v: &Value, span: Span) -> Result<Value, EvalError> {
        match v {
            Value::Texto(s) => {
                let Some(mod_env) = self.native_modules.get(s) else {
                    return Err(self.err(format!("módulo nativo `{s}` não existe"), span));
                };
                Ok(catalog_frame(&mod_env.borrow(), s))
            }
            Value::Modulo(mod_env) => Ok(catalog_frame(&mod_env.borrow(), "")),
            Value::Mapa(xs) => {
                let mut rows: Vec<(String, Value)> = xs
                    .borrow()
                    .iter()
                    .map(|(k, val)| {
                        let nome = mapkey_nome(k);
                        (nome.clone(), catalog_value_row(&nome, val))
                    })
                    .collect();
                rows.sort_by(|a, b| a.0.cmp(&b.0));
                Ok(Value::lista(rows.into_iter().map(|(_, e)| e).collect()))
            }
            Value::Funcao(_) | Value::Builtin(_) => {
                Ok(Value::lista(vec![catalog_value_row("", v)]))
            }
            other => Err(self.err(
                format!(
                    "catalogo espera um módulo, mapa, função ou nome de módulo, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }
}

fn catalog_entry(name: &str, value: &Value) -> Option<Value> {
    match value {
        Value::Builtin(_) | Value::Funcao(_) | Value::Modulo(_) => {
            Some(catalog_value_row(name, value))
        }
        _ => None,
    }
}

fn catalog_value_row(name: &str, value: &Value) -> Value {
    match value {
        Value::Builtin(canon) => {
            catalog_row(name, "nativa", &builtin_args(canon), builtin_modulo(canon))
        }
        Value::Funcao(c) => catalog_row(name, "funcao", &c.args_text(), ""),
        Value::Modulo(env) => {
            catalog_row(name, "modulo", &module_member_names(&env.borrow()), name)
        }
        other => catalog_row(name, other.type_name(), "", ""),
    }
}

fn catalog_frame(env: &Env, modulo: &str) -> Value {
    let mut rows = Vec::new();
    for (name, value) in env.bindings_sorted() {
        match &value {
            Value::Builtin(canon) => {
                rows.push(catalog_row(&name, "nativa", &builtin_args(canon), modulo))
            }
            Value::Funcao(c) => rows.push(catalog_row(&name, "funcao", &c.args_text(), modulo)),
            Value::Modulo(_) => {
                if let Some(e) = catalog_entry(&name, &value) {
                    rows.push(e);
                }
            }
            _ => {}
        }
    }
    Value::lista(rows)
}

fn mapkey_nome(k: &MapKey) -> String {
    match k {
        MapKey::Texto(s) => s.clone(),
        MapKey::Numero(n) => n.to_string(),
        MapKey::Bool(true) => "verdadeiro".into(),
        MapKey::Bool(false) => "falso".into(),
    }
}

fn catalog_row(nome: &str, tipo: &str, args: &str, modulo: &str) -> Value {
    Value::mapa(vec![
        (MapKey::Texto("nome".into()), Value::Texto(nome.into())),
        (MapKey::Texto("tipo".into()), Value::Texto(tipo.into())),
        (MapKey::Texto("args".into()), Value::Texto(args.into())),
        (MapKey::Texto("modulo".into()), Value::Texto(modulo.into())),
    ])
}

fn module_member_names(env: &Env) -> String {
    env.bindings_sorted()
        .into_iter()
        .filter(|(_, v)| matches!(v, Value::Builtin(_) | Value::Funcao(_)))
        .map(|(k, _)| k)
        .collect::<Vec<_>>()
        .join(", ")
}
