//! Expressa Aula on Freya. Ships beside the GTK `expressa-aula` binary.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod atalhos;
mod busca;
mod comandos;
mod documentos;
mod edicao;
mod editor;
mod indent;
mod lang;
mod prefs;
mod shell;

use freya::{
    code_editor::CodeEditor,
    prelude::*,
    text_edit::TextEditor,
};

use crate::{
    atalhos::{Acao, acao_de_tecla},
    comandos::{
        achar, aplicar, confirmar_descarte, fechar_aba, ir_para_linha, selecionar_aba,
    },
    documentos::{Abas, Depois, Dialogo, gravar_rascunhos, sincronizar},
    edicao::tratar_tecla,
    editor::{FONT_FAMILY, TEXTO_INICIAL, fonte_px, mensagem_sintaxe},
    prefs::Prefs,
    shell::{
        MenuAberto, Paleta, barra_abas, barra_busca, barra_ferramentas, barra_menus, barra_status,
        dialogo_confirmar, dialogo_linha, editor_inicial, painel_atalhos, tema_freya,
    },
};

const TITULO: &str = "Expressa Aula";

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio");
    let _rt = rt.enter();
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
    let mut editor = use_state(|| editor_inicial(TEXTO_INICIAL, prefs_iniciais.dark));
    let status = use_state(String::new);
    let menu = use_state(MenuAberto::default);
    let mut atalhos_visiveis = use_state(|| false);
    let mut abas = use_state(|| Abas::nova(TEXTO_INICIAL));
    let dialogo = use_state(Dialogo::default);
    let linha_campo = use_state(String::new);
    let busca_aberta = use_state(|| false);
    let busca_termo = use_state(String::new);
    let mut sintaxe = use_state(String::new);

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            sincronizar(&mut abas.write(), &editor.read());
            gravar_rascunhos(&abas.read());
        }
    });

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let texto = editor.read().rope.to_string();
            sintaxe.set(mensagem_sintaxe(&texto));
        }
    });

    let dark = prefs.read().dark;
    let paleta = Paleta::de(dark);
    let font_size = fonte_px(prefs.read().font_size);
    let pasta = prefs.read().pasta.clone();
    let recado = {
        let s = status.read().clone();
        let e = sintaxe.read().clone();
        if e.is_empty() {
            s
        } else if s.is_empty() {
            e
        } else {
            format!("{e}  ·  {s}")
        }
    };
    let linha = editor.read().cursor_row();
    let coluna = editor.read().cursor_col();
    let mut abas_agora = abas.read().clone();
    abas_agora.atual_mut().dirty = editor.read().is_edited();
    let dialogo_agora = dialogo.read().clone();

    let on_acao = move |acao: Acao| {
        aplicar(
            acao,
            prefs,
            editor,
            status,
            theme,
            atalhos_visiveis,
            abas,
            dialogo,
            busca_aberta,
            busca_termo,
        );
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
        .child(barra_abas(
            &abas_agora,
            paleta,
            move |idx| selecionar_aba(idx, editor, prefs, abas),
            move |id| {
                fechar_aba(
                    Some(id),
                    Depois::SoFechar,
                    editor,
                    prefs,
                    abas,
                    status,
                    dialogo,
                )
            },
        ))
        .child(
            rect()
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    CodeEditor::new(editor, a11y_id)
                        .font_size(font_size)
                        .font_family(FONT_FAMILY)
                        .a11y_auto_focus(true)
                        .on_pre_key_down(move |e: Event<KeyboardEventData>| {
                            tratar_tecla(&e, &mut editor.write())
                        }),
                ),
        )
        .maybe_child(busca_aberta().then(move || {
            barra_busca(
                paleta,
                busca_termo,
                move || achar(true, editor, status, busca_termo),
                move || achar(false, editor, status, busca_termo),
                move || {
                    let mut busca_aberta = busca_aberta;
                    busca_aberta.set(false);
                },
            )
        }))
        .child(barra_status(linha, coluna, &pasta, &recado, paleta))
        .maybe_child(atalhos_visiveis().then(move || {
            painel_atalhos(paleta, move || atalhos_visiveis.set(false))
        }))
        .maybe_child(matches!(dialogo_agora, Dialogo::Confirmar { .. }).then(move || {
            dialogo_confirmar(
                paleta,
                move || confirmar_descarte(true, editor, prefs, abas, status, dialogo),
                move || confirmar_descarte(false, editor, prefs, abas, status, dialogo),
                move || {
                    let mut dialogo = dialogo;
                    dialogo.set(Dialogo::Nenhum);
                },
            )
        }))
        .maybe_child(matches!(dialogo_agora, Dialogo::IrParaLinha).then(move || {
            dialogo_linha(
                paleta,
                linha_campo,
                move || ir_para_linha(&linha_campo.read(), editor, status, dialogo),
                move || {
                    let mut dialogo = dialogo;
                    dialogo.set(Dialogo::Nenhum);
                },
            )
        }))
}
