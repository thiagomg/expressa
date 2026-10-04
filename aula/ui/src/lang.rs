//! Text analysis for editor help: imports, definitions, what to complete at
//! the cursor, the call around it. Works on plain text (often incomplete
//! while typing), so it does not use the parser. Offsets are byte offsets.
//!
//! Native functions come from the interpreter (`NATIVAS`, `BUILTIN_DOCS`),
//! so the help always matches what `expressa` accepts.

use std::sync::OnceLock;

use expressa::runtime::{NATIVAS, lookup_builtin_doc};

/// Reserved words (`src/lexer/tokens.rs`, checked by a test).
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

pub struct Native {
    pub name: &'static str,
    pub aliases: Vec<&'static str>,
    /// `None` = always in scope; else needs `importe "module"`.
    pub module: Option<&'static str>,
    pub signatures: Vec<String>,
    pub summary: &'static str,
    pub example: &'static str,
    /// A value (`argumentos`), not a function.
    pub is_value: bool,
}

pub fn natives() -> &'static [Native] {
    static ALL: OnceLock<Vec<Native>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut out: Vec<Native> = NATIVAS
            .iter()
            .map(|n| {
                let doc = lookup_builtin_doc(n.names[0]);
                Native {
                    name: n.names[0],
                    aliases: n.names[1..].to_vec(),
                    module: n.module,
                    signatures: doc
                        .map(|d| {
                            d.sig
                                .split("  ou  ")
                                .map(|s| s.trim().to_string())
                                .collect()
                        })
                        .unwrap_or_else(|| vec![format!("{}(...)", n.names[0])]),
                    summary: doc.map(|d| d.summary).unwrap_or(""),
                    example: doc.map(|d| d.example).unwrap_or(""),
                    is_value: false,
                }
            })
            .collect();
        // Not a function: the interpreter defines it in every program.
        out.push(Native {
            name: "argumentos",
            aliases: Vec::new(),
            module: None,
            signatures: Vec::new(),
            summary: "Lista dos valores após o .lep na linha de comando (argumentos[1], …).",
            example: "escreva(argumentos[1])",
            is_value: true,
        });
        out
    })
}

pub fn native(name: &str) -> Option<&'static Native> {
    natives()
        .iter()
        .find(|n| n.name == name || n.aliases.contains(&name))
}

pub fn modules() -> Vec<&'static str> {
    let mut m: Vec<&str> = NATIVAS.iter().filter_map(|n| n.module).collect();
    m.sort_unstable();
    m.dedup();
    m
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

/// Comments and string contents replaced by spaces of the same byte length
/// (quotes kept, line breaks kept), so offsets still match `text`.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Code,
    Text,
    LineComment,
    BlockComment,
}

/// Lexical state at the end of `text` (the cursor).
pub fn state_at_end(text: &str) -> State {
    // Byte scan: every delimiter is ASCII and UTF-8 continuation bytes never
    // are, so this cannot split a character.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub alias: Option<String>,
    pub spec: String,
    /// 0-based line.
    pub line: usize,
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
            continue; // importe_x
        }
        let after = after.trim_start();
        if !after.starts_with('"') {
            continue;
        }
        // The blanked copy has spaces inside quotes: read the original.
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

/// Native module for `spec`, or `None` when it names a `.lep` file. Same
/// rule as the interpreter: no `/`, `./` or `.lep` means native first.
pub fn native_module(spec: &str) -> Option<&'static str> {
    if spec.contains('/') || spec.ends_with(".lep") {
        return None;
    }
    modules().into_iter().find(|m| *m == spec)
}

