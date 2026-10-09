//! Decimal-by-default numbers; `formato("float")` uses IEEE-754 for ops.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// How `+` `-` `*` `/` `%` run. Locale (`formato("pt")`) is independent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumeroAritmetica {
    /// Base 10. `0.1 + 0.2` is `0.3`. Default.
    #[default]
    Decimal,
    /// Binary `f64`. Same rounding as Python/JavaScript.
    Float,
}

impl NumeroAritmetica {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "decimal" | "exato" => Some(Self::Decimal),
            "float" | "flutuante" | "binario" | "binário" => Some(Self::Float),
            _ => None,
        }
    }
}

pub fn default_numero_aritmetica() -> NumeroAritmetica {
    std::env::var("EXPRESSA_ARITMETICA")
        .ok()
        .and_then(|s| NumeroAritmetica::from_name(&s))
        .unwrap_or(NumeroAritmetica::Decimal)
}

/// Exact decimal (28 digits). Classroom default so money sums print right.
#[derive(Clone, Copy, Debug, Eq)]
pub struct Numero(Decimal);

impl Numero {
    pub fn from_i64(n: i64) -> Self {
        Self(Decimal::from(n))
    }

    pub fn from_usize(n: usize) -> Self {
        Self(Decimal::from(n as u64))
    }

    pub fn from_f64(n: f64) -> Option<Self> {
        Decimal::from_f64_retain(n).map(Self)
    }

    pub fn to_f64(self) -> f64 {
        self.0.to_f64().unwrap_or(f64::NAN)
    }

    pub fn is_integer(self) -> bool {
        self.0.is_integer()
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn to_i64(self) -> Option<i64> {
        if !self.0.is_integer() {
            return None;
        }
        self.0.to_i64()
    }

    pub fn normalize(self) -> Self {
        Self(self.0.normalize())
    }

    /// Source-like text (`3.14`, no thousands) after stripping trailing zeros.
    pub fn to_plain_string(self) -> String {
        let n = self.0.normalize();
        if n.is_integer() {
            match n.to_i64() {
                Some(i) => i.to_string(),
                None => n.to_string(),
            }
        } else {
            n.to_string()
        }
    }

    pub fn add(self, rhs: Self, mode: NumeroAritmetica) -> Result<Self, &'static str> {
        self.bin(rhs, mode, |a, b| a.checked_add(b), |a, b| a + b)
    }

    pub fn sub(self, rhs: Self, mode: NumeroAritmetica) -> Result<Self, &'static str> {
        self.bin(rhs, mode, |a, b| a.checked_sub(b), |a, b| a - b)
    }

    pub fn mul(self, rhs: Self, mode: NumeroAritmetica) -> Result<Self, &'static str> {
        self.bin(rhs, mode, |a, b| a.checked_mul(b), |a, b| a * b)
    }

    pub fn div(self, rhs: Self, mode: NumeroAritmetica) -> Result<Self, &'static str> {
        if rhs.is_zero() {
            return Err("divisão por zero");
        }
        self.bin(rhs, mode, |a, b| a.checked_div(b), |a, b| a / b)
    }

    pub fn rem(self, rhs: Self, mode: NumeroAritmetica) -> Result<Self, &'static str> {
        if rhs.is_zero() {
            return Err("resto de divisão por zero");
        }
        self.bin(rhs, mode, |a, b| a.checked_rem(b), |a, b| a % b)
    }

    pub fn neg(self) -> Self {
        Self(-self.0)
    }

    fn bin(
        self,
        rhs: Self,
        mode: NumeroAritmetica,
        dec: impl FnOnce(Decimal, Decimal) -> Option<Decimal>,
        flt: impl FnOnce(f64, f64) -> f64,
    ) -> Result<Self, &'static str> {
        match mode {
            NumeroAritmetica::Decimal => dec(self.0, rhs.0)
                .map(Self)
                .ok_or("número fora do intervalo"),
            NumeroAritmetica::Float => {
                let x = flt(self.to_f64(), rhs.to_f64());
                if !x.is_finite() {
                    return Err("número fora do intervalo");
                }
                Self::from_f64(x).ok_or("número fora do intervalo")
            }
        }
    }
}

impl PartialEq for Numero {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for Numero {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for Numero {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<i64> for Numero {
    fn from(n: i64) -> Self {
        Self::from_i64(n)
    }
}

impl From<i32> for Numero {
    fn from(n: i32) -> Self {
        Self::from_i64(n as i64)
    }
}

impl From<u32> for Numero {
    fn from(n: u32) -> Self {
        Self::from_i64(n as i64)
    }
}

impl From<usize> for Numero {
    fn from(n: usize) -> Self {
        Self::from_usize(n)
    }
}

impl From<f64> for Numero {
    fn from(n: f64) -> Self {
        Self::from_f64(n).unwrap_or(Self(Decimal::ZERO))
    }
}

impl FromStr for Numero {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        Decimal::from_str(s).map(Self).map_err(|_| ())
    }
}

impl fmt::Display for Numero {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_plain_string())
    }
}

pub fn parse_numero(raw: &str) -> Option<Numero> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Numero {
        parse_numero(s).unwrap()
    }

    #[test]
    fn from_name_aliases() {
        assert_eq!(
            NumeroAritmetica::from_name("decimal"),
            Some(NumeroAritmetica::Decimal)
        );
        assert_eq!(
            NumeroAritmetica::from_name("exato"),
            Some(NumeroAritmetica::Decimal)
        );
        assert_eq!(
            NumeroAritmetica::from_name("float"),
            Some(NumeroAritmetica::Float)
        );
        assert_eq!(
            NumeroAritmetica::from_name("flutuante"),
            Some(NumeroAritmetica::Float)
        );
        assert_eq!(
            NumeroAritmetica::from_name("binário"),
            Some(NumeroAritmetica::Float)
        );
        assert_eq!(NumeroAritmetica::from_name("pt"), None);
    }

    #[test]
    fn decimal_keeps_money_and_tenths() {
        let mode = NumeroAritmetica::Decimal;
        let s = n("35.87")
            .add(n("12.63"), mode)
            .unwrap()
            .add(n("11.05"), mode)
            .unwrap()
            .add(n("11.47"), mode)
            .unwrap()
            .add(n("10.35"), mode)
            .unwrap();
        assert_eq!(s.to_plain_string(), "81.37");
        assert_eq!(
            n("0.1").add(n("0.2"), mode).unwrap(),
            n("0.3")
        );
    }

    #[test]
    fn float_mode_uses_ieee754() {
        let mode = NumeroAritmetica::Float;
        let s = n("0.1").add(n("0.2"), mode).unwrap();
        assert_ne!(s, n("0.3"));
        assert!(n("1").div(n("0"), mode).is_err());
    }
}
