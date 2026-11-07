//! Font size, theme and last project, saved in the same `prefs` file as GTK Aula.

use std::path::{Path, PathBuf};

pub const DEFAULT_FONT: u32 = 11;
const MIN_FONT: u32 = 7;
const MAX_FONT: u32 = 40;
const RECENTES_MAXIMO: usize = 10;
pub const DEFAULT_URL: &str = "http://127.0.0.1:50051";

/// A project opened before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recente {
    Local(String),
    Servidor {
        url: String,
        aluno: String,
        projeto: String,
    },
}

impl Recente {
    fn texto(&self) -> String {
        match self {
            Recente::Local(p) => format!("local\t{p}"),
            Recente::Servidor {
                url,
                aluno,
                projeto,
            } => format!("servidor\t{url}\t{aluno}\t{projeto}"),
        }
    }

    fn de_texto(s: &str) -> Option<Self> {
        let partes: Vec<&str> = s.split('\t').collect();
        match partes.as_slice() {
            ["local", p] if !p.is_empty() => Some(Recente::Local(p.to_string())),
            ["servidor", url, aluno, projeto] => Some(Recente::Servidor {
                url: url.to_string(),
                aluno: aluno.to_string(),
                projeto: projeto.to_string(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    pub config_path: Option<PathBuf>,
    pub font_size: u32,
    pub dark: bool,
    pub pasta: String,
    pub servidor: String,
    pub projeto: String,
    pub recentes: Vec<Recente>,
    pub arquivo: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            config_path: None,
            font_size: DEFAULT_FONT,
            dark: false,
            pasta: String::new(),
            servidor: DEFAULT_URL.to_string(),
            projeto: String::new(),
            recentes: Vec::new(),
            arquivo: String::new(),
        }
    }
}

/// Same keys as GTK Aula (`fonte`, `tema`, `pasta`, `arquivo`, …).
impl Prefs {
    pub fn parse(text: &str) -> Self {
        let mut p = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "fonte" => {
                    if let Ok(n) = v.trim().parse::<u32>() {
                        p.font_size = n.clamp(MIN_FONT, MAX_FONT);
                    }
                }
                "tema" => p.dark = v.trim() == "escuro",
                "pasta" => p.pasta = v.trim().to_string(),
                "servidor" => p.servidor = v.trim().to_string(),
                "projeto" => p.projeto = v.trim().to_string(),
                "recente" => p.recentes.extend(Recente::de_texto(v.trim())),
                "arquivo" => p.arquivo = v.trim().to_string(),
                _ => {}
            }
        }
        p.organizar_recentes();
        p
    }

    pub fn to_text(&self) -> String {
        let mut t = format!(
            "fonte={}\ntema={}\npasta={}\nservidor={}\nprojeto={}\narquivo={}\n",
            self.font_size,
            if self.dark { "escuro" } else { "claro" },
            self.pasta,
            self.servidor,
            self.projeto,
            self.arquivo,
        );
        for r in &self.recentes {
            t.push_str(&format!("recente={}\n", r.texto()));
        }
        t
    }

    pub fn lembrar_recente(&mut self, r: Recente) {
        self.recentes.insert(0, r);
        self.organizar_recentes();
    }

    fn organizar_recentes(&mut self) {
        let mut locais = std::collections::HashSet::new();
        let mut servidores = std::collections::HashSet::new();
        self.recentes.retain(|r| match r {
            Recente::Local(p) => locais.len() < RECENTES_MAXIMO && locais.insert(p.clone()),
            Recente::Servidor { url, aluno, .. } => {
                let key = (normalize_url(url), aluno.clone());
                servidores.len() < RECENTES_MAXIMO && servidores.insert(key)
            }
        });
    }

    pub fn load() -> Self {
        let target = config_path();
        let mut prefs = std::fs::read_to_string(&target)
            .map(|t| Self::parse(&t))
            .unwrap_or_default();
        prefs.config_path = Some(target);
        prefs
    }

    pub fn save(&self) {
        let Some(p) = self.config_path.clone() else {
            return;
        };
        if let Some(dir) = p.parent()
            && let Err(e) = std::fs::create_dir_all(dir)
        {
            eprintln!("preferências: {e}");
            return;
        }
        if let Err(e) = std::fs::write(p, self.to_text()) {
            eprintln!("preferências: {e}");
        }
    }

    /// Ctrl+= / Ctrl+- ; 0 resets.
    pub fn zoom(&mut self, step: i32) {
        self.font_size = if step == 0 {
            DEFAULT_FONT
        } else {
            (self.font_size as i32 + step).clamp(MIN_FONT as i32, MAX_FONT as i32) as u32
        };
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("expressa-aula")
        .join("prefs")
}

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir().or_else(|| {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    })
}

/// Lowercase host, add `http://` when missing, drop default ports.
pub fn normalize_url(url: &str) -> String {
    let url = url.trim();
    let url = if url.is_empty() {
        DEFAULT_URL.to_string()
    } else if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{url}")
    };
    let url = url.to_ascii_lowercase();
    let url = url.trim_end_matches('/');
    strip_default_port(url)
}

fn strip_default_port(url: &str) -> String {
    for (scheme, default) in [("http://", "80"), ("https://", "443")] {
        if let Some(rest) = url.strip_prefix(scheme)
            && let Some((host, port)) = rest.rsplit_once(':')
            && port == default
            && !host.contains('/')
            && !host.contains(']')
        {
            return format!("{scheme}{host}");
        }
    }
    url.to_string()
}

