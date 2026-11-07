//! Same shortcuts as `aula/README.md` / GTK `atalhos.rs`.

use freya::prelude::{Key, Modifiers, NamedKey};

/// IDE command. Later stages fill in the no-ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acao {
    Novo,
    Abrir,
    AbrirProjeto,
    Servidor,
    Salvar,
    SalvarComo,
    FecharAba,
    AbaSeguinte,
    AbaAnterior,
    Sair,
    Executar,
    Parar,
    Depurar,
    Proximo,
    Entrar,
    SairFuncao,
    Ponto,
    MostrarArquivos,
    MostrarPainel,
    TemaClaro,
    TemaEscuro,
    ZoomMais,
    ZoomMenos,
    ZoomNormal,
    IrParaLinha,
    Buscar,
    BuscarProxima,
    Substituir,
    Escapar,
    Comentar,
    Duplicar,
    Recuar,
    MoverCima,
    MoverBaixo,
    Ajuda,
    Completar,
    IrDefinicao,
    Catalogo,
    Atalhos,
    Tutoriais,
    Repl,
}

/// Sections of (keys, what they do). Keep in step with [`acao_de_tecla`].
pub const ATALHOS: &[(&str, &[(&str, &str)])] = &[
    (
        "Arquivos e abas",
        &[
            ("Ctrl+N", "arquivo novo"),
            ("Ctrl+S", "salvar"),
            ("Ctrl+W", "fechar a aba"),
            ("Ctrl+Tab · Ctrl+PgDn", "próxima aba"),
            ("Ctrl+Shift+Tab · Ctrl+PgUp", "aba anterior"),
            ("Ctrl+Q", "sair"),
        ],
    ),
    (
        "Editar",
        &[
            ("Ctrl+/", "comentar ou descomentar as linhas"),
            ("Ctrl+D", "duplicar as linhas"),
            ("Ctrl+Espaço", "mostrar sugestões de código"),
            ("Ctrl+Z · Ctrl+Shift+Z", "desfazer · refazer"),
        ],
    ),
    (
        "Buscar e navegar",
        &[
            ("Ctrl+F", "buscar no arquivo"),
            ("Ctrl+H", "substituir no arquivo"),
            ("Ctrl+G", "ir para a linha"),
            ("F1", "ajuda sobre o nome sob o cursor"),
        ],
    ),
    (
        "Executar e depurar",
        &[
            ("F5", "executar · continuar quando pausado no depurador"),
            ("F6", "depurar"),
            ("F9", "ponto de parada"),
            ("F10", "próxima linha"),
            ("F11 · Shift+F11", "entrar na função · sair dela"),
        ],
    ),
    (
        "Ver",
        &[
            ("F7", "mostrar ou esconder os arquivos"),
            ("F8", "mostrar ou esconder o painel"),
            ("Ctrl+= · Ctrl+- · Ctrl+0", "letra maior · menor · normal"),
            ("Ctrl+?", "esta lista"),
        ],
    ),
];

pub fn ctrl(modifiers: Modifiers) -> bool {
    modifiers.contains(Modifiers::CONTROL) || modifiers.contains(Modifiers::META)
}

pub fn acao_de_tecla(key: &Key, modifiers: Modifiers) -> Option<Acao> {
    let shift = modifiers.contains(Modifiers::SHIFT);
    let alt = modifiers.contains(Modifiers::ALT);
    let control = ctrl(modifiers);

    match key {
        Key::Named(NamedKey::F5) if !control && !alt => Some(Acao::Executar),
        Key::Named(NamedKey::F6) if !control && !alt => Some(Acao::Depurar),
        Key::Named(NamedKey::F7) if !control && !alt => Some(Acao::MostrarArquivos),
        Key::Named(NamedKey::F8) if !control && !alt => Some(Acao::MostrarPainel),
        Key::Named(NamedKey::F9) if !control && !alt => Some(Acao::Ponto),
        Key::Named(NamedKey::F10) if !control && !alt => Some(Acao::Proximo),
        Key::Named(NamedKey::F11) if !control && !alt && shift => Some(Acao::SairFuncao),
        Key::Named(NamedKey::F11) if !control && !alt => Some(Acao::Entrar),
        Key::Named(NamedKey::F1) if !control && !alt => Some(Acao::Ajuda),
        Key::Named(NamedKey::F12) if !control && !alt => Some(Acao::IrDefinicao),
        Key::Named(NamedKey::Tab) if control && !alt && shift => Some(Acao::AbaAnterior),
        Key::Named(NamedKey::Tab) if control && !alt => Some(Acao::AbaSeguinte),
        Key::Named(NamedKey::PageDown) if control && !alt => Some(Acao::AbaSeguinte),
        Key::Named(NamedKey::PageUp) if control && !alt => Some(Acao::AbaAnterior),
        Key::Named(NamedKey::Escape) => Some(Acao::Escapar),
        Key::Named(NamedKey::Enter) if control && !alt => Some(Acao::BuscarProxima),
        Key::Named(NamedKey::ArrowUp) if alt && !control => Some(Acao::MoverCima),
        Key::Named(NamedKey::ArrowDown) if alt && !control => Some(Acao::MoverBaixo),
        Key::Character(c) if control && alt && (c == "\\" || c == "|") => Some(Acao::Recuar),
        Key::Character(c) if control && !alt => match c.to_lowercase().as_str() {
            "n" if !shift => Some(Acao::Novo),
            "s" if !shift => Some(Acao::Salvar),
            "s" if shift => Some(Acao::SalvarComo),
            "w" if !shift => Some(Acao::FecharAba),
            "q" if !shift => Some(Acao::Sair),
            " " if !shift => Some(Acao::Completar),
            "f" if !shift => Some(Acao::Buscar),
            "h" if !shift => Some(Acao::Substituir),
            "g" if !shift => Some(Acao::IrParaLinha),
            "/" if !shift => Some(Acao::Comentar),
            "d" if !shift => Some(Acao::Duplicar),
            "0" => Some(Acao::ZoomNormal),
            "-" | "_" => Some(Acao::ZoomMenos),
            "=" | "+" => Some(Acao::ZoomMais),
            "?" => Some(Acao::Atalhos),
            _ => None,
        },
        _ => None,
    }
}

