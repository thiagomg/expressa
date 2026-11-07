//! Keywords and string/comment blanking, shared with indent and search.

#[allow(dead_code)]
pub const KEYWORDS: &[&str] = &[
    "se",
    "senao",
    "senão",
    "ou",
    "e",
    "nao",
    "não",
    "inicio",
    "início",
    "fim",
    "funcao",
    "função",
    "para",
    "de",
    "ate",
    "até",
    "em",
    "repita",
    "enquanto",
    "vezes",
    "importe",
    "contem",
    "contém",
    "verdadeiro",
    "falso",
    "se_falhar",
    "retorne",
    "pare",
    "continue",
    "continua",
];

/// Replace comments and string bodies with spaces (newlines kept).
pub fn blank_comments_and_strings(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let blank = |out: &mut String, s: &str| {
        for c in s.chars() {
            if c == '\n' {
                out.push('\n');
            } else {
                out.extend(std::iter::repeat_n(' ', c.len_utf8()));
            }
        }
    };
    let b = text.as_bytes();
    let mut i = 0;
    if text.starts_with("#!") {
        let end = text.find('\n').unwrap_or(text.len());
        blank(&mut out, &text[..end]);
        i = end;
    }
    while i < text.len() {
        if text[i..].starts_with("//") {
            let end = text[i..].find('\n').map_or(text.len(), |j| i + j);
            blank(&mut out, &text[i..end]);
            i = end;
        } else if text[i..].starts_with("/*") {
            let end = text[i + 2..]
                .find("*/")
                .map_or(text.len(), |j| i + 2 + j + 2);
            blank(&mut out, &text[i..end]);
            i = end;
        } else if b[i] == b'"' {
            out.push('"');
            let mut j = i + 1;
            while j < text.len() && b[j] != b'"' && b[j] != b'\n' {
                j += if b[j] == b'\\' && j + 1 < text.len() && b[j + 1] != b'\n' {
                    1 + text[j + 1..].chars().next().map_or(1, char::len_utf8)
                } else {
                    text[j..].chars().next().map_or(1, char::len_utf8)
                };
            }
            let j = j.min(text.len());
            blank(&mut out, &text[i + 1..j]);
            if j < text.len() && b[j] == b'"' {
                out.push('"');
                i = j + 1;
            } else {
                i = j;
            }
        } else {
            let c = text[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palavras_com_acento() {
        assert!(KEYWORDS.contains(&"função"));
        assert!(KEYWORDS.contains(&"não"));
        assert!(KEYWORDS.contains(&"início"));
        assert!(KEYWORDS.contains(&"senão"));
        assert_eq!(KEYWORDS.len(), 30);
    }

    #[test]
    fn strings_nao_contam_como_codigo() {
        let s = blank_comments_and_strings(r#"escreva("início: ")"#);
        assert!(!s.contains("início"));
        assert!(s.contains("escreva"));
    }

    #[test]
    fn gramatica_tem_as_mesmas_palavras() {
        let grammar = include_str!("../../tree-sitter-expressa/grammar.js");
        for k in KEYWORDS {
            assert!(grammar.contains(&format!("\"{k}\"")), "{k}");
        }
    }
}
