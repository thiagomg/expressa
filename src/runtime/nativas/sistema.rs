use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::lexer::Span;

use super::super::env::Env;
use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::numero::Numero;
use super::super::value::{MapKey, Value};

/// Fields of the map `agora()` returns, in order.
const CAMPOS_DATA: [&str; 8] = [
    "ano",
    "mes",
    "dia",
    "hora",
    "minuto",
    "segundo",
    "dia_da_semana",
    "epoch",
];

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

    /// Local date and time as a map (see `CAMPOS_DATA`).
    pub(crate) fn bi_agora(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Ok(mapa_de_data(&data_local(epoch)))
    }

    /// `formate_data(data, modelo)`: `%Y %m %d %H %M %S %%`. `data` is the
    /// map from `agora()` or seconds since 1970 (`:epoch`).
    pub(crate) fn bi_formate_data(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let d = match &args[0] {
            Value::Numero(_) => data_local(self.expect_int(&args[0], span)?),
            Value::Mapa(m) => {
                let mut campos = [0i64; 6];
                for (i, nome) in CAMPOS_DATA[..6].iter().enumerate() {
                    let v = m
                        .borrow()
                        .iter()
                        .find(|(k, _)| matches!(k, MapKey::Texto(s) if s == nome))
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| {
                            self.err(format!("formate_data: falta :{nome} na data"), span)
                        })?;
                    campos[i] = self.expect_int(&v, span)?;
                }
                let [ano, mes, dia, hora, minuto, segundo] = campos;
                Data {
                    ano,
                    mes,
                    dia,
                    hora,
                    minuto,
                    segundo,
                    dia_da_semana: 0,
                    epoch: 0,
                }
            }
            other => {
                return Err(self.err(
                    format!(
                        "formate_data espera o mapa de agora() ou um número, encontrado {}",
                        other.type_name()
                    ),
                    span,
                ));
            }
        };
        let modelo = self.expect_texto(&args[1], span)?;
        let mut out = String::new();
        let mut chars = modelo.chars();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('Y') => out.push_str(&format!("{:04}", d.ano)),
                Some('m') => out.push_str(&format!("{:02}", d.mes)),
                Some('d') => out.push_str(&format!("{:02}", d.dia)),
                Some('H') => out.push_str(&format!("{:02}", d.hora)),
                Some('M') => out.push_str(&format!("{:02}", d.minuto)),
                Some('S') => out.push_str(&format!("{:02}", d.segundo)),
                Some('%') => out.push('%'),
                outro => {
                    return Err(self.err(
                        format!(
                            "formate_data: %{} não existe (use %Y %m %d %H %M %S ou %%)",
                            outro.map(String::from).unwrap_or_default()
                        ),
                        span,
                    ));
                }
            }
        }
        Ok(Value::Texto(out))
    }
}

struct Data {
    ano: i64,
    mes: i64,
    dia: i64,
    hora: i64,
    minuto: i64,
    segundo: i64,
    /// 1 = domingo … 7 = sábado.
    dia_da_semana: i64,
    epoch: i64,
}

fn mapa_de_data(d: &Data) -> Value {
    let vals = [
        d.ano,
        d.mes,
        d.dia,
        d.hora,
        d.minuto,
        d.segundo,
        d.dia_da_semana,
        d.epoch,
    ];
    Value::mapa(
        CAMPOS_DATA
            .iter()
            .zip(vals)
            .map(|(k, v)| {
                (
                    MapKey::Texto((*k).to_string()),
                    Value::Numero(Numero::from_i64(v)),
                )
            })
            .collect(),
    )
}

#[cfg(unix)]
fn data_local(epoch: i64) -> Data {
    let t = epoch as libc::time_t;
    // SAFETY: `tm` is plain data and `localtime_r` only writes into it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return data_utc(epoch);
    }
    Data {
        ano: i64::from(tm.tm_year) + 1900,
        mes: i64::from(tm.tm_mon) + 1,
        dia: i64::from(tm.tm_mday),
        hora: i64::from(tm.tm_hour),
        minuto: i64::from(tm.tm_min),
        segundo: i64::from(tm.tm_sec),
        dia_da_semana: i64::from(tm.tm_wday) + 1,
        epoch,
    }
}

/// No time zone outside Unix yet: the clock is UTC.
#[cfg(not(unix))]
fn data_local(epoch: i64) -> Data {
    data_utc(epoch)
}

/// Civil date from days since 1970 (Howard Hinnant's algorithm).
fn data_utc(epoch: i64) -> Data {
    let dias = epoch.div_euclid(86_400);
    let resto = epoch.rem_euclid(86_400);
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + i64::from(mes <= 2);
    Data {
        ano,
        mes,
        dia,
        hora: resto / 3600,
        minuto: resto % 3600 / 60,
        segundo: resto % 60,
        // 1970-01-01 was a Thursday (5).
        dia_da_semana: (dias + 4).rem_euclid(7) + 1,
        epoch,
    }
}

#[cfg(test)]
mod tests {
    use super::data_utc;
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

    #[test]
    fn agora_e_formate_data() {
        assert_eq!(
            run(
                "d = agora()\nescreva(d:mes >= 1 e d:mes <= 12, d:dia_da_semana <= 7, \
                 formate_data(d, \"%Y\") == formate_data(d:epoch, \"%Y\"))"
            ),
            "verdadeiro verdadeiro verdadeiro\n"
        );
        assert_eq!(
            run(
                "d = mapa([:ano -> 2024, :mes -> 3, :dia -> 9, :hora -> 7, :minuto -> 5, \
                 :segundo -> 0])\nescreva(formate_data(d, \"%Y-%m-%d %H:%M:%S 100%%\"))"
            ),
            "2024-03-09 07:05:00 100%\n"
        );
        assert!(run_err("formate_data(agora(), \"%x\")").contains("%x não existe"));
        assert!(run_err("formate_data(mapa([:ano -> 1]), \"%Y\")").contains("falta :mes"));
    }

    #[test]
    fn datas_utc() {
        let d = data_utc(0);
        assert_eq!(
            (d.ano, d.mes, d.dia, d.hora, d.dia_da_semana),
            (1970, 1, 1, 0, 5)
        );
        // 2024-02-29 13:14:15 UTC, a Thursday.
        let d = data_utc(1_709_212_455);
        assert_eq!(
            (
                d.ano,
                d.mes,
                d.dia,
                d.hora,
                d.minuto,
                d.segundo,
                d.dia_da_semana
            ),
            (2024, 2, 29, 13, 14, 15, 5)
        );
        let d = data_utc(-1);
        assert_eq!((d.ano, d.mes, d.dia, d.hora), (1969, 12, 31, 23));
    }
}
