//! Expressa Aula on Freya. Ships beside the GTK `expressa-aula` binary.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod atalhos;
mod editor;
mod prefs;
mod shell;

use freya::{
    code_editor::CodeEditor,
    prelude::*,
    text_edit::TextEditor,
};

use crate::{
    atalhos::{Acao, acao_de_tecla},
    editor::{FONT_FAMILY, TEXTO_INICIAL, fonte_px},
    prefs::Prefs,
    shell::{
        MenuAberto, Paleta, aplicar_acao, barra_ferramentas, barra_menus, barra_status,
        editor_inicial, painel_atalhos, tema_freya,
    },
};

const TITULO: &str = "Expressa Aula";

fn main() {
    launch(LaunchConfig::new().with_window(
        WindowConfig::new(app)
            .with_title(TITULO)
            .with_size(1200.0, 800.0)
            .with_app_id("dev.expressa.aula")
            .with_window_handle(|w| w.set_maximized(true)),
    ));
}

fn app() -> impl IntoElement {
    let prefs_iniciais = Prefs::load();
    let theme = use_init_theme(|| tema_freya(prefs_iniciais.dark));
    let a11y_id = use_a11y();
    let prefs = use_state(|| prefs_iniciais.clone());
    let editor = use_state(|| editor_inicial(TEXTO_INICIAL, prefs_iniciais.dark));
    let status = use_state(String::new);
    let menu = use_state(MenuAberto::default);
    let mut atalhos_visiveis = use_state(|| false);

    let dark = prefs.read().dark;
    let paleta = Paleta::de(dark);
    let font_size = fonte_px(prefs.read().font_size);
    let pasta = prefs.read().pasta.clone();
    let recado = status.read().clone();
    let linha = editor.read().cursor_row();
    let coluna = editor.read().cursor_col();

    let on_acao = move |acao: Acao| {
        aplicar_acao(acao, prefs, editor, status, theme, atalhos_visiveis);
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(paleta.fundo)
        .color(paleta.texto)
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            if let Some(acao) = acao_de_tecla(&e.key, e.modifiers) {
                e.stop_propagation();
                on_acao(acao);
            }
        })
        .child(barra_menus(menu, paleta, on_acao))
        .child(barra_ferramentas(paleta, on_acao))
        .child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    CodeEditor::new(editor, a11y_id)
                        .font_size(font_size)
                        .font_family(FONT_FAMILY)
                        .a11y_auto_focus(true),
                ),
        )
        .child(barra_status(linha, coluna, &pasta, &recado, paleta))
        .maybe_child(atalhos_visiveis().then(move || {
            painel_atalhos(paleta, move || atalhos_visiveis.set(false))
        }))
}
