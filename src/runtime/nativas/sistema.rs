use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::lexer::Span;

use super::super::env::Env;
use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;

impl Vm<'_, '_> {
    pub(crate) fn bi_ambiente(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let nome = self.expect_texto(&args[0], span)?;
        match std::env::var_os(&nome) {
            Some(v) => Ok(Value::Texto(v.to_string_lossy().into_owned())),
            None => Err(self.err(format!("variável de ambiente não definida: {nome}"), span)),
        }
    }

    pub(crate) fn bi_comeca_com(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let t = self.expect_texto(&args[0], span)?;
        let p = self.expect_texto(&args[1], span)?;
        Ok(Value::Bool(t.starts_with(&p)))
    }

    pub(crate) fn bi_termina_com(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let t = self.expect_texto(&args[0], span)?;
        let s = self.expect_texto(&args[1], span)?;
        Ok(Value::Bool(t.ends_with(&s)))
    }

    /// `ordene(lista)` / `ordene(lista, chave)`: a sorted copy (stable).
    /// Keys are all numbers or all texts, compared like `<`.
    pub(crate) fn bi_ordene(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        if args.is_empty() || args.len() > 2 {
            return Err(self.err(
                format!("ordene espera 1 ou 2 argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        let itens = self.expect_lista(&args[0], span)?.borrow().clone();
        let chaves = match args.get(1) {
            None => itens.clone(),
            Some(f @ (Value::Funcao(_) | Value::Builtin(_))) => {
                let mut ks = Vec::with_capacity(itens.len());
                for x in &itens {
                    ks.push(self.call_value(f, std::slice::from_ref(x), span, "chave", env)?);
                }
                ks
            }
            Some(other) => {
                return Err(self.err(
                    format!(
                        "ordene: a chave tem que ser uma função, encontrado {}",
                        other.type_name()
                    ),
                    span,
                ));
            }
        };
        let tipo = chaves.first().map(Value::type_name);
        for k in &chaves {
            if !matches!(k, Value::Numero(_) | Value::Texto(_)) || Some(k.type_name()) != tipo {
                return Err(self.err(
                    format!(
                        "ordene só compara números com números ou textos com textos \
                         (encontrado {} e {})",
                        tipo.unwrap_or("nada"),
                        k.type_name()
                    ),
                    span,
                ));
            }
        }
        let mut ordem: Vec<usize> = (0..itens.len()).collect();
        ordem.sort_by(|&a, &b| match (&chaves[a], &chaves[b]) {
            (Value::Numero(x), Value::Numero(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
            (Value::Texto(x), Value::Texto(y)) => x.cmp(y),
            _ => Ordering::Equal,
        });
        Ok(Value::lista(
            ordem.into_iter().map(|i| itens[i].clone()).collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn ambiente_le_variaveis() {
        assert_eq!(run("escreva(ambiente(\"PATH\") == \"\")"), "falso\n");
        assert_eq!(
            run("escreva(ambiente(\"EXPRESSA_NAO_EXISTE_X\") se_falhar \"padrão\")"),
            "padrão\n"
        );
        assert!(run_err("ambiente(\"EXPRESSA_NAO_EXISTE_X\")").contains("não definida"));
    }

    #[test]
    fn ordene_numeros_textos_e_chave() {
        assert_eq!(run("escreva(ordene([3, 1.5, 2]))"), "[1,5, 2, 3]\n");
        assert_eq!(
            run("escreva(ordene([\"b\", \"a\", \"c\"]))"),
            "[\"a\", \"b\", \"c\"]\n"
        );
        assert_eq!(run("escreva(ordene([]))"), "[]\n");
        // Stable: equal keys keep their order.
        assert_eq!(
            run("escreva(ordene([\"bb\", \"a\", \"cc\", \"d\"], tamanho))"),
            "[\"a\", \"d\", \"bb\", \"cc\"]\n"
        );
        assert_eq!(
            run("escreva(ordene([1, 3, 2], funcao(x) { -x }))"),
            "[3, 2, 1]\n"
        );
        let xs = run("xs = [2, 1]\nys = ordene(xs)\nescreva(xs, ys)");
        assert_eq!(xs, "[2, 1] [1, 2]\n");
        assert!(run_err("ordene([1, \"a\"])").contains("números com números"));
        assert!(run_err("ordene([[1]])").contains("encontrado lista"));
        assert!(run_err("ordene([1], 2)").contains("tem que ser uma função"));
    }

    #[test]
    fn comeca_e_termina() {
        assert_eq!(
            run(
                "escreva(comeca_com(\"abc\", \"ab\"), começa_com(\"abc\", \"b\"), \
                 termina_com(\"abc\", \"bc\"), \"abc\".termina_com(\"\"))"
            ),
            "verdadeiro falso verdadeiro verdadeiro\n"
        );
    }
}
