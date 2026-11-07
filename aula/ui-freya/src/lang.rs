//! Keywords and string/comment blanking, shared with indent and search.

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

pub fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

pub fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_ident(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(is_ident_start) && cs.all(is_ident_char)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Code,
    Text,
    LineComment,
    BlockComment,
}

/// Lexical state at the end of `text` (the cursor).
pub fn state_at_end(text: &str) -> State {
    let b = text.as_bytes();
    let at = |i: usize, s: &[u8]| b[i..].starts_with(s);
    let mut i = 0;
    if text.starts_with("#!") {
        match text.find('\n') {
            Some(n) => i = n,
            None => return State::LineComment,
        }
    }
    let mut st = State::Code;
    while i < b.len() {
        match st {
            State::Code => {
                if at(i, b"//") {
                    st = State::LineComment;
                    i += 1;
                } else if at(i, b"/*") {
                    st = State::BlockComment;
                    i += 1;
                } else if b[i] == b'"' {
                    st = State::Text;
                }
            }
            State::Text => {
                if b[i] == b'\\' {
                    i += 1;
                } else if b[i] == b'"' || b[i] == b'\n' {
                    st = State::Code;
                }
            }
            State::LineComment => {
                if b[i] == b'\n' {
                    st = State::Code;
                }
            }
            State::BlockComment => {
                if at(i, b"*/") {
                    st = State::Code;
                    i += 1;
                }
            }
        }
        i += 1;
    }
    st
}

