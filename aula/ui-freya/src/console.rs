//! Program output: ANSI colors, cls/quadro, clickable `arquivo:linha`.

pub const MAX_LINHAS: usize = 5000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trecho {
    pub texto: String,
    pub stderr: bool,
    pub fg: Option<(u8, u8, u8)>,
    pub negrito: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Console {
    pub linhas: Vec<Vec<Trecho>>,
    pub leia_prompt: Option<String>,
    pub entrada: String,
}

impl Console {
    pub fn limpar(&mut self) {
        self.linhas.clear();
        self.leia_prompt = None;
        self.entrada.clear();
    }

    pub fn escrever(&mut self, texto: &str, stderr: bool) {
        if texto.contains("\x1b[2J") {
            self.linhas.clear();
        }
        let limpo = texto.replace("\x1b[2J", "").replace("\x1b[H", "");
        for (vis, estilo) in ansi_spans(&limpo) {
            self.acrescentar(&vis, stderr, estilo);
        }
        self.aparar();
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
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AnsiStyle {
    fg: Option<(u8, u8, u8)>,
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

fn ansi_spans(text: &str) -> Vec<(String, AnsiStyle)> {
    let mut out = Vec::new();
    let mut estilo = AnsiStyle::default();
    let mut buf = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        if rest.starts_with("\x1b[")
            && let Some((n, novo)) = parse_sgr(rest)
        {
            if !buf.is_empty() {
                out.push((std::mem::take(&mut buf), estilo));
            }
            estilo = novo;
            rest = &rest[n..];
            continue;
        }
        let c = rest.chars().next().unwrap();
        if c == '\x1b' {
            rest = &rest[c.len_utf8()..];
            continue;
        }
        buf.push(c);
        rest = &rest[c.len_utf8()..];
    }
    if !buf.is_empty() {
        out.push((buf, estilo));
    }
    out
}

fn parse_sgr(s: &str) -> Option<(usize, AnsiStyle)> {
    let rest = s.strip_prefix("\x1b[")?;
    let fim = rest.find('m')?;
    let params = &rest[..fim];
    let mut estilo = AnsiStyle::default();
    if params.is_empty() {
        return Some((3, estilo));
    }
    for p in params.split(';') {
        match p.parse::<u8>().ok()? {
            0 => estilo = AnsiStyle::default(),
            1 => estilo.negrito = true,
            n @ 30..=37 => estilo.fg = Some(CORES[(n - 30) as usize]),
            39 => estilo.fg = None,
            _ => {}
        }
    }
    Some((2 + fim + 1, estilo))
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
}
