//! Program output: ANSI colors, cls/quadro, clickable `arquivo:linha`.

pub const MAX_LINHAS: usize = 5000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trecho {
    pub texto: String,
    pub stderr: bool,
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    pub negrito: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Console {
    pub linhas: Vec<Vec<Trecho>>,
    pub leia_prompt: Option<String>,
    pub entrada: String,
    tela: Tela,
    estilo: AnsiStyle,
}

/// Cursor after `cls()` / `casa()` / `quadro` / `escreva_em`.
/// `row: None` means plain appending (no screen codes in this run).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Tela {
    row: Option<usize>,
    col: usize,
    start_col: usize,
    partial: String,
    held: String,
}

impl Console {
    pub fn limpar(&mut self) {
        self.linhas.clear();
        self.leia_prompt = None;
        self.entrada.clear();
        self.tela = Tela::default();
        self.estilo = AnsiStyle::default();
    }

    pub fn escrever(&mut self, texto: &str, stderr: bool) {
        if !stderr
            && (self.tela.row.is_some()
                || !self.tela.held.is_empty()
                || has_screen_code(texto)
                || csi_incomplete(texto))
        {
            write_screen(self, texto);
            return;
        }
        self.acrescentar_ansi(texto, stderr);
        self.aparar();
    }

    fn acrescentar_ansi(&mut self, texto: &str, stderr: bool) {
        let spans = ansi_spans(texto, &mut self.estilo);
        for (vis, estilo) in spans {
            self.acrescentar(&vis, stderr, estilo);
        }
    }

    fn acrescentar(&mut self, texto: &str, stderr: bool, estilo: AnsiStyle) {
        if texto.is_empty() {
            return;
        }
        let mut rest = texto;
        while let Some(i) = rest.find('\n') {
            let (head, tail) = rest.split_at(i);
            if !head.is_empty() {
                self.push_trecho(head, stderr, estilo);
            }
            self.linhas.push(Vec::new());
            rest = &tail[1..];
        }
        if !rest.is_empty() {
            self.push_trecho(rest, stderr, estilo);
        }
    }

    fn push_trecho(&mut self, texto: &str, stderr: bool, estilo: AnsiStyle) {
        if self.linhas.is_empty() {
            self.linhas.push(Vec::new());
        }
        let linha = self.linhas.last_mut().unwrap();
        linha.push(Trecho {
            texto: texto.to_string(),
            stderr,
            fg: estilo.fg,
            bg: estilo.bg,
            negrito: estilo.negrito,
        });
    }

    fn aparar(&mut self) {
        if self.linhas.len() > MAX_LINHAS {
            let extra = self.linhas.len() - MAX_LINHAS;
            self.linhas.drain(..extra);
        }
    }