/// Identifier ending exactly at byte `end` of `s`, as (start, name).
fn ident_before(s: &str, end: usize) -> Option<(usize, &str)> {
    let head = &s[..end];
    let start = head
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map(|(i, _)| i)?;
    let name = &head[start..];
    is_ident(name).then_some((start, name))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completion {
    Nothing,
    Import { prefix: String },
    Module { alias: String, prefix: String },
    Method { prefix: String },
    Name { prefix: String },
}

/// What to complete at the cursor; `before` is the text up to it.
pub fn completion_context(before: &str) -> Completion {
    let line = &before[before.rfind('\n').map_or(0, |i| i + 1)..];
    match state_at_end(before) {
        State::Text => {
            let t = line.trim_end_matches(|c: char| c != '"');
            let prefix = &line[t.len()..];
            let head = t[..t.len().saturating_sub(1)].trim_end();
            return if head.ends_with("importe") {
                Completion::Import {
                    prefix: prefix.to_string(),
                }
            } else {
                Completion::Nothing
            };
        }
        State::LineComment | State::BlockComment => return Completion::Nothing,
        State::Code => {}
    }
    let code = blank_comments_and_strings(line);
    let run = code.len() - code.trim_end_matches(is_ident_char).len();
    if code[code.len() - run..].starts_with(|c: char| c.is_ascii_digit()) {
        return Completion::Nothing;
    }
    let (pstart, prefix) = ident_before(&code, code.len()).unwrap_or((code.len(), ""));
    let head = &code[..pstart];
    if let Some(h) = head.strip_suffix("::") {
        if let Some((_, alias)) = ident_before(h, h.len()) {
            return Completion::Module {
                alias: alias.to_string(),
                prefix: prefix.to_string(),
            };
        }
        return Completion::Nothing;
    }
    if head.ends_with(':') {
        return Completion::Nothing;
    }
    if let Some(h) = head.strip_suffix('.') {
        let receiver_ok = h
            .chars()
            .last()
            .is_some_and(|c| is_ident_char(c) || matches!(c, ')' | ']' | '"'));
        let number = ident_before(h, h.len())
            .is_some_and(|(_, w)| w.chars().all(|c| c.is_ascii_digit() || c == '_'))
            || (h.chars().last().is_some_and(|c| c.is_ascii_digit())
                && ident_before(h, h.len()).is_none());
        if receiver_ok && !number {
            return Completion::Method {
                prefix: prefix.to_string(),
            };
        }
    }
    Completion::Name {
        prefix: prefix.to_string(),
    }
}

pub fn prefixo_do_contexto(ctx: &Completion) -> &str {
    match ctx {
        Completion::Import { prefix }
        | Completion::Module { prefix, .. }
        | Completion::Method { prefix }
        | Completion::Name { prefix } => prefix,
        Completion::Nothing => "",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub name: String,
    pub alias: Option<String>,
    pub method: bool,
    pub active: usize,
}

/// The call whose parentheses contain the cursor.
pub fn call_context(before: &str) -> Option<Call> {
    let code = blank_comments_and_strings(before);
    let b = code.as_bytes();
    let mut depth = 0usize;
    let mut commas = 0usize;
    let mut i = b.len();
    while i > 0 {
        i -= 1;
        match b[i] {
            b')' | b']' | b'}' => depth += 1,
            b'[' | b'{' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            b'(' => {
                if depth > 0 {
                    depth -= 1;
                    continue;
                }
                let head = code[..i].trim_end_matches([' ', '\t']);
                let (start, name) = ident_before(head, head.len())?;
                if KEYWORDS.contains(&name) {
                    return None;
                }
                let lead = &head[..start];
                let (alias, lead) = match lead.strip_suffix("::") {
                    Some(l) => match ident_before(l, l.len()) {
                        Some((s, a)) => (Some(a.to_string()), &l[..s]),
                        None => (None, lead),
                    },
                    None => (None, lead),
                };
                return Some(Call {
                    name: name.to_string(),
                    alias,
                    method: lead.trim_end().ends_with('.'),
                    active: commas,
                });
            }
            b',' if depth == 0 => commas += 1,
            b'\n' if i > 0 && b[i - 1] == b'\n' => return None,
            _ => {}
        }
    }
    None
}

/// Parameter names of a signature: `pinte(texto, frente) -> texto`.
pub fn signature_params(sig: &str) -> Vec<String> {
    let (Some(a), Some(z)) = (sig.find('('), sig.find(')')) else {
        return Vec::new();
    };
    let inside = sig[a + 1..z].trim();
    if inside.is_empty() {
        return Vec::new();
    }
    inside.split(',').map(|p| p.trim().to_string()).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub alias: Option<String>,
    pub spec: String,
    pub line: usize,
}

/// `importe "x"` and `alias = importe "x"`, in order.
pub fn parse_imports(text: &str) -> Vec<Import> {
    let code = blank_comments_and_strings(text);
    let mut out = Vec::new();
    for (line_no, (code_line, line)) in code.split('\n').zip(text.split('\n')).enumerate() {
        let mut rest = code_line.trim_start();
        let mut alias = None;
        if let Some(eq) = rest.find('=') {
            let lhs = rest[..eq].trim();
            let after = rest[eq + 1..].trim_start();
            if is_ident(lhs) && !after.starts_with('=') && after.starts_with("importe") {
                alias = Some(lhs.to_string());
                rest = after;
            }
        }
        let Some(after) = rest.strip_prefix("importe") else {
            continue;
        };
        if after.starts_with(is_ident_char) {
            continue;
        }
        let after = after.trim_start();
        if !after.starts_with('"') {
            continue;
        }
        let open = code_line.len() - after.len() + 1;
        let Some(close) = line[open..].find('"') else {
            continue;
        };
        out.push(Import {
            alias,
            spec: line[open..open + close].to_string(),
            line: line_no,
        });
    }
    out
}

/// Line where a new `importe` goes: after the last one, else after `#!`.
pub fn import_insert_line(text: &str) -> usize {
    match parse_imports(text).last() {
        Some(i) => i.line + 1,
        None => usize::from(text.starts_with("#!")),
    }
}

pub fn modules() -> Vec<&'static str> {
    use expressa::runtime::NATIVAS;
    let mut m: Vec<&str> = NATIVAS.iter().filter_map(|n| n.module).collect();
    m.sort_unstable();
    m.dedup();
    m
}

/// Native module for `spec`, or `None` when it names a `.lep` file.
pub fn native_module(spec: &str) -> Option<&'static str> {
    if spec.contains('/') || spec.ends_with(".lep") {
        return None;
    }
    modules().into_iter().find(|m| *m == spec)
}

