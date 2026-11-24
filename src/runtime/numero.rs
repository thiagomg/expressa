//! Decimal-by-default numbers; `define_opcao(:tipo_numero, :float)` uses IEEE-754 for ops.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

/// How `+` `-` `*` `/` `%` run. Locale (`define_opcao(:formato_numero, :pt)`) is independent.
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

/// Finite decimal with up to 28 fractional places; arithmetic can round.
#[derive(Clone, Copy, Debug, Eq)]
pub struct Numero(Decimal);

impl Numero {
    pub(crate) fn exb_bytes(self) -> [u8; 16] {
        self.0.serialize()
    }

    pub(crate) fn from_exb_bytes(bytes: [u8; 16]) -> Option<Self> {
        let flags = u32::from_le_bytes(bytes[..4].try_into().unwrap());
        if flags & !0x80ff_0000 != 0 || (flags >> 16) & 0xff > 28 {
            return None;
        }
        Some(Self(Decimal::deserialize(bytes)))
    }

    pub fn from_i64(n: i64) -> Self {
        Self(Decimal::from(n))
    }

    pub fn from_usize(n: usize) -> Self {
        Self::from_u64(n as u64)
    }

    pub fn from_u64(n: u64) -> Self {
        Self(Decimal::from(n))
    }

    pub fn from_f64(n: f64) -> Option<Self> {
        if !n.is_finite() {
            return None;
        }
        // Shortest text when Decimal gives the same f64 back, so 0.1*3 prints
        // 0,30000000000000004. A text that rounds (0.3-0.1) keeps the extra
        // digits; the next float operation must see the original value.
        let texto = format!("{n}");
        if let Ok(d) = Decimal::from_str(&texto) {
            if n != 0.0 && d.is_zero() {
                return None;
            }
            if d.to_f64() == Some(n) {
                return Some(Self(d));
            }
        }
        let d = Decimal::from_f64_retain(n)?;
        if n != 0.0 && d.is_zero() {
            return None;
        }
        Some(Self(d))
    }

    /// A result of binary floating point (`mat`, `matriz`) as a decimal.
    /// When the value is only binary noise around a number with at most 15
    /// significant digits (a few units in the last place away), that short
    /// number is used: `seno(radianos(30))` is 0.5, not 0.49999999999999994.
    /// Any other value keeps every digit (`raiz(2)`), so feeding it back into
    /// another function does not lose precision.
    pub fn from_f64_15(n: f64) -> Option<Self> {
        if !n.is_finite() {
            return None;
        }
        if n == 0.0 {
            return Some(Self(Decimal::ZERO));
        }
        if let Ok(d) = Decimal::from_scientific(&format!("{n:.14e}"))
            && !d.is_zero()
            && let Some(volta) = d.to_f64()
            && (volta - n).abs() <= 4.0 * f64::EPSILON * n.abs()
        {
            return Some(Self(d.normalize()));
        }
        Self::from_f64(n)
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

    pub fn floor(self) -> Self {
        Self(self.0.floor())
    }

    pub fn ceil(self) -> Self {
        Self(self.0.ceil())
    }

    /// Nearest integer, with halfway cases away from zero (as f64::round).
    pub fn round(self) -> Self {
        Self(
            self.0
                .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero),
        )
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
        Some(self.cmp(other))
    }
}

impl std::ops::Neg for Numero {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
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

impl TryFrom<f64> for Numero {
    type Error = &'static str;

    fn try_from(n: f64) -> Result<Self, Self::Error> {
        Self::from_f64(n).ok_or("número fora do intervalo")
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
    if casas_alem_da_escala(&cleaned) {
        return None;
    }
    cleaned.parse().ok()
}

/// Decimal keeps 28 places. Extra non-zero places would round or become 0.
fn casas_alem_da_escala(s: &str) -> bool {
    let Some((_, frac)) = s.split_once('.') else {
        return false;
    };
    let frac = frac.split(['e', 'E']).next().unwrap_or(frac);
    frac.trim_end_matches('0').len() > 28
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Numero {
        parse_numero(s).unwrap()
    }

    #[test]
    fn parse_numero_rejects_scale_past_28() {
        assert!(parse_numero("0.00000000000000000000000000001").is_none());
        assert!(parse_numero("1.00000000000000000000000000001").is_none());
        assert!(parse_numero("1.0000000000000000000000000000").is_some());
    }

    #[test]
    fn from_f64_uses_the_shortest_round_trip() {
        let n = Numero::from_f64(0.1 * 3.0).unwrap();
        assert_eq!(n.to_plain_string(), "0.30000000000000004");
        assert_eq!(n.to_f64().to_bits(), (0.1_f64 * 3.0).to_bits());
        assert!(Numero::from_f64(1e-32).is_none());
    }

    #[test]
    fn from_f64_15_hides_binary_noise() {
        let s = |n: f64| Numero::from_f64_15(n).unwrap().to_plain_string();
        assert_eq!(s(0.1 * 3.0), "0.3");
        assert_eq!(s((30.0_f64).to_radians().sin()), "0.5");
        assert_eq!(s(2.0_f64.sqrt()), "1.4142135623730951");
        assert_eq!(s((60.0_f64).to_radians().cos()), "0.5");
        assert_eq!(s(-1234.5), "-1234.5");
        assert_eq!(s(1e20), "100000000000000000000");
        assert!(Numero::from_f64_15(f64::NAN).is_none());
    }

    #[test]
    fn from_f64_keeps_a_value_whose_short_text_rounds() {
        for n in [0.3 - 0.1, 0.3 / 3.0, 0.1 / 3.0, 0.2 + 0.01] {
            let stored = Numero::from_f64(n).unwrap();
            assert_eq!(stored.to_f64().to_bits(), n.to_bits(), "{n}");
        }
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
        assert_eq!(n("0.1").add(n("0.2"), mode).unwrap(), n("0.3"));
    }

    #[test]
    fn float_mode_uses_ieee754() {
        let mode = NumeroAritmetica::Float;
        let s = n("0.1").add(n("0.2"), mode).unwrap();
        assert_ne!(s, n("0.3"));
        assert!(n("1").div(n("0"), mode).is_err());
    }
}