/// F5–F11 and Esc still work while a text field or dialog has focus.
pub fn acao_vale_em_campo(acao: Acao) -> bool {
    matches!(
        acao,
        Acao::Executar
            | Acao::Parar
            | Acao::Depurar
            | Acao::Proximo
            | Acao::Entrar
            | Acao::SairFuncao
            | Acao::Ponto
            | Acao::MostrarArquivos
            | Acao::MostrarPainel
            | Acao::Escapar
            | Acao::AbaSeguinte
            | Acao::AbaAnterior
    )
}

pub fn acao_de_tecla_com_foco(key: &Key, modifiers: Modifiers, em_campo: bool) -> Option<Acao> {
    let acao = acao_de_tecla(key, modifiers)?;
    if em_campo && !acao_vale_em_campo(acao) {
        None
    } else {
        Some(acao)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tecla(c: &str, mods: Modifiers) -> Option<Acao> {
        acao_de_tecla(&Key::Character(c.into()), mods)
    }

    #[test]
    fn atalhos_principais() {
        let ctrl = Modifiers::CONTROL;
        assert_eq!(tecla("s", ctrl), Some(Acao::Salvar));
        assert_eq!(tecla("S", ctrl | Modifiers::SHIFT), Some(Acao::SalvarComo));
        assert_eq!(tecla("q", ctrl), Some(Acao::Sair));
        assert_eq!(tecla("=", ctrl), Some(Acao::ZoomMais));
        assert_eq!(tecla("-", ctrl), Some(Acao::ZoomMenos));
        assert_eq!(tecla("0", ctrl), Some(Acao::ZoomNormal));
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::F5), Modifiers::empty()),
            Some(Acao::Executar)
        );
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::F11), Modifiers::SHIFT),
            Some(Acao::SairFuncao)
        );
        assert_eq!(tecla("s", Modifiers::empty()), None);
        assert_eq!(tecla("/", ctrl), Some(Acao::Comentar));
        assert_eq!(tecla("f", ctrl), Some(Acao::Buscar));
        assert_eq!(tecla("h", ctrl), Some(Acao::Substituir));
        assert_eq!(tecla(" ", ctrl), Some(Acao::Completar));
        assert_eq!(tecla("d", ctrl), Some(Acao::Duplicar));
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::Tab), ctrl),
            Some(Acao::AbaSeguinte)
        );
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::Tab), ctrl | Modifiers::SHIFT),
            Some(Acao::AbaAnterior)
        );
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::PageDown), ctrl),
            Some(Acao::AbaSeguinte)
        );
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::PageUp), ctrl),
            Some(Acao::AbaAnterior)
        );
        assert_eq!(
            acao_de_tecla(&Key::Named(NamedKey::Escape), Modifiers::empty()),
            Some(Acao::Escapar)
        );
        assert_eq!(
            acao_de_tecla_com_foco(&Key::Character("s".into()), ctrl, true),
            None
        );
        assert_eq!(
            acao_de_tecla_com_foco(&Key::Character("w".into()), ctrl, true),
            None
        );
        assert_eq!(
            acao_de_tecla_com_foco(&Key::Character("d".into()), ctrl, true),
            None
        );
        assert_eq!(
            acao_de_tecla_com_foco(&Key::Named(NamedKey::F5), Modifiers::empty(), true),
            Some(Acao::Executar)
        );
        assert_eq!(
            acao_de_tecla_com_foco(&Key::Named(NamedKey::Escape), Modifiers::empty(), true),
            Some(Acao::Escapar)
        );
        assert_eq!(
            acao_de_tecla_com_foco(&Key::Character("s".into()), ctrl, false),
            Some(Acao::Salvar)
        );
    }

    #[test]
    fn tabela_tem_os_menus_da_aula() {
        let titulos: Vec<&str> = ATALHOS.iter().map(|(s, _)| *s).collect();
        assert!(titulos.contains(&"Arquivos e abas"));
        assert!(titulos.contains(&"Executar e depurar"));
        assert!(titulos.contains(&"Ver"));
    }
}