/// Project-relative path of a file `importe` from the file `from`.
pub fn resolve_import(spec: &str, from: &str) -> Option<String> {
    let mut spec = spec.to_string();
    if !spec.rsplit('/').next().unwrap_or("").contains('.') {
        spec.push_str(".lep");
    }
    let mut parts: Vec<&str> = if spec.starts_with('/') {
        Vec::new()
    } else {
        from.split('/').collect()
    };
    parts.pop();
    for p in spec.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefKind {
    Function(Vec<String>),
    Value,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def {
    pub name: String,
    pub kind: DefKind,
    pub line: usize,
    pub top_level: bool,
    pub doc: String,
}

impl Def {
    pub fn signature(&self) -> String {
        match &self.kind {
            DefKind::Function(p) => format!("{}({})", self.name, p.join(", ")),
            _ => self.name.clone(),
        }
    }
}

/// Functions (`f = funcao(a, b)`), variables, modules and loop variables.
pub fn parse_definitions(text: &str) -> Vec<Def> {
    let code = blank_comments_and_strings(text);
    let orig: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<Def> = Vec::new();
    let mut pending_doc: Vec<String> = Vec::new();
    let mut add = |def: Def| {
        if !KEYWORDS.contains(&def.name.as_str()) && !out.iter().any(|d| d.name == def.name) {
            out.push(def);
        }
    };
    for (line_no, line) in code.split('\n').enumerate() {
        let raw = orig.get(line_no).copied().unwrap_or("");
        let raw_trim = raw.trim_start();
        if raw_trim.starts_with("///") && !raw_trim.starts_with("////") {
            let body = raw_trim.trim_start_matches('/').trim_start();
            pending_doc.push(body.to_string());
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let body = line.trim_start();
        let name_end = body.find(|c: char| !is_ident_char(c)).unwrap_or(body.len());
        let name = &body[..name_end];
        let after = body[name_end..].trim_start();
        if is_ident(name) && after.starts_with('=') && !after.starts_with("==") {
            let rhs = after[1..].trim_start();
            let kind = if let Some(rest) = rhs
                .strip_prefix("funcao")
                .or_else(|| rhs.strip_prefix("função"))
                .filter(|r| r.trim_start().starts_with('('))
            {
                let rest = rest.trim_start();
                let close = rest.find(')').unwrap_or(rest.len());
                DefKind::Function(
                    rest[1..close]
                        .split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect(),
                )
            } else if rhs.starts_with("importe") {
                DefKind::Module
            } else {
                DefKind::Value
            };
            add(Def {
                name: name.to_string(),
                kind,
                line: line_no,
                top_level: indent == 0,
                doc: pending_doc.join("\n"),
            });
            pending_doc.clear();
        } else if !raw_trim.is_empty() && !raw_trim.starts_with("//") && !raw_trim.starts_with("/*")
        {
            pending_doc.clear();
        }
        let mut rest = line;
        while let Some(i) = rest.find("para") {
            let before_ok = rest[..i].chars().last().is_none_or(|c| !is_ident_char(c));
            let tail = &rest[i + 4..];
            rest = tail;
            if !before_ok || !tail.starts_with(char::is_whitespace) {
                continue;
            }
            let tail = tail.trim_start();
            let end = tail.find(|c: char| !is_ident_char(c)).unwrap_or(tail.len());
            let var = &tail[..end];
            let next = tail[end..].trim_start();
            let kw = next.split(|c: char| !is_ident_char(c)).next().unwrap_or("");
            if is_ident(var) && (kw == "de" || kw == "em") {
                add(Def {
                    name: var.to_string(),
                    kind: DefKind::Value,
                    line: line_no,
                    top_level: false,
                    doc: String::new(),
                });
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub alias: Option<String>,
    pub name: String,
    pub start: usize,
    pub end: usize,
}

/// Identifier (with an optional `alias::`) at byte `col` of `line`.
pub fn word_at(line: &str, col: usize) -> Option<Word> {
    let col = col.min(line.len());
    let col = (0..=col).rev().find(|&c| line.is_char_boundary(c))?;
    let start = line[..col]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map_or(col, |(i, _)| i);
    let end = line[col..]
        .char_indices()
        .find(|(_, c)| !is_ident_char(*c))
        .map_or(line.len(), |(i, _)| col + i);
    let name = &line[start..end];
    if !is_ident(name) {
        return None;
    }
    let alias = line[..start]
        .strip_suffix("::")
        .and_then(|h| ident_before(h, h.len()))
        .map(|(_, a)| a.to_string());
    Some(Word {
        alias,
        name: name.to_string(),
        start,
        end,
    })
}

/// Line containing `byte`, plus the column inside that line.
pub fn linha_no_byte(texto: &str, byte: usize) -> (usize, &str, usize) {
    let byte = byte.min(texto.len());
    let start = texto[..byte].rfind('\n').map_or(0, |i| i + 1);
    let end = texto[byte..].find('\n').map_or(texto.len(), |i| byte + i);
    let idx = texto[..start].bytes().filter(|&b| b == b'\n').count();
    (idx, &texto[start..end], byte - start)
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

    #[test]
    fn contexto_de_completar() {
        let c = completion_context;
        let name = |p: &str| Completion::Name { prefix: p.into() };
        assert_eq!(
            c("importe \"ma"),
            Completion::Import {
                prefix: "ma".into()
            }
        );
        assert_eq!(
            c("importe \"lib/"),
            Completion::Import {
                prefix: "lib/".into()
            }
        );
        assert_eq!(c("x = \"texto"), Completion::Nothing);
        assert_eq!(
            c("x = m::ra"),
            Completion::Module {
                alias: "m".into(),
                prefix: "ra".into()
            }
        );
        assert_eq!(
            c("m::"),
            Completion::Module {
                alias: "m".into(),
                prefix: "".into()
            }
        );
        assert_eq!(
            c("lista.ta"),
            Completion::Method {
                prefix: "ta".into()
            }
        );
        assert_eq!(c("y = 3"), Completion::Nothing);
        assert_eq!(c("esc"), name("esc"));
        assert_eq!(c("x // esc"), Completion::Nothing);
    }

    #[test]
    fn palavra_qualificada() {
        assert_eq!(
            word_at("x = m::raiz(4)", 9),
            Some(Word {
                alias: Some("m".into()),
                name: "raiz".into(),
                start: 7,
                end: 11
            })
        );
        assert_eq!(word_at("a + b", 2), None);
        assert_eq!(
            signature_params("pinte(texto, frente, fundo) -> texto"),
            ["texto", "frente", "fundo"]
        );
    }

    #[test]
    fn chamada_ativa() {
        let c = call_context("escreva(1, pinte(\"a\", ");
        assert_eq!(c.as_ref().map(|x| x.name.as_str()), Some("pinte"));
        assert_eq!(c.map(|x| x.active), Some(1));
        assert_eq!(
            call_context("m::raiz(").map(|x| (x.alias, x.name)),
            Some((Some("m".into()), "raiz".into()))
        );
    }

    #[test]
    fn imports_e_defs() {
        let src = "importe \"tela\"\nsoma = funcao(a, b) { a + b }\n";
        assert_eq!(parse_imports(src)[0].spec, "tela");
        assert_eq!(native_module("matriz"), Some("matriz"));
        let defs = parse_definitions(src);
        assert_eq!(defs[0].name, "soma");
        assert_eq!(defs[0].kind, DefKind::Function(vec!["a".into(), "b".into()]));
        assert_eq!(
            resolve_import("lib/ajuda", "main.lep").as_deref(),
            Some("lib/ajuda.lep")
        );
    }
}
