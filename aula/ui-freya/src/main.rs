//! Expressa Aula on Freya. Ships beside the GTK `expressa-aula` binary.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod atalhos;
mod busca;
mod comandos;
mod console;
mod documentos;
mod execucao;
mod edicao;
mod editor;
mod indent;
mod lang;
mod prefs;
mod projeto;
mod shell;

use freya::{
    code_editor::CodeEditor,
    prelude::*,
    text_edit::TextEditor,
};

use crate::{
    atalhos::{Acao, acao_de_tecla},
    comandos::{
        achar, aplicar, aplicar_eventos, clicar_arvore, confirmar_apagar, confirmar_descarte,
        confirmar_nome, escolher_projeto, fechar_aba, ir_para_linha, ir_para_saida,
        pasta_selecionada, pedir_apagar, pedir_nome, responder_leia, restaurar_arquivo,
        selecionar_aba,
    },
    console::Console,
    execucao::Execucao,
    documentos::{Abas, Depois, DepoisNome, Dialogo, gravar_rascunhos, sincronizar},
    edicao::tratar_tecla,
    editor::{FONT_FAMILY, TEXTO_INICIAL, fonte_px, mensagem_sintaxe},
    prefs::Prefs,
    projeto::{Projeto, abrir_raiz, instalar_exemplos, restaurar_inicio},
    shell::{
        MenuAberto, Paleta, barra_abas, barra_argumentos, barra_busca, barra_ferramentas,
        barra_menus, barra_status, dialogo_apagar, dialogo_confirmar, dialogo_linha, dialogo_nome,
        editor_inicial, painel_arvore, painel_atalhos, painel_console, tema_freya,
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
    let mut prefs = use_state(|| prefs_iniciais.clone());
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
    let mut projeto = use_state(Projeto::default);
    let nome_campo = use_state(String::new);
    let mut exec = use_state(Execucao::default);
    let console = use_state(Console::default);
    let args = use_state(String::new);
    let leia_entrada = use_state(String::new);

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

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            let eventos = exec.write().drenar();
            if !eventos.is_empty() {
                aplicar_eventos(eventos, console, status, exec);
            }
        }
    });

    use_future(move || async move {
        let _ = instalar_exemplos();
        let pasta = restaurar_inicio(&prefs.read());
        if let Some(pasta) = pasta {
            let _ = abrir_raiz(&mut projeto.write(), &mut prefs.write(), pasta);
            restaurar_arquivo(editor, prefs, abas, status);
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
    let projeto_agora = projeto.read().clone();

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
            projeto,
            exec,
            console,
            args,
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
        .child(barra_argumentos(paleta, args))
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
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .height(Size::fill())
                .maybe_child(projeto_agora.visivel.then(move || {
                    painel_arvore(
                        projeto_agora,
                        paleta,
                        move |rel| clicar_arvore(rel, editor, prefs, abas, status, projeto),
                        move || {
                            pedir_nome(
                                DepoisNome::NovoArquivo(pasta_selecionada(projeto)),
                                dialogo,
                                nome_campo,
                            )
                        },
                        move || {
                            pedir_nome(
                                DepoisNome::NovaPasta(pasta_selecionada(projeto)),
                                dialogo,
                                nome_campo,
                            )
                        },
                        move || {
                            if let Some(rel) = projeto.read().selecionado.clone() {
                                pedir_nome(DepoisNome::Renomear(rel), dialogo, nome_campo);
                            }
                        },
                        move || pedir_apagar(dialogo, projeto),
                        move || escolher_projeto(editor, prefs, abas, status, projeto),
                    )
                }))
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
        .child(painel_console(
            console.read().clone(),
            paleta,
            leia_entrada,
            move || responder_leia(exec, console, leia_entrada),
            move |linha| {
                ir_para_saida(&linha, editor, prefs, abas, status, projeto, dialogo)
            },
        ))
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
        .maybe_child(
            if let Dialogo::Nome { titulo, .. } = &dialogo_agora {
                Some(titulo.clone())
            } else {
                None
            }
            .map(|titulo| {
                dialogo_nome(
                    titulo,
                    nome_campo,
                    move || confirmar_nome(&nome_campo.read(), editor, prefs, abas, status, dialogo, projeto),
                    move || {
                        let mut dialogo = dialogo;
                        dialogo.set(Dialogo::Nenhum);
                    },
                )
            }),
        )
        .maybe_child(
            if let Dialogo::Apagar { rel } = &dialogo_agora {
                Some(rel.clone())
            } else {
                None
            }
            .map(|rel| {
                dialogo_apagar(
                    rel,
                    move || confirmar_apagar(status, dialogo, projeto),
                    move || {
                        let mut dialogo = dialogo;
                        dialogo.set(Dialogo::Nenhum);
                    },
                )
            }),
        )
}
