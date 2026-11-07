//! Indent like GTK Aula / Emacs expressa-mode, on plain text.

use crate::lang::blank_comments_and_strings;

pub const OFFSET: usize = 4;

fn strip_line_code(s: &str) -> String {
    blank_comments_and_strings(s)
}

fn leading_ws(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ev {
    Open,
    Close,
}

fn events(code: &str) -> Vec<Ev> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < code.len() {
        if code[i..].starts_with("inicio") && word_at(code, i, "inicio") {
            out.push(Ev::Open);
            i += "inicio".len();
            continue;
        }
        if code[i..].starts_with("início") && word_at(code, i, "início") {
            out.push(Ev::Open);
            i += "início".len();
            continue;
        }
        if code[i..].starts_with("fim") && word_at(code, i, "fim") {
            out.push(Ev::Close);
            i += "fim".len();
            continue;
        }
        match code[i..].chars().next() {
            Some('{') => {
                out.push(Ev::Open);
                i += 1;
            }
            Some('}') => {
                out.push(Ev::Close);
                i += 1;
            }
            Some(c) => i += c.len_utf8(),
            None => break,
        }
    }
    out
}

fn word_at(s: &str, i: usize, word: &str) -> bool {
    let before_ok = i == 0
        || !s[..i]
            .chars()
            .last()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
    let after = i + word.len();
    let after_ok = after >= s.len()
        || !s[after..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
    before_ok && after_ok
}

pub fn starts_with_closer(code: &str) -> bool {
    let t = code.trim_start();
    t.starts_with('}') || (t.starts_with("fim") && word_at(t, 0, "fim"))
}

fn opener_stack(prev_lines: &[&str]) -> Vec<usize> {
    let mut stack = Vec::new();
    for raw in prev_lines {
        let lead = leading_ws(raw);
        let code = strip_line_code(raw);
        let evs = events(&code);
        let mut opened_this_line = false;
        for ev in evs {
            match ev {
                Ev::Close => {
                    stack.pop();
                    opened_this_line = false;
                }
                Ev::Open => {
                    if !opened_this_line {
                        stack.push(lead);
                        opened_this_line = true;
                    }
                }
            }
        }
    }
    stack
}

pub fn want_indent(prev_lines: &[&str], current: &str) -> usize {
    let stack = opener_stack(prev_lines);
    let code = strip_line_code(current);
    match stack.last().copied() {
        None => 0,
        Some(base) if starts_with_closer(&code) => base,
        Some(base) => base + OFFSET,
    }
}

/// Split keeping line endings on each line except possibly the last.
pub fn linhas(texto: &str) -> Vec<String> {
    if texto.is_empty() {
        return vec![String::new()];
    }
    texto.split_inclusive('\n').map(str::to_string).collect()
}

pub fn indentar_linha(texto: &str, linha: usize) -> Option<String> {
    let mut ls = linhas(texto);
    if linha >= ls.len() {
        return None;
    }
    let prev: Vec<&str> = ls[..linha].iter().map(String::as_str).collect();
    let want = want_indent(&prev, &ls[linha]);
    let have = leading_ws(&ls[linha]);
    if have == want {
        return None;
    }
    let rest = ls[linha].chars().skip(have).collect::<String>();
    ls[linha] = format!("{}{rest}", " ".repeat(want));
    Some(ls.concat())
}

#[allow(clippy::needless_range_loop)]
pub fn recuar_bloco(texto: &str, primeira: usize, ultima: usize, delta: i32) -> String {
    let mut ls = linhas(texto);
    let fim = ultima.min(ls.len().saturating_sub(1));
    for i in primeira..=fim {
        if ls[i].trim().is_empty() {
            continue;
        }
        let have = leading_ws(&ls[i]) as i32;
        let want = (have + delta).max(0) as usize;
        let rest = ls[i].chars().skip(have as usize).collect::<String>();
        ls[i] = format!("{}{rest}", " ".repeat(want));
    }
    ls.concat()
}

#[allow(clippy::needless_range_loop)]
pub fn comentar_linhas(texto: &str, primeira: usize, ultima: usize) -> String {
    let mut ls = linhas(texto);
    let fim = ultima.min(ls.len().saturating_sub(1));
    let todas_comentadas = (primeira..=fim).all(|i| {
        let t = ls[i].trim_start();
        t.is_empty() || t.starts_with("//")
    });
    for i in primeira..=fim {
        if ls[i].trim().is_empty() {
            continue;
        }
        let lead = leading_ws(&ls[i]);
        let rest = ls[i].chars().skip(lead).collect::<String>();
        if todas_comentadas {
            let sem = rest
                .strip_prefix("// ")
                .or_else(|| rest.strip_prefix("//"))
                .unwrap_or(&rest);
            ls[i] = format!("{}{sem}", " ".repeat(lead));
        } else {
            ls[i] = format!("{}// {rest}", " ".repeat(lead));
        }
    }
    ls.concat()
}

pub fn duplicar_linha(texto: &str, linha: usize) -> String {
    let mut ls = linhas(texto);
    if linha >= ls.len() {
        return texto.to_string();
    }
    let copia = ls[linha].clone();
    let tem_nl = copia.ends_with('\n');
    let inserir = if tem_nl {
        copia.clone()
    } else {
        format!("{copia}\n")
    };
    ls.insert(linha + 1, inserir);
    if !tem_nl {
        ls[linha].push('\n');
    }
    ls.concat()
}

/// Reindent every line from the start of the file.
pub fn indentar_todo(texto: &str) -> String {
    let ls = linhas(texto);
    let mut out = Vec::with_capacity(ls.len());
    for linha in &ls {
        let prev: Vec<&str> = out.iter().map(String::as_str).collect();
        let want = want_indent(&prev, linha);
        let have = leading_ws(linha);
        let rest = linha.chars().skip(have).collect::<String>();
        out.push(format!("{}{rest}", " ".repeat(want)));
    }
    out.concat()
}

/// Block delimiters in `text` (comments and strings skipped): (opens, start, end).
fn block_tokens(text: &str) -> Vec<(bool, usize, usize)> {
    let code = blank_comments_and_strings(text);
    let mut out = Vec::new();
    let mut i = 0;
    while i < code.len() {
        let rest = &code[i..];
        let word = |w: &str| rest.starts_with(w) && word_at(&code, i, w);
        if word("inicio") || word("início") {
            let len = if rest.starts_with("inicio") {
                6
            } else {
                "início".len()
            };
            out.push((true, i, i + len));
            i += len;
        } else if word("fim") {
            out.push((false, i, i + 3));
            i += 3;
        } else if rest.starts_with('{') {
            out.push((true, i, i + 1));
            i += 1;
        } else if rest.starts_with('}') {
            out.push((false, i, i + 1));
            i += 1;
        } else {
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

/// Opener and closer byte ranges for the block at `cursor`.
pub fn par_do_bloco(text: &str, cursor: usize) -> Option<((usize, usize), (usize, usize))> {
    let tokens = block_tokens(text);
    let mut pair = vec![None; tokens.len()];
    let mut stack = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        if t.0 {
            stack.push(i);
        } else if let Some(o) = stack.pop() {
            pair[o] = Some(i);
            pair[i] = Some(o);
        }
    }
    let line_start = text[..cursor.min(text.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let line_end = text[cursor.min(text.len())..]
        .find('\n')
        .map_or(text.len(), |i| cursor + i);
    let on_line: Vec<usize> = (0..tokens.len())
        .filter(|&i| tokens[i].1 >= line_start && tokens[i].2 <= line_end && pair[i].is_some())
        .collect();
    let chosen = on_line
        .iter()
        .copied()
        .find(|&i| tokens[i].1 <= cursor && cursor <= tokens[i].2)
        .or_else(|| {
            on_line
                .first()
                .copied()
                .filter(|&i| !tokens[i].0 && text[line_start..tokens[i].1].trim().is_empty())
        })
        .or_else(|| {
            on_line
                .iter()
                .rev()
                .copied()
                .find(|&i| tokens[i].0 && tokens[pair[i].unwrap()].1 > line_end)
        })?;
    let other = pair[chosen]?;
    let (o, c) = if tokens[chosen].0 {
        (chosen, other)
    } else {
        (other, chosen)
    };
    Some(((tokens[o].1, tokens[o].2), (tokens[c].1, tokens[c].2)))
}

/// 0-based lines of the matching opener and closer, if any.
pub fn linhas_do_par(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let ((a, _), (c, _)) = par_do_bloco(text, cursor)?;
    let linha = |b: usize| text[..b.min(text.len())].bytes().filter(|&x| x == b'\n').count();
    Some((linha(a), linha(c)))
}

pub fn mover_linha(texto: &str, linha: usize, delta: i32) -> (String, usize) {
    let mut ls = linhas(texto);
    if ls.is_empty() || linha >= ls.len() {
        return (texto.to_string(), linha);
    }
    let dest = (linha as i32 + delta).clamp(0, ls.len() as i32 - 1) as usize;
    if dest == linha {
        return (texto.to_string(), linha);
    }
    let item = ls.remove(linha);
    ls.insert(dest, item);
    (ls.concat(), dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_on_same_line_as_se_is_one_level() {
        assert_eq!(want_indent(&["se a == 1 {"], "x"), 4);
        assert_eq!(want_indent(&["se a == 1 {"], "}"), 0);
        assert_eq!(want_indent(&["x = funcao(a) {"], "retorne a"), 4);
    }

    #[test]
    fn top_level_statement_does_not_indent() {
        assert_eq!(want_indent(&["escreva(1)"], "escreva(2)"), 0);
        assert_eq!(want_indent(&[], "escreva(1)"), 0);
        assert_eq!(want_indent(&["x = 1"], ""), 0);
    }

    #[test]
    fn after_closer_next_line_is_not_indented() {
        assert_eq!(want_indent(&["se a {", "    x", "}"], ""), 0);
        assert_eq!(want_indent(&["se a {", "}"], "escreva(1)"), 0);
        assert_eq!(want_indent(&["se a {", "    }"], ""), 0);
    }

    #[test]
    fn nested_uses_opener_line_indent() {
        assert_eq!(want_indent(&["se a {", "    se b {"], "x"), 8);
        assert_eq!(want_indent(&["se a {", "    se b {", "        x"], "}"), 4);
        assert_eq!(
            want_indent(&["    se a == 1 {"], "x"),
            8,
            "opener already indented by 4"
        );
    }

    #[test]
    fn par_inicio_fim_ignora_string() {
        let src = "se a {\n    x // {\n    s = \"}\"\n}\npara i de 1 ate 2\ninicio\n    y\nfim\n";
        let at = |needle: &str, nth: usize| src.match_indices(needle).nth(nth).unwrap().0;
        let brace_open = at("{", 0);
        let brace_close = at("}\n", 0);
        assert_eq!(
            par_do_bloco(src, 0),
            Some(((brace_open, brace_open + 1), (brace_close, brace_close + 1)))
        );
        assert_eq!(par_do_bloco(src, at("x //", 0)), None);
        let inicio = at("inicio", 0);
        let fim = at("fim", 0);
        assert_eq!(
            par_do_bloco(src, inicio + 2),
            Some(((inicio, inicio + 6), (fim, fim + 3)))
        );
        let poema = "escreva(\"início: …\")\ninicio\n    x\nfim\n";
        let ini = poema.find("\ninicio").unwrap() + 1;
        assert_eq!(
            par_do_bloco(poema, ini + 1).map(|((a, b), _)| &poema[a..b]),
            Some("inicio")
        );
    }

    #[test]
    fn keywords_in_strings_and_comments_do_not_indent() {
        assert_eq!(
            want_indent(
                &[r#"escreva("início: " + verso[1..5])"#],
                r#"escreva("hashtag: " + junte(palavras, "0"))"#
            ),
            0
        );
        assert_eq!(want_indent(&[r#"s = "{""#], "x"), 0);
        assert_eq!(want_indent(&["x // {"], "y"), 0);
        assert_eq!(want_indent(&["se a {", r#"    s = "fim""#], "x"), 4);
    }

    #[test]
    fn comentar_e_descomentar() {
        let t = "x = 1\ny = 2\n";
        let c = comentar_linhas(t, 0, 1);
        assert_eq!(c, "// x = 1\n// y = 2\n");
        assert_eq!(comentar_linhas(&c, 0, 1), t);
    }

    #[test]
    fn recuo_do_arquivo_inteiro() {
        let t = "se a {\nx\n}\nescreva(1)\n";
        assert_eq!(indentar_todo(t), "se a {\n    x\n}\nescreva(1)\n");
    }
}