/// `~` and `~/…` become the user home directory. `~nome` is left as-is.
pub fn expand_tilde(path: &str) -> PathBuf {
    expand_tilde_with_home(path, home_dir())
}

#[allow(dead_code)]
fn expand_tilde_with_home(path: &str, home: Option<PathBuf>) -> PathBuf {
    if path == "~" {
        return home.unwrap_or_else(|| PathBuf::from("~"));
    }
    let rest = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"));
    if let Some(rest) = rest
        && let Some(home) = home
    {
        return home.join(rest);
    }
    PathBuf::from(path)
}

/// `/home/ana/proj` → `~/proj` when the prefix is this user’s home.
pub fn path_for_display(path: &Path) -> String {
    path_for_display_with_home(path, home_dir())
}

fn path_for_display_with_home(path: &Path, home: Option<PathBuf>) -> String {
    let shown = path.display().to_string();
    let Some(home) = home else {
        return shown;
    };
    let home_s = home.display().to_string();
    if shown == home_s {
        return "~".to_string();
    }
    let sep = std::path::MAIN_SEPARATOR;
    let prefix = format!("{home_s}{sep}");
    if let Some(rest) = shown.strip_prefix(&prefix) {
        return format!("~{sep}{rest}");
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_print() {
        let p = Prefs::parse(
            "fonte=18\ntema=escuro\nprojeto=aula-2\n\
             recente=servidor\thttp://s:1\tana\taula-2\nrecente=local\t/home/ana/x\n\
             recente=lixo\n",
        );
        let expect = Prefs {
            font_size: 18,
            dark: true,
            projeto: "aula-2".into(),
            recentes: vec![
                Recente::Servidor {
                    url: "http://s:1".into(),
                    aluno: "ana".into(),
                    projeto: "aula-2".into(),
                },
                Recente::Local("/home/ana/x".into()),
            ],
            ..Prefs::default()
        };
        assert_eq!(p, expect);
        assert_eq!(Prefs::parse(&p.to_text()), p);
        assert_eq!(Prefs::parse("fonte=999\nlixo\n").font_size, MAX_FONT);
        assert_eq!(Prefs::parse(""), Prefs::default());
    }

    #[test]
    fn legacy_history_groups_servers_and_preserves_latest_project() {
        let p = Prefs::parse(
            "recente=servidor\thttp://SERVER:80/\tana\tlatest\nrecente=local\t/a/project\nrecente=servidor\tserver\tana\told\nrecente=servidor\thttp://server\tbia\tother\nrecente=local\t/a/project\nrecente=local\t/b/project\n",
        );
        assert_eq!(p.recentes.len(), 4);
        assert!(matches!(&p.recentes[0], Recente::Servidor { projeto, .. } if projeto == "latest"));
        assert_eq!(p.recentes[1], Recente::Local("/a/project".into()));
        assert_eq!(p.recentes[3], Recente::Local("/b/project".into()));
        assert_eq!(Prefs::parse(&p.to_text()), p);
    }

    #[test]
    fn retention_is_independent_and_latest_server_use_replaces_project() {
        let mut p = Prefs::default();
        for i in 0..12 {
            p.lembrar_recente(Recente::Local(format!("/p{i}")));
            p.lembrar_recente(Recente::Servidor {
                url: format!("http://s{i}"),
                aluno: "ana".into(),
                projeto: "old".into(),
            });
        }
        assert_eq!(p.recentes.len(), 20);
        p.lembrar_recente(Recente::Servidor {
            url: "http://S11:80/".into(),
            aluno: "ana".into(),
            projeto: "new".into(),
        });
        assert_eq!(p.recentes.len(), 20);
        assert!(matches!(&p.recentes[0], Recente::Servidor { projeto, .. } if projeto == "new"));
        assert_eq!(
            p.recentes
                .iter()
                .filter(|r| matches!(r, Recente::Local(_)))
                .count(),
            10
        );
    }

    #[test]
    fn zoom_steps_and_limits() {
        let mut p = Prefs::default();
        p.zoom(2);
        assert_eq!(p.font_size, DEFAULT_FONT + 2);
        p.zoom(0);
        assert_eq!(p.font_size, DEFAULT_FONT);
        for _ in 0..50 {
            p.zoom(-1);
        }
        assert_eq!(p.font_size, MIN_FONT);
    }

    #[test]
    fn tilde_becomes_home() {
        let home = std::env::temp_dir().join("expressa-aula-tilde-home");
        let expand = |path| expand_tilde_with_home(path, Some(home.clone()));
        assert_eq!(expand("~"), home);
        assert_eq!(expand("~/proj/a"), home.join("proj/a"));
        assert_eq!(expand("/abs"), PathBuf::from("/abs"));
        assert_eq!(expand("~nao"), PathBuf::from("~nao"));
        let proj = home.join("proj");
        assert_eq!(
            path_for_display_with_home(&proj, Some(home)),
            format!("~{}proj", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(expand_tilde_with_home("~", None), PathBuf::from("~"));
    }

    #[test]
    fn config_path_uses_expressa_aula() {
        let p = config_path();
        assert!(p.ends_with("expressa-aula/prefs") || p.ends_with(r"expressa-aula\prefs"));
    }

    #[test]
    fn expand_tilde_keeps_named_home() {
        assert_eq!(expand_tilde("~nao"), PathBuf::from("~nao"));
    }
}