    pub fn texto_linha(&self, i: usize) -> String {
        self.linhas
            .get(i)
            .map(|ts| ts.iter().map(|t| t.texto.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn texto_completo(&self) -> String {
        if self.linhas.is_empty() {
            return String::new();
        }
        let mut s = (0..self.linhas.len())
            .map(|i| self.texto_linha(i))
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(row) = self.tela.row
            && self.tela.col == 0
            && row >= self.linhas.len()
            && !s.ends_with('\n')
        {
            s.push('\n');
        }
        s
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AnsiStyle {
    fg: Option<(u8, u8, u8)>,
    bg: Option<(u8, u8, u8)>,
    negrito: bool,
}

const CORES: [(u8, u8, u8); 8] = [
    (33, 33, 33),
    (198, 40, 40),
    (46, 125, 50),
    (249, 168, 37),
    (21, 101, 192),
    (106, 27, 154),
    (0, 131, 143),
    (66, 66, 66),
];

fn ansi_spans(text: &str, estilo: &mut AnsiStyle) -> Vec<(String, AnsiStyle)> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if let Some((n, final_byte, params)) = parse_csi(rest) {
            if !buf.is_empty() {
                out.push((std::mem::take(&mut buf), *estilo));
            }
            if final_byte == b'm' {
                apply_sgr(estilo, params);
            }
            rest = &rest[n..];
            continue;
        }
        let c = rest.chars().next().unwrap();
        if c == '\x1b' {
            if csi_incomplete(rest) {
                break;
            }
            rest = &rest[c.len_utf8()..];
            continue;
        }
        buf.push(c);
        rest = &rest[c.len_utf8()..];
    }
    if !buf.is_empty() {
        out.push((buf, *estilo));
    }
    out
}

fn apply_sgr(estilo: &mut AnsiStyle, params: &str) {
    if params.is_empty() {
        *estilo = AnsiStyle::default();
        return;
    }
    for p in params.split(';') {
        match p.parse::<u8>().unwrap_or(0) {
            0 => *estilo = AnsiStyle::default(),
            1 => estilo.negrito = true,
            22 => estilo.negrito = false,
            n @ 30..=37 => estilo.fg = Some(CORES[(n - 30) as usize]),
            n @ 40..=47 => estilo.bg = Some(CORES[(n - 40) as usize]),
            39 => estilo.fg = None,
            49 => estilo.bg = None,
            _ => {}
        }
    }
}

fn has_screen_code(text: &str) -> bool {
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        if let Some((len, final_byte, _)) = parse_csi(rest) {
            if matches!(final_byte, b'H' | b'f' | b'J' | b'K') {
                return true;
            }
            pos += len;
            continue;
        }
        pos += rest.chars().next().unwrap().len_utf8();
    }
    false
}

fn csi_incomplete(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 || bytes[0] != 0x1b {
        return false;
    }
    if bytes.len() == 1 {
        return true;
    }
    if bytes[1] != b'[' {
        return false;
    }
    let mut i = 2;
    while i < bytes.len() && (0x30..=0x3f).contains(&bytes[i]) {
        i += 1;
    }
    while i < bytes.len() && (0x20..=0x2f).contains(&bytes[i]) {
        i += 1;
    }
    i == bytes.len()
}

fn parse_csi(text: &str) -> Option<(usize, u8, &str)> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || bytes[0] != 0x1b || bytes[1] != b'[' {
        return None;
    }
    let mut i = 2;
    while i < bytes.len() && (0x30..=0x3f).contains(&bytes[i]) {
        i += 1;
    }
    while i < bytes.len() && (0x20..=0x2f).contains(&bytes[i]) {
        i += 1;
    }
    if i >= bytes.len() || !(0x40..=0x7e).contains(&bytes[i]) {
        return None;
    }
    let params = std::str::from_utf8(&bytes[2..i]).ok()?;
    Some((i + 1, bytes[i], params))
}

fn parse_cup(params: &str) -> (usize, usize) {
    let mut row = 1usize;
    let mut col = 1usize;
    let mut parts = params.split(';');
    if let Some(p) = parts.next()
        && !p.is_empty()
    {
        row = p.parse().unwrap_or(1);
    }
    if let Some(p) = parts.next()
        && !p.is_empty()
    {
        col = p.parse().unwrap_or(1);
    }
    (row.max(1), col.max(1))
}

fn visible_len(text: &str) -> usize {
    let mut estilo = AnsiStyle::default();
    ansi_spans(text, &mut estilo)
        .iter()
        .map(|(s, _)| s.chars().count())
        .sum()
}

fn write_screen(c: &mut Console, text: &str) {
    if c.tela.held.is_empty() {
        write_screen_now(c, text);
    } else {
        let mut all = std::mem::take(&mut c.tela.held);
        all.push_str(text);
        write_screen_now(c, &all);
    }
}

fn write_screen_now(c: &mut Console, text: &str) {
    let mut row = c.tela.row.unwrap_or_else(|| c.linhas.len().saturating_sub(1));
    let mut col = c.tela.col;
    let mut start_col = c.tela.start_col;
    let mut rest = text;
    while let Some(ch) = rest.chars().next() {
        if let Some((len, final_byte, params)) = parse_csi(rest) {
            let csi = &rest[..len];
            rest = &rest[len..];
            match final_byte {
                b'J' => {
                    flush_partial(c, &mut row, &mut col, &mut start_col);
                    if params == "2" {
                        c.linhas.clear();
                        row = 0;
                        col = 0;
                        start_col = 0;
                    } else {
                        erase_down(c, row, col);
                    }
                }
                b'K' => {
                    flush_partial(c, &mut row, &mut col, &mut start_col);
                    if params == "2" {
                        erase_line_from(c, row, 0);
                    } else {
                        erase_line_from(c, row, col);
                    }
                }
                b'H' | b'f' => {
                    flush_partial(c, &mut row, &mut col, &mut start_col);
                    let (r, col1) = parse_cup(params);
                    row = r - 1;
                    col = col1 - 1;
                    start_col = col;
                    ensure_line(c, row);
                }
                b'm' => {
                    c.tela.partial.push_str(csi);
                }
                _ => {}
            }
            continue;
        }
        if rest.as_bytes().first() == Some(&0x1b) && csi_incomplete(rest) {
            c.tela.held = rest.to_string();
            break;
        }
        if ch == '\r' {
            flush_partial(c, &mut row, &mut col, &mut start_col);
            col = 0;
            start_col = 0;
            rest = &rest[1..];
        } else if ch == '\n' {
            flush_partial(c, &mut row, &mut col, &mut start_col);
            row += 1;
            col = 0;
            start_col = 0;
            ensure_line(c, row);
            aparar_tela(c, &mut row);
            rest = &rest[1..];
        } else {
            c.tela.partial.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }
    flush_partial(c, &mut row, &mut col, &mut start_col);
    c.tela.row = Some(row);
    c.tela.col = col;
    c.tela.start_col = start_col;
}

fn aparar_tela(c: &mut Console, row: &mut usize) {
    if c.linhas.len() > MAX_LINHAS {
        let extra = c.linhas.len() - MAX_LINHAS;
        c.linhas.drain(..extra);
        *row = row.saturating_sub(extra);
    }
}

fn flush_partial(c: &mut Console, row: &mut usize, col: &mut usize, start_col: &mut usize) {
    if c.tela.partial.is_empty() {
        return;
    }
    let content = std::mem::take(&mut c.tela.partial);
    let vis = visible_len(&content);
    if *start_col == 0 {
        replace_line(c, *row, &content);
    } else {
        overlay_line(c, *row, *start_col, &content);
    }
    *col = *start_col + vis;
    *start_col = *col;
}

fn ensure_line(c: &mut Console, row: usize) {
    while row >= c.linhas.len() {
        c.linhas.push(Vec::new());
    }
}

fn flatten(linha: &[Trecho]) -> Vec<(char, AnsiStyle, bool)> {
    let mut out = Vec::new();
    for t in linha {
        let estilo = AnsiStyle {
            fg: t.fg,
            bg: t.bg,
            negrito: t.negrito,
        };
        for ch in t.texto.chars() {
            out.push((ch, estilo, t.stderr));
        }
    }
    out
}

fn unflatten(chars: Vec<(char, AnsiStyle, bool)>) -> Vec<Trecho> {
    let mut out: Vec<Trecho> = Vec::new();
    for (ch, estilo, stderr) in chars {
        if let Some(last) = out.last_mut()
            && last.stderr == stderr
            && last.fg == estilo.fg
            && last.bg == estilo.bg
            && last.negrito == estilo.negrito
        {
            last.texto.push(ch);
            continue;
        }
        out.push(Trecho {
            texto: ch.to_string(),
            stderr,
            fg: estilo.fg,
            bg: estilo.bg,
            negrito: estilo.negrito,
        });
    }
    out
}

fn chars_from_ansi(content: &str, estilo: &mut AnsiStyle) -> Vec<(char, AnsiStyle, bool)> {
    let mut out = Vec::new();
    for (vis, e) in ansi_spans(content, estilo) {
        for ch in vis.chars() {
            out.push((ch, e, false));
        }
    }
    out
}

fn replace_line(c: &mut Console, row: usize, content: &str) {
    ensure_line(c, row);
    c.linhas[row] = unflatten(chars_from_ansi(content, &mut c.estilo));
}

fn overlay_line(c: &mut Console, row: usize, col: usize, content: &str) {
    ensure_line(c, row);
    let mut cells = flatten(&c.linhas[row]);
    if cells.len() < col {
        cells.resize(col, (' ', AnsiStyle::default(), false));
    }
    let novo = chars_from_ansi(content, &mut c.estilo);
    let vis = novo.len();
    let end = (col + vis).min(cells.len());
    if col < cells.len() {
        cells.drain(col..end);
    }
    for (i, cell) in novo.into_iter().enumerate() {
        cells.insert(col + i, cell);
    }
    c.linhas[row] = unflatten(cells);
}

fn erase_line_from(c: &mut Console, row: usize, col: usize) {
    if row >= c.linhas.len() {
        return;
    }
    let mut cells = flatten(&c.linhas[row]);
    if col < cells.len() {
        cells.truncate(col);
        c.linhas[row] = unflatten(cells);
    }
}

fn erase_down(c: &mut Console, row: usize, col: usize) {
    let from = if col == 0 { row } else { row + 1 };
    if col > 0 {
        erase_line_from(c, row, col);
    }
    if from < c.linhas.len() {
        c.linhas.truncate(from);
    }
}

fn parse_file_line(loc: &str) -> Option<(String, u32)> {
    let loc = loc.trim();
    let mut parts = loc.rsplitn(3, ':');
    let last = parts.next()?;
    let last: u32 = last.parse().ok()?;
    let mid = parts.next()?;
    match (mid.parse::<u32>(), parts.next()) {
        (Ok(line), Some(file)) if !file.is_empty() => Some((file.to_string(), line)),
        _ => Some((mid.to_string(), last)),
    }
}

/// `"erro: … em lib/a.lep:12"` or `"  0: pega (app.lep:4:1)"`.
pub fn parse_error_location(line: &str) -> Option<(String, u32)> {
    let line = line.trim();
    if let Some(em) = line.rfind(" em ")
        && let Some(parsed) = parse_file_line(&line[em + 4..])
    {
        return Some(parsed);
    }
    let open = line.rfind('(')?;
    let inner = line.get(open + 1..)?.strip_suffix(')')?;
    parse_file_line(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tela(chunks: &[&str]) -> String {
        let mut c = Console::default();
        for chunk in chunks {
            c.escrever(chunk, false);
        }
        c.texto_completo()
    }

    #[test]
    fn escreva_vira_linha() {
        let mut c = Console::default();
        c.escrever("olá\n", false);
        assert_eq!(c.texto_linha(0), "olá");
    }

    #[test]
    fn cls_limpa() {
        let mut c = Console::default();
        c.escrever("a\n", false);
        c.escrever("\x1b[2J\x1b[H", false);
        assert!(c.linhas.iter().all(|l| l.is_empty()));
    }

    #[test]
    fn pinte_nao_aparece_como_lixo() {
        let mut c = Console::default();
        c.escrever("\x1b[32mverde\x1b[0m\n", false);
        assert_eq!(c.texto_linha(0), "verde");
        assert_eq!(c.linhas[0][0].fg, Some((46, 125, 50)));
    }

    #[test]
    fn pinte_depois_negrito_acumula() {
        let mut c = Console::default();
        c.escrever("\x1b[32m", false);
        c.escrever("\x1b[1mverde\n", false);
        assert_eq!(c.texto_linha(0), "verde");
        assert_eq!(c.linhas[0][0].fg, Some((46, 125, 50)));
        assert!(c.linhas[0][0].negrito);
    }

    #[test]
    fn acha_arquivo_e_linha() {
        assert_eq!(
            parse_error_location("erro: x em lib/a.lep:12"),
            Some(("lib/a.lep".into(), 12))
        );
        assert_eq!(
            parse_error_location("  0: pega (app.lep:4:1)"),
            Some(("app.lep".into(), 4))
        );
    }

    #[test]
    fn write_screen_like_a_terminal() {
        assert_eq!(tela(&["velho\n", "\x1b[2J\x1b[Hnovo\n"]), "novo\n");
        assert_eq!(tela(&["q1 a\nq1 b\n", "\x1b[H", "q2 a\n"]), "q2 a\nq1 b\n");
        assert_eq!(
            tela(&["q1 a\nq1 b\n", "\x1b[H", "q2 a\n", "q2 b\n"]),
            "q2 a\nq2 b\n"
        );
        assert_eq!(tela(&["\x1b[H", "\x1b[32m*", "\x1b[0m!\n"]), "*!\n");
        assert_eq!(tela(&["\x1b[2J\x1b[H", "a\n", "b\n"]), "a\nb\n");
        assert_eq!(
            tela(&["q1 a\nq1 b\nq1 c\n", "\x1b[H", "q2 a\nq2 b\n", "\x1b[J"]),
            "q2 a\nq2 b\n"
        );
        assert_eq!(
            tela(&["cargo leftover here\n", "\x1b[Hok\x1b[K\n\x1b[J"]),
            "ok\n"
        );
        assert_eq!(tela(&["\x1b[Ha\x1b[K\r\nb\x1b[K\r\n\x1b[J"]), "a\nb\n");
        assert_eq!(tela(&["AAAAAA\n", "\x1b[1;4HXY\x1b[K"]), "AAAXY\n");
        assert_eq!(
            tela(&["AAAAAA\n", "\x1b[", "1", ";", "4", "H", "XY", "\x1b[K"]),
            "AAAXY\n"
        );
        assert_eq!(
            tela(&[
                "\x1b[H", "  |   |", "\x1b[K", "\r\n", "\x1b[", "1", ";", "12", "H", "Q W",
                "\x1b[K",
            ]),
            "  |   |    Q W\n"
        );
    }

    #[test]
    fn csi_prefix_espera_o_final() {
        assert!(csi_incomplete("\u{1b}"));
        assert!(csi_incomplete("\u{1b}["));
        assert!(csi_incomplete("\u{1b}[12;"));
        assert!(!csi_incomplete("\u{1b}[12;1H"));
        assert!(!csi_incomplete("\u{1b}[K"));
        assert!(!csi_incomplete("hello"));
        assert!(!csi_incomplete("\u{1b}x"));
    }
}
