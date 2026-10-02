//! `formate(modelo, valores…)` — Rust-style `{}` / `{:<n}` / `{:^n}` / `{:>n}`.

use super::value::{NumeroLocale, Value};

#[derive(Clone, Copy)]
enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Indexing {
    Auto,
    Manual,
}

pub(crate) fn formate(template: &str, args: &[Value], loc: NumeroLocale) -> Result<String, String> {
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    let mut out = String::new();
    let mut auto_next = 0usize;
    let mut indexing: Option<Indexing> = None;
    let mut used = vec![false; args.len()];

    while i < chars.len() {
        match chars[i] {
            '{' => {
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    out.push('{');
                    i += 2;
                    continue;
                }
                i += 1;
                let spec_start = i;
                while i < chars.len() && chars[i] != '}' {
                    if chars[i] == '{' {
                        return Err("chave '{' sem fechar no modelo".into());
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("chave '{' sem fechar no modelo".into());
                }
                let inner: String = chars[spec_start..i].iter().collect();
                i += 1; // skip '}'
                let (idx, piece) =
                    render_placeholder(&inner, args, loc, &mut auto_next, &mut indexing)?;
                if idx >= args.len() {
                    return Err(format!(
                        "índice {} fora do intervalo (há {} valor(es))",
                        idx + 1,
                        args.len()
                    ));
                }
                used[idx] = true;
                out.push_str(&piece);
            }
            '}' => {
                if i + 1 < chars.len() && chars[i + 1] == '}' {
                    out.push('}');
                    i += 2;
                    continue;
                }
                return Err("chave '}' sem abrir no modelo (use }} para escrever })".into());
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }

    if let Some(i) = used.iter().position(|u| !*u) {
        return Err(format!(
            "valor extra na posição {} (o modelo não usa)",
            i + 1
        ));
    }
    Ok(out)
}

fn render_placeholder(
    inner: &str,
    args: &[Value],
    loc: NumeroLocale,
    auto_next: &mut usize,
    indexing: &mut Option<Indexing>,
) -> Result<(usize, String), String> {
    let (idx_part, spec_part) = match inner.find(':') {
        Some(colon) => (&inner[..colon], Some(&inner[colon + 1..])),
        None => (inner, None),
    };

    let idx = if idx_part.is_empty() {
        set_indexing(indexing, Indexing::Auto)?;
        let idx = *auto_next;
        *auto_next += 1;
        if idx >= args.len() {
            return Err(format!(
                "modelo espera pelo menos {} valor(es), recebeu {}",
                idx + 1,
                args.len()
            ));
        }
        idx
    } else {
        set_indexing(indexing, Indexing::Manual)?;
        parse_manual_index(idx_part)?
    };

    if idx >= args.len() {
        return Err(format!(
            "índice {} fora do intervalo (há {} valor(es))",
            idx + 1,
            args.len()
        ));
    }

    let value = &args[idx];
    let text = value.format_with(loc);
    let formatted = match spec_part {
        None | Some("") => text,
        Some(spec) => apply_spec(spec, &text, value)?,
    };
    Ok((idx, formatted))
}

fn set_indexing(slot: &mut Option<Indexing>, got: Indexing) -> Result<(), String> {
    match *slot {
        None => *slot = Some(got),
        Some(prev) if prev == got => {}
        Some(_) => {
            return Err("não misture {} e {1} no mesmo modelo".into());
        }
    }
    Ok(())
}

fn parse_manual_index(s: &str) -> Result<usize, String> {
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("especificação inválida `{{{s}}}`"));
    }
    let n: usize = s
        .parse()
        .map_err(|_| format!("especificação inválida `{{{s}}}`"))?;
    if n < 1 {
        return Err("índice 0 deve ser >= 1 (o primeiro valor é {1})".into());
    }
    Ok(n - 1)
}

