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

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nada => "nada",
            Value::Numero(_) => "numero",
            Value::Texto(_) => "texto",
            Value::Bool(_) => "bool",
            Value::Lista(_) => "lista",
            Value::Mapa(_) => "mapa",
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

pub fn format_numero(n: f64) -> String {
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
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let s = format!("{n}");
        s
    }
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
