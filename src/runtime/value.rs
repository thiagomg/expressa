use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::lexer::Span;
use crate::parser::{Block, Param};

use super::env::Env;

#[derive(Clone)]
pub enum Value {
    Nada,
    Numero(f64),
    Texto(String),
    Bool(bool),
    Lista(Rc<RefCell<Vec<Value>>>),
    Mapa(Rc<RefCell<Vec<(MapKey, Value)>>>),
    Matriz(Rc<RefCell<Vec<Vec<f64>>>>),
    Funcao(Rc<Closure>),
    Builtin(&'static str),
    Modulo(Rc<RefCell<Env>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapKey {
    Texto(String),
    Numero(i64),
    Bool(bool),
}

pub struct Closure {
    pub params: Vec<Param>,
    pub body: Block,
    pub env: Rc<RefCell<Env>>,
    pub span: Span,
}

impl Value {
    pub fn lista(items: Vec<Value>) -> Self {
        Value::Lista(Rc::new(RefCell::new(items)))
    }

    pub fn mapa(entries: Vec<(MapKey, Value)>) -> Self {
        Value::Mapa(Rc::new(RefCell::new(entries)))
    }

    pub fn matriz(rows: Vec<Vec<f64>>) -> Self {
        Value::Matriz(Rc::new(RefCell::new(rows)))
    }

    /// How the REPL prints a value (`"texto"` quoted; numbers follow locale).
    pub fn repl_format(&self, loc: NumeroLocale) -> String {
        match self {
            Value::Texto(s) => format!("\"{}\"", escape_texto(s)),
            Value::Numero(n) => loc.format(*n),
            other => other.to_string(),
        }
    }

    /// User-facing print (`escreva`, `"a" + n`): numbers follow locale.
    pub fn format_with(&self, loc: NumeroLocale) -> String {
        match self {
            Value::Numero(n) => loc.format(*n),
            Value::Texto(s) => s.clone(),
            Value::Lista(xs) => {
                let inner: Vec<_> = xs
                    .borrow()
                    .iter()
                    .map(|v| match v {
                        Value::Texto(s) => format!("\"{}\"", escape_texto(s)),
                        other => other.format_with(loc),
                    })
                    .collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Mapa(xs) => {
                let inner: Vec<_> = xs
                    .borrow()
                    .iter()
                    .map(|(k, v)| {
                        let vs = match v {
                            Value::Texto(s) => format!("\"{}\"", escape_texto(s)),
                            other => other.format_with(loc),
                        };
                        format!("{k}: {vs}")
                    })
                    .collect();
                format!("mapa{{{}}}", inner.join(", "))
            }
            Value::Matriz(m) => format_matriz(&m.borrow(), loc),
            other => other.to_string(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nada => "nada",
            Value::Numero(_) => "numero",
            Value::Texto(_) => "texto",
            Value::Bool(_) => "bool",
            Value::Lista(_) => "lista",
            Value::Mapa(_) => "mapa",
            Value::Matriz(_) => "matriz",
            Value::Funcao(_) => "funcao",
            Value::Builtin(_) => "funcao",
            Value::Modulo(_) => "modulo",
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nada, Value::Nada) => true,
            (Value::Numero(a), Value::Numero(b)) => a == b,
            (Value::Texto(a), Value::Texto(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Lista(a), Value::Lista(b)) => a.borrow()[..] == b.borrow()[..],
            (Value::Mapa(a), Value::Mapa(b)) => a.borrow()[..] == b.borrow()[..],
            (Value::Matriz(a), Value::Matriz(b)) => a.borrow()[..] == b.borrow()[..],
            (Value::Funcao(a), Value::Funcao(b)) => Rc::ptr_eq(a, b),
            (Value::Builtin(a), Value::Builtin(b)) => a == b,
            (Value::Modulo(a), Value::Modulo(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Display for MapKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapKey::Texto(s) => write!(f, "\"{}\"", escape_texto(s)),
            MapKey::Numero(n) => write!(f, "{n}"),
            MapKey::Bool(true) => write!(f, "verdadeiro"),
            MapKey::Bool(false) => write!(f, "falso"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nada => write!(f, "nada"),
            Value::Numero(n) => write!(f, "{}", format_numero(*n)),
            Value::Texto(s) => write!(f, "{s}"),
            Value::Bool(true) => write!(f, "verdadeiro"),
            Value::Bool(false) => write!(f, "falso"),
            Value::Lista(xs) => {
                write!(f, "[")?;
                for (i, v) in xs.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write_debug_value(f, v)?;
                }
                write!(f, "]")
            }
            Value::Mapa(xs) => {
                write!(f, "mapa{{")?;
                for (i, (k, v)) in xs.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: ")?;
                    write_debug_value(f, v)?;
                }
                write!(f, "}}")
            }
            Value::Matriz(m) => write!(f, "{}", format_matriz(&m.borrow(), NumeroLocale::PtBr)),
            Value::Funcao(_) => write!(f, "<funcao>"),
            Value::Builtin(name) => write!(f, "<funcao {name}>"),
            Value::Modulo(_) => write!(f, "<modulo>"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// Quoted form used inside lists/maps so nested text is visible.
fn write_debug_value(f: &mut fmt::Formatter<'_>, v: &Value) -> fmt::Result {
    match v {
        Value::Texto(s) => write!(f, "\"{}\"", escape_texto(s)),
        other => write!(f, "{other}"),
    }
}

/// How `numero()` / `escreva` interpret and print numbers in text.
/// Source literals always use `.` (`3.14`, `1_000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumeroLocale {
    /// `1.000,5` — default, matches Brazilian notebooks.
    #[default]
    PtBr,
    /// `1,000.5`
    EnUs,
}

impl NumeroLocale {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pt" | "pt-br" | "pt_br" | "br" | "brasil" | "brasileiro" => Some(Self::PtBr),
            "en" | "en-us" | "en_us" | "us" | "eua" | "americano" => Some(Self::EnUs),
            _ => None,
        }
    }

    pub fn parse_texto(self, raw: &str) -> Option<f64> {
        let s = raw.trim();
        if s.is_empty() {
            return None;
        }
        let s: String = s.chars().filter(|c| *c != '_' && *c != ' ').collect();
        match self {
            Self::PtBr => parse_pt_br(&s),
            Self::EnUs => parse_en_us(&s),
        }
    }

    pub fn format(self, n: f64) -> String {
        format_numero_locale(n, self)
    }
}

pub fn default_numero_locale() -> NumeroLocale {
    std::env::var("EXPRESSA_NUMEROS")
        .ok()
        .and_then(|s| NumeroLocale::from_name(&s))
        .unwrap_or(NumeroLocale::PtBr)
}

/// Programming-form (source-like): `3.14`, no thousands. Used in CSV and errors.
pub fn format_numero(n: f64) -> String {
    format_numero_locale(n, None)
}

fn format_numero_locale(n: f64, loc: impl Into<Option<NumeroLocale>>) -> String {
    if n.is_nan() {
        return "nan".to_string();
    }
    if n.is_infinite() {
        return if n.is_sign_positive() {
            "inf".to_string()
        } else {
            "-inf".to_string()
        };
    }
    let loc = loc.into();
    let neg = n < 0.0;
    let abs = n.abs();
    let integer = abs.fract() == 0.0 && abs < 1e15;
    let raw = if integer {
        format!("{}", abs as i64)
    } else {
        format!("{abs}")
    };
    let (int_part, frac) = match raw.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f)),
        None => (raw, None),
    };
    let (thou, dec) = match loc {
        Some(NumeroLocale::PtBr) => ('.', ','),
        Some(NumeroLocale::EnUs) => (',', '.'),
        None => {
            let body = match frac {
                Some(f) => format!("{int_part}.{f}"),
                None => int_part,
            };
            return if neg { format!("-{body}") } else { body };
        }
    };
    let grouped = group_thousands(&int_part, thou);
    let body = match frac {
        Some(f) => format!("{grouped}{dec}{f}"),
        None => grouped,
    };
    if neg { format!("-{body}") } else { body }
}

fn format_matriz(rows: &[Vec<f64>], loc: NumeroLocale) -> String {
    if rows.is_empty() {
        return "[]".to_string();
    }
    let cells: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(|n| loc.format(*n)).collect())
        .collect();
    let cols = cells[0].len();
    let mut widths = vec![0; cols];
    for row in &cells {
        for (j, c) in row.iter().enumerate() {
            if j < widths.len() {
                widths[j] = widths[j].max(c.len());
            }
        }
    }
    let mut lines = Vec::new();
    for row in &cells {
        let parts: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(j, c)| format!("{:>width$}", c, width = widths[j]))
            .collect();
        lines.push(format!("[ {} ]", parts.join("  ")));
    }
    lines.join("\n")
}

fn group_thousands(digits: &str, sep: char) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

fn parse_pt_br(s: &str) -> Option<f64> {
    let comma = s.chars().filter(|c| *c == ',').count();
    if comma > 1 {
        return None;
    }
    if comma == 1 {
        let (left, right) = s.split_once(',')?;
        if right.is_empty() || !right.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let left = strip_thousands(left, '.')?;
        format!("{left}.{right}").parse().ok()
    } else if looks_like_thousands(s, '.') {
        strip_thousands(s, '.')?.parse().ok()
    } else {
        // single `.` as decimal (paste from code) or plain integer
        let dots = s.chars().filter(|c| *c == '.').count();
        if dots > 1 {
            return None;
        }
        s.parse().ok()
    }
}

fn parse_en_us(s: &str) -> Option<f64> {
    let dots = s.chars().filter(|c| *c == '.').count();
    if dots > 1 {
        return None;
    }
    if s.contains(',') {
        let last_comma = s.rfind(',')?;
        if !s[last_comma + 1..].contains('.') && looks_like_thousands(s, ',') {
            return strip_thousands(s, ',')?.parse().ok();
        }
        let (left, right) = if let Some((l, r)) = s.split_once('.') {
            (l, Some(r))
        } else {
            (s, None)
        };
        let left = strip_thousands(left, ',')?;
        match right {
            Some(r) => format!("{left}.{r}").parse().ok(),
            None => left.parse().ok(),
        }
    } else {
        s.parse().ok()
    }
}

fn looks_like_thousands(s: &str, sep: char) -> bool {
    let t = s
        .strip_prefix('-')
        .unwrap_or(s)
        .strip_prefix('+')
        .unwrap_or(s);
    if !t.contains(sep) {
        return false;
    }
    let mut parts = t.split(sep);
    let first = parts.next().unwrap_or("");
    if first.is_empty() || !first.chars().all(|c| c.is_ascii_digit()) || first.len() > 3 {
        return false;
    }
    parts.all(|p| p.len() == 3 && p.chars().all(|c| c.is_ascii_digit()))
}

fn strip_thousands(s: &str, sep: char) -> Option<String> {
    if s.contains(sep) && !looks_like_thousands(s, sep) {
        return None;
    }
    let sign = if s.starts_with('-') { "-" } else { "" };
    let rest = s
        .strip_prefix('-')
        .or_else(|| s.strip_prefix('+'))
        .unwrap_or(s);
    Some(format!(
        "{sign}{}",
        rest.chars().filter(|c| *c != sep).collect::<String>()
    ))
}

fn escape_texto(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn parse_numero(raw: &str) -> Option<f64> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    cleaned.parse().ok()
}

impl MapKey {
    pub fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Texto(s) => Some(MapKey::Texto(s.clone())),
            Value::Bool(b) => Some(MapKey::Bool(*b)),
            Value::Numero(n) if n.is_finite() && n.fract() == 0.0 => {
                Some(MapKey::Numero(*n as i64))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_names_are_portuguese() {
        assert_eq!(Value::Nada.type_name(), "nada");
        assert_eq!(Value::Numero(1.0).type_name(), "numero");
        assert_eq!(Value::Texto("x".into()).type_name(), "texto");
        assert_eq!(Value::Bool(true).type_name(), "bool");
        assert_eq!(Value::lista(vec![]).type_name(), "lista");
        assert_eq!(Value::mapa(vec![]).type_name(), "mapa");
        assert_eq!(Value::Builtin("escreva").type_name(), "funcao");
    }

    #[test]
    fn display_matches_what_escreva_prints() {
        assert_eq!(Value::Nada.to_string(), "nada");
        assert_eq!(Value::Numero(10.0).to_string(), "10");
        assert_eq!(Value::Numero(7.5).to_string(), "7.5");
        assert_eq!(Value::Bool(true).to_string(), "verdadeiro");
        assert_eq!(Value::Bool(false).to_string(), "falso");
        assert_eq!(Value::Texto("oi".into()).to_string(), "oi");
        assert_eq!(
            Value::lista(vec![Value::Numero(1.0), Value::Texto("a".into())]).to_string(),
            "[1, \"a\"]"
        );
        assert_eq!(
            Value::mapa(vec![(MapKey::Texto("n".into()), Value::Numero(1.0))]).to_string(),
            "mapa{\"n\": 1}"
        );
        assert_eq!(Value::Builtin("leia").to_string(), "<funcao leia>");
    }

    #[test]
    fn format_numero_drops_trailing_point_on_integers() {
        assert_eq!(format_numero(0.0), "0");
        assert_eq!(format_numero(-3.0), "-3");
        assert_eq!(format_numero(1.25), "1.25");
        assert_eq!(format_numero(f64::NAN), "nan");
        assert_eq!(format_numero(f64::INFINITY), "inf");
    }

    #[test]
    fn pt_br_and_en_us_string_conversion() {
        let pt = NumeroLocale::PtBr;
        let en = NumeroLocale::EnUs;
        assert_eq!(pt.parse_texto("3,14"), Some(3.14));
        assert_eq!(pt.parse_texto("1.000"), Some(1000.0));
        assert_eq!(pt.parse_texto("1.000,5"), Some(1000.5));
        assert_eq!(pt.parse_texto("3.14"), Some(3.14));
        assert_eq!(en.parse_texto("1,000.5"), Some(1000.5));
        assert_eq!(en.parse_texto("1,000"), Some(1000.0));
        assert_eq!(en.parse_texto("3.14"), Some(3.14));
        assert_eq!(pt.format(1000.0), "1.000");
        assert_eq!(pt.format(7.3), "7,3");
        assert_eq!(en.format(1000.0), "1,000");
        assert_eq!(en.format(7.3), "7.3");
        assert_eq!(NumeroLocale::from_name("pt-br"), Some(pt));
        assert_eq!(NumeroLocale::from_name("en"), Some(en));
    }

    #[test]
    fn parse_numero_accepts_underscores() {
        assert_eq!(parse_numero("10"), Some(10.0));
        assert_eq!(parse_numero("1_000"), Some(1000.0));
        assert_eq!(parse_numero("3.14"), Some(3.14));
        assert_eq!(parse_numero("abc"), None);
    }

    #[test]
    fn equality_is_by_content_for_lists_and_identity_for_functions() {
        let a = Value::lista(vec![Value::Numero(1.0)]);
        let b = Value::lista(vec![Value::Numero(1.0)]);
        assert_eq!(a, b);

        assert_eq!(Value::Numero(1.0), Value::Numero(1.0));
        assert_ne!(Value::Numero(1.0), Value::Texto("1".into()));
        assert_eq!(Value::Builtin("escreva"), Value::Builtin("escreva"));
        assert_ne!(Value::Builtin("escreva"), Value::Builtin("leia"));
    }

    #[test]
    fn list_clone_shares_storage() {
        let a = Value::lista(vec![Value::Numero(1.0)]);
        let b = a.clone();
        if let Value::Lista(xs) = &a {
            xs.borrow_mut()[0] = Value::Numero(9.0);
        }
        assert_eq!(a, b);
        assert_eq!(b.to_string(), "[9]");
    }

    #[test]
    fn map_keys_are_text_bool_or_whole_number() {
        assert_eq!(
            MapKey::from_value(&Value::Texto("n".into())),
            Some(MapKey::Texto("n".into()))
        );
        assert_eq!(
            MapKey::from_value(&Value::Numero(3.0)),
            Some(MapKey::Numero(3))
        );
        assert_eq!(MapKey::from_value(&Value::Numero(1.5)), None);
        assert_eq!(MapKey::from_value(&Value::lista(vec![])), None);
        assert_eq!(
            MapKey::from_value(&Value::Bool(true)),
            Some(MapKey::Bool(true))
        );
    }
}