/// Project-relative path of a file `importe` from the file `from` (also
/// project-relative). `None` if it leaves the project.
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
    parts.pop(); // file name of `from`
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
    /// 0-based line.
    pub line: usize,
    /// Defined at column 0 (visible to files that import this one).
    pub top_level: bool,
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
/// The first definition of a name wins.
pub fn parse_definitions(text: &str) -> Vec<Def> {
    let code = blank_comments_and_strings(text);
    let mut out: Vec<Def> = Vec::new();
    let mut add = |def: Def| {
        if !KEYWORDS.contains(&def.name.as_str()) && !out.iter().any(|d| d.name == def.name) {
            out.push(def);
        }
    };
    for (line_no, line) in code.split('\n').enumerate() {
        let indent = line.len() - line.trim_start().len();
        let body = line.trim_start();
        // name = …  (but not ==)
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
            });
        }
        // para x de / para x em
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
                });
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completion {
    /// In a comment or string: nothing.
    Nothing,
    /// Inside `importe "…`.
    Import {
        prefix: String,
    },
    /// After `alias::`.
    Module {
        alias: String,
        prefix: String,
    },
    /// After `x.` (UFCS call).
    Method {
        prefix: String,
    },
    Name {
        prefix: String,
    },
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
    // Typing a number (`3`, `1_000`): nothing to complete.
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
    // `pessoa:nome`, `:verde`: a key, not a name.
    if head.ends_with(':') {
        return Completion::Nothing;
    }
    if let Some(h) = head.strip_suffix('.') {
        let receiver_ok = h
            .chars()
            .last()
            .is_some_and(|c| is_ident_char(c) || matches!(c, ')' | ']' | '"'));
        // `3.` is a number being typed.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub name: String,
    pub alias: Option<String>,
    /// `x.f(…)`: `x` is the first argument.
    pub method: bool,
    /// Index of the argument being typed (commas so far).
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
            b'\n' if i > 0 && b[i - 1] == b'\n' => return None, // blank line
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

/// Line where a new `importe` goes: after the last one, else after `#!`.
pub fn import_insert_line(text: &str) -> usize {
    match parse_imports(text).last() {
        Some(i) => i.line + 1,
        None => usize::from(text.starts_with("#!")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub alias: Option<String>,
    pub name: String,
    /// Byte range of `name` in the line.
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

/// Expressa program whose stdout is the general catalog (natives, imported
/// native modules, and top-level `funcao` from `text`).
pub fn catalogo_fonte(text: &str) -> String {
    let mut src = String::new();
    for imp in parse_imports(text) {
        if native_module(&imp.spec).is_none() {
            continue;
        }
        match &imp.alias {
            Some(a) => src.push_str(&format!("{a} = importe \"{}\"\n", imp.spec)),
            None => src.push_str(&format!("importe \"{}\"\n", imp.spec)),
        }
    }
    for def in parse_definitions(text) {
        if !def.top_level {
            continue;
        }
        let DefKind::Function(params) = &def.kind else {
            continue;
        };
        if native(&def.name).is_some() {
            continue;
        }
        src.push_str(&format!(
            "{} = funcao({}) {{ 0 }}\n",
            def.name,
            params.join(", ")
        ));
    }
    src.push_str(
        r#"
para f em catalogo() {
    sig = f:nome
    se f:tipo != "modulo" e f:args != "" {
        sig = sig + "(" + f:args + ")"
    }
    extra = ""
    se f:modulo != "" e f:tipo != "modulo" {
        extra = "  " + f:modulo
    }
    escreva(sig + "    " + f:tipo + extra)
}
"#,
    );
    src
}

/// Run [`catalogo_fonte`] in the interpreter; the editor shows this text.
pub fn catalogo_texto(text: &str) -> String {
    expressa::runtime::run_to_string(&catalogo_fonte(text), "<catalogo.lep>")
        .unwrap_or_else(|e| format!("ajuda: {}", e.message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_match_the_lexer() {
        for k in KEYWORDS {
            assert!(
                expressa::lexer::tokens::keyword(k).is_some(),
                "{k} is not a keyword"
            );
        }
        // And nothing common is missing.
        for k in ["se", "senão", "até", "função", "se_falhar", "contém"] {
            assert!(KEYWORDS.contains(&k));
        }
    }

    #[test]
    fn natives_from_the_interpreter() {
        let pinte = native("pinte").unwrap();
        assert_eq!(pinte.module, Some("tela"));
        assert_eq!(pinte.signatures.len(), 2);
        assert_eq!(native("é_terminal").unwrap().name, "eh_terminal");
        assert!(native("argumentos").unwrap().is_value);
        assert!(native("escreva").unwrap().module.is_none());
        assert_eq!(modules(), ["arquivo", "mat", "matriz", "tela"]);
        for n in natives() {
            assert!(!n.summary.is_empty(), "{} has no summary", n.name);
        }
    }

    #[test]
    fn catalogo_texto_comes_from_expressa() {
        let geral = catalogo_texto("");
        assert!(geral.contains("escreva"), "{geral}");
        assert!(geral.contains("nativa"), "{geral}");
        assert!(geral.contains("matriz"), "{geral}");
        let com_fn = catalogo_texto("soma = funcao(a, b) { a + b }\n");
        assert!(com_fn.contains("soma(a, b)"), "{com_fn}");
        assert!(com_fn.contains("funcao"), "{com_fn}");
        let com_imp = catalogo_texto("importe \"tela\"\n");
        assert!(com_imp.contains("pinte"), "{com_imp}");
    }

    #[test]
    fn blanking_keeps_offsets() {
        let src = "x = \"olá // não\" // comentário ç\ny /* a\nb */ z";
        let b = blank_comments_and_strings(src);
        assert_eq!(b.len(), src.len());
        assert_eq!(b.matches('\n').count(), 2);
        assert!(b.contains("x = \""));
        assert!(!b.contains("olá"));
        assert!(b.ends_with(" z"));
    }

    #[test]
    fn state() {
        assert_eq!(state_at_end("x = \"ab"), State::Text);
        assert_eq!(state_at_end("x = \"a\\\"b"), State::Text);
        assert_eq!(state_at_end("x = \"a\" + y"), State::Code);
        assert_eq!(state_at_end("x // nota"), State::LineComment);
        assert_eq!(state_at_end("/* a\n b"), State::BlockComment);
        assert_eq!(state_at_end("a /* b */ c"), State::Code);
        assert_eq!(state_at_end("#!/usr/bin/env expressa\nx"), State::Code);
    }

    #[test]
    fn imports() {
        let src = "// importe \"nao\"\nimporte \"tela\"\nm = importe \"lib/ajuda\"\nx = \"importe \\\"y\\\"\"\n";
        assert_eq!(
            parse_imports(src),
            vec![
                Import {
                    alias: None,
                    spec: "tela".into(),
                    line: 1
                },
                Import {
                    alias: Some("m".into()),
                    spec: "lib/ajuda".into(),
                    line: 2
                },
            ]
        );
        assert_eq!(import_insert_line(src), 3);
        assert_eq!(import_insert_line("#!/usr/bin/env expressa\nescreva(1)"), 1);
        assert_eq!(import_insert_line("escreva(1)"), 0);
        assert_eq!(native_module("matriz"), Some("matriz"));
        assert_eq!(native_module("./matriz"), None);
        assert_eq!(native_module("matematica"), None);
        assert_eq!(
            resolve_import("lib/ajuda", "main.lep").as_deref(),
            Some("lib/ajuda.lep")
        );
        assert_eq!(
            resolve_import("./b", "lib/a.lep").as_deref(),
            Some("lib/b.lep")
        );
        assert_eq!(
            resolve_import("../c.lep", "lib/a.lep").as_deref(),
            Some("c.lep")
        );
        assert_eq!(resolve_import("../../x", "a.lep"), None);
    }

    #[test]
    fn definitions() {
        let src = "soma = funcao(a, b) {\n    total = a + b\n}\nx = 1\nx == 2\npara i de 1 ate 3 {}\nm = importe \"mat\"\n// comentado = 2\nvelocidade_média = função() { 3 }\ns = \"y = 1\"\npara item em lista {}\n";
        let defs = parse_definitions(src);
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "soma",
                "total",
                "x",
                "i",
                "m",
                "velocidade_média",
                "s",
                "item"
            ]
        );
        assert_eq!(
            defs[0].kind,
            DefKind::Function(vec!["a".into(), "b".into()])
        );
        assert!(defs[0].top_level && !defs[1].top_level);
        assert_eq!(defs[4].kind, DefKind::Module);
        assert_eq!(defs[5].kind, DefKind::Function(vec![]));
        assert_eq!(defs[0].signature(), "soma(a, b)");
    }

    #[test]
    fn completion() {
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
        assert_eq!(c("\"oi\"."), Completion::Method { prefix: "".into() });
        assert_eq!(c("f(x)."), Completion::Method { prefix: "".into() });
        assert_eq!(c("y = 3."), name(""));
        assert_eq!(c("y = 3"), Completion::Nothing);
        assert_eq!(c("p:"), Completion::Nothing);
        assert_eq!(c("pinte(:ve"), Completion::Nothing);
        assert_eq!(c("x // esc"), Completion::Nothing);
        assert_eq!(c("/* esc"), Completion::Nothing);
        assert_eq!(c("esc"), name("esc"));
        assert_eq!(c("a = espa"), name("espa"));
        assert_eq!(c("linha 1\n  ação"), name("ação"));
    }

    #[test]
    fn calls() {
        let call = |name: &str, alias: Option<&str>, method: bool, active: usize| Call {
            name: name.into(),
            alias: alias.map(Into::into),
            method,
            active,
        };
        assert_eq!(
            call_context("escreva(1, pinte(\"a\", "),
            Some(call("pinte", None, false, 1))
        );
        assert_eq!(
            call_context("x.pinte(:verde, "),
            Some(call("pinte", None, true, 1))
        );
        assert_eq!(
            call_context("m::raiz("),
            Some(call("raiz", Some("m"), false, 0))
        );
        assert_eq!(call_context("f([1, 2], "), Some(call("f", None, false, 1)));
        assert_eq!(
            call_context("escreva(\"a,b\", "),
            Some(call("escreva", None, false, 1))
        );
        assert_eq!(call_context("escreva(1)\n"), None);
        assert_eq!(
            call_context("soma(1,\n    2"),
            Some(call("soma", None, false, 1))
        );
        assert_eq!(call_context("se (a"), None);
    }

    #[test]
    fn words_and_signatures() {
        assert_eq!(
            signature_params("pinte(texto, frente, fundo) -> texto"),
            ["texto", "frente", "fundo"]
        );
        assert!(signature_params("cls()").is_empty());
        assert_eq!(
            word_at("x = m::raiz(4)", 9),
            Some(Word {
                alias: Some("m".into()),
                name: "raiz".into(),
                start: 7,
                end: 11
            })
        );
        let w = word_at("espaço = 1", 3).unwrap();
        assert_eq!((w.name.as_str(), w.start, w.end), ("espaço", 0, 7));
        assert_eq!(word_at("a + b", 2), None);
        assert_eq!(word_at("abc", 3).unwrap().name, "abc");
    }
}