fn apply_spec(spec: &str, text: &str, value: &Value) -> Result<String, String> {
    let chars: Vec<char> = spec.chars().collect();
    if chars.is_empty() {
        return Ok(text.to_string());
    }

    let (fill, align, width_chars) = if is_align(chars[0]) {
        (' ', parse_align(chars[0]), &chars[1..])
    } else if chars.len() >= 2 && is_align(chars[1]) {
        (chars[0], parse_align(chars[1]), &chars[2..])
    } else {
        let default = match value {
            Value::Numero(_) => Align::Right,
            _ => Align::Left,
        };
        (' ', default, chars.as_slice())
    };

    let width = if width_chars.is_empty() {
        0
    } else if width_chars.iter().all(|c| c.is_ascii_digit()) {
        width_chars.iter().collect::<String>().parse().unwrap_or(0)
    } else {
        return Err(format!("especificação inválida `:{}`", spec));
    };

    Ok(pad(text, width, align, fill))
}

fn is_align(c: char) -> bool {
    matches!(c, '<' | '^' | '>')
}

fn parse_align(c: char) -> Align {
    match c {
        '<' => Align::Left,
        '^' => Align::Center,
        '>' => Align::Right,
        _ => Align::Left,
    }
}

fn pad(text: &str, width: usize, align: Align, fill: char) -> String {
    let n = text.chars().count();
    if n >= width {
        return text.to_string();
    }
    let extra = width - n;
    match align {
        Align::Left => {
            let mut s = text.to_string();
            s.extend(std::iter::repeat_n(fill, extra));
            s
        }
        Align::Right => {
            let mut s: String = std::iter::repeat_n(fill, extra).collect();
            s.push_str(text);
            s
        }
        Align::Center => {
            let left = extra / 2;
            let right = extra - left;
            let mut s: String = std::iter::repeat_n(fill, left).collect();
            s.push_str(text);
            s.extend(std::iter::repeat_n(fill, right));
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::value::Value;

    fn t(s: &str) -> Value {
        Value::Texto(s.into())
    }
    fn n(x: f64) -> Value {
        Value::Numero(x)
    }

    fn f(template: &str, args: &[Value]) -> String {
        formate(template, args, NumeroLocale::PtBr).unwrap()
    }

    fn err(template: &str, args: &[Value]) -> String {
        formate(template, args, NumeroLocale::PtBr).unwrap_err()
    }

    #[test]
    fn rust_alignment_examples() {
        assert_eq!(f("Hello {:<5}!", &[t("x")]), "Hello x    !");
        assert_eq!(f("Hello {:-<5}!", &[t("x")]), "Hello x----!");
        assert_eq!(f("Hello {:^5}!", &[t("x")]), "Hello   x  !");
        assert_eq!(f("Hello {:>5}!", &[t("x")]), "Hello     x!");
    }

    #[test]
    fn sequential_and_literal_braces() {
        assert_eq!(f("{}-{}", &[t("a"), t("b")]), "a-b");
        assert_eq!(f("{{ok}}", &[]), "{ok}");
        assert_eq!(f("só texto", &[]), "só texto");
    }

    #[test]
    fn one_based_manual_index() {
        assert_eq!(f("{1} {1}", &[t("x")]), "x x");
        assert_eq!(f("{2} {1}", &[t("a"), t("b")]), "b a");
    }

    #[test]
    fn default_align_text_left_number_right() {
        assert_eq!(f("{:5}", &[t("x")]), "x    ");
        assert_eq!(f("{:5}", &[n(7.0)]), "    7");
    }

    #[test]
    fn unicode_width() {
        assert_eq!(f("{:<5}", &[t("olá")]), "olá  ");
    }

    #[test]
    fn longer_than_width_stays_whole() {
        assert_eq!(f("{:>3}", &[t("abcd")]), "abcd");
    }

    #[test]
    fn errors() {
        assert!(err("{", &[]).contains("sem fechar"));
        assert!(err("}", &[]).contains("sem abrir"));
        assert!(err("{}", &[]).contains("espera pelo menos 1"));
        assert!(err("{}", &[t("a"), t("b")]).contains("valor extra"));
        assert!(err("{0}", &[t("a")]).contains(">= 1"));
        assert!(err("{} {1}", &[t("a")]).contains("não misture"));
        assert!(err("{:z}", &[t("a")]).contains("inválida"));
    }
}
