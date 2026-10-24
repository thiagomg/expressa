//! Markdown enough for `tutorial/basico.md` and `tutorial/avancado.md`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Bold(String),
    Code(String),
    Link { text: String, href: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading { level: u32, text: String },
    Paragraph(Vec<Inline>),
    Code { lang: String, text: String },
    Quote(Vec<Inline>),
    List(Vec<Vec<Inline>>),
    Table { headers: Vec<String>, rows: Vec<Vec<String>> },
    Hr,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: String,
    pub index: usize,
}

pub fn chapters(blocks: &[Block]) -> Vec<Chapter> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(index, b)| match b {
            Block::Heading { level, text } if *level <= 2 => Some(Chapter {
                title: text.clone(),
                index,
            }),
            _ => None,
        })
        .collect()
}

pub fn parse(src: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        if trimmed == "---" || trimmed == "***" {
            out.push(Block::Hr);
            i += 1;
            continue;
        }
        if trimmed.starts_with("```") {
            let lang = trimmed[3..].trim().to_string();
            i += 1;
            let mut body = String::new();
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            out.push(Block::Code { lang, text: body });
            continue;
        }
        if let Some(rest) = heading(trimmed) {
            out.push(Block::Heading {
                level: rest.0,
                text: rest.1,
            });
            i += 1;
            continue;
        }
        if trimmed.starts_with("> ") || trimmed == ">" {
            let mut text = String::new();
            while i < lines.len() {
                let t = lines[i].trim();
                if let Some(r) = t.strip_prefix("> ") {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(r);
                    i += 1;
                } else if t == ">" {
                    i += 1;
                    break;
                } else {
                    break;
                }
            }
            out.push(Block::Quote(inlines(&text)));
            continue;
        }
        if is_table_row(trimmed) && i + 1 < lines.len() && is_table_sep(lines[i + 1].trim()) {
            let headers = split_row(trimmed);
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && is_table_row(lines[i].trim()) {
                rows.push(split_row(lines[i].trim()));
                i += 1;
            }
            out.push(Block::Table { headers, rows });
            continue;
        }
        if is_list_item(trimmed) {
            let mut items = Vec::new();
            while i < lines.len() && is_list_item(lines[i].trim()) {
                items.push(inlines(list_text(lines[i].trim())));
                i += 1;
            }
            out.push(Block::List(items));
            continue;
        }
        let mut para = String::new();
        while i < lines.len() {
            let t = lines[i].trim();
            if t.is_empty()
                || t.starts_with('#')
                || t.starts_with("```")
                || t.starts_with("> ")
                || t == "---"
                || is_list_item(t)
                || is_table_row(t)
            {
                break;
            }
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t);
            i += 1;
        }
        if !para.is_empty() {
            out.push(Block::Paragraph(inlines(&para)));
        }
    }
    out
}

fn heading(line: &str) -> Option<(u32, String)> {
    if !line.starts_with('#') {
        return None;
    }
    let mut n = 0u32;
    let bytes = line.as_bytes();
    while n < 6 && (n as usize) < bytes.len() && bytes[n as usize] == b'#' {
        n += 1;
    }
    let rest = line.get(n as usize..)?.trim();
    if rest.is_empty() || n == 0 {
        return None;
    }
    Some((n, rest.to_string()))
}

fn is_list_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || numbered_list(line).is_some()
}

fn numbered_list(line: &str) -> Option<&str> {
    let (num, rest) = line.split_once('.')?;
    if num.chars().all(|c| c.is_ascii_digit()) && rest.starts_with(' ') {
        Some(rest.trim_start())
    } else {
        None
    }
}

fn list_text(line: &str) -> &str {
    if let Some(r) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        r
    } else {
        numbered_list(line).unwrap_or(line)
    }
}

fn is_table_row(line: &str) -> bool {
    line.starts_with('|') && line.ends_with('|') && line.len() > 1
}

fn is_table_sep(line: &str) -> bool {
    is_table_row(line)
        && line
            .chars()
            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
}

fn split_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|c| strip_md_span(c.trim()).to_string())
        .collect()
}

fn strip_md_span(s: &str) -> String {
    s.replace("**", "").replace('`', "")
}

pub fn inlines(s: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut buf = String::new();
    let push_buf = |buf: &mut String, out: &mut Vec<Inline>| {
        if !buf.is_empty() {
            out.push(Inline::Text(std::mem::take(buf)));
        }
    };
    while i < chars.len() {
        if chars[i] == '`' {
            push_buf(&mut buf, &mut out);
            i += 1;
            let mut code = String::new();
            while i < chars.len() && chars[i] != '`' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            out.push(Inline::Code(code));
        } else if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            push_buf(&mut buf, &mut out);
            i += 2;
            let mut bold = String::new();
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            out.push(Inline::Bold(bold));
        } else if chars[i] == '[' {
            if let Some((text, href, next)) = parse_link(&chars, i) {
                push_buf(&mut buf, &mut out);
                out.push(Inline::Link { text, href });
                i = next;
            } else {
                buf.push(chars[i]);
                i += 1;
            }
        } else {
            buf.push(chars[i]);
            i += 1;
        }
    }
    push_buf(&mut buf, &mut out);
    out
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let mut i = start + 1;
    let mut text = String::new();
    while i < chars.len() && chars[i] != ']' {
        text.push(chars[i]);
        i += 1;
    }
    if i + 1 >= chars.len() || chars[i] != ']' || chars[i + 1] != '(' {
        return None;
    }
    i += 2;
    let mut href = String::new();
    while i < chars.len() && chars[i] != ')' {
        href.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() || chars[i] != ')' {
        return None;
    }
    Some((text, href, i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_and_code() {
        let b = parse("# Olá\n\n```text\nescreva(1)\n```\n");
        assert!(matches!(&b[0], Block::Heading { level: 1, text } if text == "Olá"));
        assert!(matches!(&b[1], Block::Code { text, .. } if text.contains("escreva")));
    }

    #[test]
    fn inlines_bold_and_code() {
        let v = inlines("use **F5** and `escreva`");
        assert_eq!(v[1], Inline::Bold("F5".into()));
        assert_eq!(v[3], Inline::Code("escreva".into()));
    }

    #[test]
    fn parses_shipped_tutorials() {
        let basico = include_str!("../../../tutorial/basico.md");
        let avancado = include_str!("../../../tutorial/avancado.md");
        let a = parse(basico);
        let b = parse(avancado);
        assert!(chapters(&a).len() >= 8);
        assert!(chapters(&b).len() >= 8);
        assert!(a.iter().any(|x| matches!(x, Block::Code { .. })));
    }
}
