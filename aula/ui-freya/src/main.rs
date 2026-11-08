//! Expressa Aula on Freya. Ships beside the GTK `expressa-aula` binary.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod ajuda;
mod atalhos;
mod busca;
mod comandos;
mod console;
mod debug;
mod documentos;
mod execucao;
mod edicao;
mod fundo;
mod grafico;
mod editor;
mod indent;
mod lang;
mod prefs;
mod projeto;
mod repl;
mod shell;
mod terminal_aula;
mod turma;

use freya::{
    code_editor::CodeEditor,
    prelude::*,
    text_edit::TextEditor,
};

use crate::{
    atalhos::{Acao, acao_de_tecla_com_foco},
    comandos::{
        achar, adicionar_observado, abrir_projeto_servidor, aplicar, aplicar_eventos, aplicar_fundo,
        atualizar_arvore,
        substituir_todas_no_arquivo, substituir_uma,
        aplicar_sugestao, abrir_recente, alternar_ponto_em, clicar_arvore, confirmar_apagar, confirmar_descarte, confirmar_nome,
        conectar_servidor, criar_projeto_servidor, descartar_rascunhos, escolher_projeto,
        fechar_dialogo_rascunhos,
        fechar_aba, destacar_graficos, enviar_repl, fechar_repl, garantir_janelas_graf,
        ir_para_linha, ir_para_pausa, ir_para_saida, limpar_repl, parar_repl, pasta_selecionada,
        menu_contexto_arvore, pedir_apagar, pedir_nome, recuperar_rascunhos, reiniciar_repl,
        remover_observado,
        resolver_conflito, restaurar_arquivo, selecionar_aba,
    },
    ajuda::{Sugestao, assinatura, catalogo, ficha},
    console::Console,
    debug::Depuracao,
    execucao::Execucao,
    fundo::Fundo,
    grafico::{Graficos, JanelasGraf},
    repl::Repl,
    documentos::{Abas, Depois, DepoisNome, Dialogo, gravar_rascunhos, sincronizar, titulo_da_janela},
    execucao::caminho_relativo,
    turma::Turma,
    edicao::{reindentar_depois, tecla_pede_indent, tratar_tecla},
    editor::{TEXTO_INICIAL, erro_sintaxe, fonte_familia, fonte_px, instalar_fonte},
    prefs::Prefs,
    projeto::{Projeto, abrir_raiz, instalar_exemplos, restaurar_inicio},
    shell::{
        MenuAberto, Paleta, barra_abas, barra_argumentos, barra_busca,
        ALTURA_CONSOLE, LARGURA_ARVORE, LARGURA_DEBUG, alca_horizontal, alca_vertical, aplicar_arrasto,
        aplicar_arrasto_v, barra_menus, barra_status,
        MarcasVer,
        linha_gutter,
        FocoCampos,
        dialogo_apagar, dialogo_comparar, dialogo_confirmar, dialogo_conflito, dialogo_linha,
        dialogo_nome, dialogo_projetos, dialogo_recuperar, dialogo_servidor, editor_inicial,
        painel_arvore, painel_atalhos, painel_catalogo, painel_console, painel_debug, painel_ficha,
        painel_assinatura, painel_graficos, painel_repl, painel_sugestoes, tema_freya,
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
    let prefs_iniciais = {
        let mut p = Prefs::load();
        let familia = instalar_fonte((!p.familia.is_empty()).then_some(p.familia.as_str()));
        if p.familia != familia {
            p.familia = familia.to_string();
            p.save();
        }
        p
    };
    let theme = use_init_theme(|| tema_freya(prefs_iniciais.dark));
    let a11y_id = use_a11y();
    let mut prefs = use_state(|| prefs_iniciais.clone());
    let mut editor = use_state(|| editor_inicial(TEXTO_INICIAL, prefs_iniciais.dark));
    let status = use_state(String::new);
    let menu = use_state(MenuAberto::default);
    let mut atalhos_visiveis = use_state(|| false);
    let mut abas = use_state(|| Abas::nova(TEXTO_INICIAL));
    let mut dialogo = use_state(Dialogo::default);
    let linha_campo = use_state(String::new);
    let busca_aberta = use_state(|| false);
    let busca_termo = use_state(String::new);
    let substituir_aberta = use_state(|| false);
    let substituir_por = use_state(String::new);
    let mut sintaxe_linha = use_state(|| None::<u32>);
    let mut projeto = use_state(Projeto::default);
    let nome_campo = use_state(String::new);
    let mut exec = use_state(Execucao::default);
    let nucleo_term = use_state(|| {
        let (ponte, backend) = crate::terminal_aula::criar();
        let handle = freya::terminal::TerminalHandle::new(
            freya::terminal::TerminalId::new(),
            backend,
            Some(5000),
        )
        .ok();
        (ponte, handle)
    });
    let console = use_state({
        let ponte = nucleo_term.read().0.clone();
        move || {
            let mut c = Console::default();
            c.ponte = ponte;
            c
        }
    });
    let args = use_state(String::new);
    let ajuda = use_state(|| None::<String>);
    let catalogo_aberto = use_state(|| false);
    let sugestoes = use_state(Vec::<Sugestao>::new);
    let mut assinatura_oculta = use_state(|| false);
    let depuracao = use_state(Depuracao::default);
    let observado_campo = use_state(String::new);
    let mut repl = use_state(Repl::default);
    let repl_entrada = use_state(String::new);
    let graficos = use_state(Graficos::default);
    let janelas = use_state(JanelasGraf::default);
    let turma = use_state(|| {
        let mut t = Turma::default();
        t.url = prefs_iniciais.servidor.clone();
        t
    });
    let mut indentar_pendente = use_state(|| false);
    let mut largura_arvore = use_state(|| LARGURA_ARVORE);
    let mut largura_debug = use_state(|| LARGURA_DEBUG);
    let mut altura_console = use_state(|| ALTURA_CONSOLE);
    let mut arrasto_arvore = use_state(|| None::<f32>);
    let mut arrasto_debug = use_state(|| None::<f32>);
    let mut arrasto_console = use_state(|| None::<f32>);
    let fundo = use_provide_root_context(Fundo::default);
    let foco_campos = use_provide_root_context(FocoCampos::default);
    let scroll_editor = use_scroll_controller(ScrollConfig::default);
    let servidor_campo = use_state(|| prefs_iniciais.servidor.clone());
    let aluno_campo = use_state(|| "local".to_string());

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            sincronizar(&mut abas.write(), &editor.read());
            gravar_rascunhos(&abas.read());
        }
    });

    use_future(move || async move {
        let mut ultimo = String::new();
        loop {
            let texto = editor.read().rope.to_string();
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            let agora = editor.read().rope.to_string();
            if agora != texto || agora == ultimo {
                continue;
            }
            ultimo = agora.clone();
            let linha = tokio::task::spawn_blocking(move || erro_sintaxe(&agora).map(|(l, _)| l))
                .await
                .ok()
                .flatten();
            sintaxe_linha.set(linha);
        }
    });

    use_future({
        let fundo = fundo.clone();
        move || {
            let fundo = fundo.clone();
            async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            let eventos = exec.write().drenar();
            if !eventos.is_empty() {
                aplicar_eventos(
                    eventos,
                    console,
                    status,
                    exec,
                    depuracao,
                    editor,
                    prefs,
                    abas,
                    projeto,
                    dialogo,
                    graficos,
                );
            }
            let pendentes = fundo.drenar();
            if !pendentes.is_empty() {
                aplicar_fundo(
                    pendentes,
                    editor,
                    prefs,
                    abas,
                    status,
                    projeto,
                    turma,
                    dialogo,
                );
            }
            if assinatura_oculta() {
                let texto = editor.read().rope.to_string();
                let byte = crate::editor::byte_do_cursor(&editor.read());
                if assinatura(&texto, byte).is_none() {
                    assinatura_oculta.set(false);
                }
            }
            garantir_janelas_graf(graficos, exec, janelas, Paleta::de(prefs.read().dark));
        }
            }
        }
    });

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            if repl.read().visivel || repl.read().ocupado {
                repl.write().drenar();
            }
        }
    });

    use_future(move || async move {
        let _ = instalar_exemplos();
        let pasta = restaurar_inicio(&prefs.read());
        if let Some(pasta) = pasta {
            let _ = abrir_raiz(&mut projeto.write(), &mut prefs.write(), pasta);
            restaurar_arquivo(editor, prefs, abas, status, projeto);
        }
        let rascunhos = crate::turma::listar_rascunhos();
        if !rascunhos.is_empty() {
            dialogo.set(Dialogo::Recuperar { itens: rascunhos });
        }
    });

    if *indentar_pendente.read() {
        indentar_pendente.set(false);
        reindentar_depois(&mut editor.write());
    }

    let dark = prefs.read().dark;
    let paleta = Paleta::de(dark);
    let font_size = fonte_px(prefs.read().font_size);
    let pasta = {
        let t = turma.read();
        if t.conectado && !t.projeto.is_empty() {
            format!("{} — {}/{}", t.url, t.aluno, t.projeto)
        } else {
            prefs.read().pasta.clone()
        }
    };
    let recado = status.read().clone();
    let linha_erro = *sintaxe_linha.read();
    let linha = editor.read().cursor_row();
    let coluna = editor.read().cursor_col();
    let mut abas_agora = abas.read().clone();
    abas_agora.atual_mut().dirty = editor.read().is_edited();
    Platform::get().set_window_title(Platform::window_id(), titulo_da_janela(abas_agora.atual()));
    let dialogo_agora = dialogo.read().clone();
    let projeto_agora = projeto.read().clone();
    let depuracao_agora = depuracao.read().clone();
    let repl_agora = repl.read().clone();
    let graficos_agora = graficos.read().clone();
    let graficos_dock = graficos_agora.clone();
    let n_linhas = editor.read().rope.len_lines().max(1);
    let line_height = (font_size * 1.4).floor();
    let gutter_w = font_size * 5.0;
    let arquivo_gutter = caminho_relativo(
        projeto_agora.raiz.as_deref().unwrap_or(std::path::Path::new(".")),
        abas_agora.atual().caminho.as_deref(),
    );
    let pontos_gutter = depuracao_agora.pontos.clone();
    let pausado_gutter = depuracao_agora.pausado;
    let pausa_arq = depuracao_agora.arquivo.clone();
    let pausa_linha = depuracao_agora.linha;
    let par_linhas = {
        let texto = editor.read().rope.to_string();
        let byte = crate::editor::byte_do_cursor(&editor.read());
        crate::indent::linhas_do_par(&texto, byte)
    };

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
            substituir_aberta,
            substituir_por,
            projeto,
            exec,
            console,
            args,
            ajuda,
            catalogo_aberto,
            sugestoes,
            depuracao,
            repl,
            graficos,
            turma,
            assinatura_oculta,
        );
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .content(Content::flex())
        .background(paleta.fundo)
        .color(paleta.texto)
        .child(ContextMenuViewer::new())
        .on_global_key_down({
            let dialogo_atalho = dialogo_agora.clone();
            let foco_campos = foco_campos.clone();
            move |e: Event<KeyboardEventData>| {
                let em_campo = foco_campos.tem_foco()
                    || dialogo_atalho != Dialogo::Nenhum
                    || console.read().ponte.leia_aberta();
                if let Some(acao) = acao_de_tecla_com_foco(&e.key, e.modifiers, em_campo) {
                    e.stop_propagation();
                    on_acao(acao);
                }
            }
        })
        .on_global_pointer_move(move |e: Event<PointerEventData>| {
            let loc = e.data().global_location();
            let x = loc.x as f32;
            let y = loc.y as f32;
            if let Some(w) = aplicar_arrasto(*arrasto_arvore.read(), x, false) {
                largura_arvore.set(w);
            }
            if let Some(w) = aplicar_arrasto(*arrasto_debug.read(), x, true) {
                largura_debug.set(w);
            }
            if let Some(h) = aplicar_arrasto_v(*arrasto_console.read(), y) {
                altura_console.set(h);
            }
        })
        .on_global_pointer_up(move |_| {
            arrasto_arvore.set(None);
            arrasto_debug.set(None);
            arrasto_console.set(None);
        })
        .child(barra_menus(
            menu,
            paleta,
            on_acao,
            move |r| {
                abrir_recente(r, editor, prefs, abas, status, projeto, turma, dialogo)
            },
            prefs.read().recentes.clone(),
            MarcasVer {
                arquivos: projeto_agora.visivel,
                painel: depuracao_agora.visivel,
                escuro: dark,
                repl: repl_agora.visivel,
            },
        ))
        .child(barra_argumentos(paleta, args, on_acao))
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
                .height(Size::flex(1.0))
                .maybe_child(projeto_agora.visivel.then(move || {
                    rect()
                        .direction(Direction::Horizontal)
                        .height(Size::fill())
                        .child(painel_arvore(
                            projeto_agora,
                            paleta,
                            *largura_arvore.read(),
                            move |rel| clicar_arvore(rel, editor, prefs, abas, status, projeto, turma),
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
                            move || escolher_projeto(editor, prefs, abas, status, projeto, turma),
                            move || atualizar_arvore(projeto, turma, status),
                            move |rel| menu_contexto_arvore(rel, projeto, dialogo, nome_campo),
                        ))
                        .child(alca_vertical(paleta, largura_arvore, false, arrasto_arvore))
                }))
                .child(
                    rect()
                        .direction(Direction::Vertical)
                        .width(Size::fill())
                        .height(Size::fill())
                        .content(Content::flex())
                        .child(
                            rect()
                                .direction(Direction::Horizontal)
                                .width(Size::fill())
                                .height(Size::flex(1.0))
                                .min_height(Size::px(0.0))
                                .max_height(Size::fill())
                                .overflow(Overflow::Clip)
                                .child({
                                    let arquivo = arquivo_gutter.clone();
                                    let pontos = pontos_gutter.clone();
                                    let pausa_arq = pausa_arq.clone();
                                    VirtualScrollView::new(move |item, _| {
                                        let i = item.index;
                                        let par = par_linhas.is_some_and(|(a, b)| i == a || i == b);
                                        let erro = linha_erro.is_some_and(|l| i + 1 == l as usize);
                                        linha_gutter(
                                            i,
                                            &arquivo,
                                            &pontos,
                                            pausado_gutter,
                                            &pausa_arq,
                                            pausa_linha,
                                            par,
                                            erro,
                                            font_size,
                                            line_height,
                                            paleta,
                                            move |linha| {
                                                alternar_ponto_em(
                                                    linha,
                                                    editor,
                                                    prefs,
                                                    abas,
                                                    status,
                                                    projeto,
                                                    exec,
                                                    depuracao,
                                                )
                                            },
                                        )
                                        .into_element()
                                    })
                                    .width(Size::px(gutter_w))
                                    .height(Size::fill())
                                    .show_scrollbar(false)
                                    .scroll_controller(scroll_editor)
                                    .length(n_linhas)
                                    .item_size(line_height)
                                })
                                .child(
                                    rect()
                                        .width(Size::fill())
                                        .height(Size::fill())
                                        .child(
                                            CodeEditor::new(editor, a11y_id)
                                                .font_size(font_size)
                                                .font_family(fonte_familia())
                                                .gutter(false)
                                                .read_only(abas_agora.atual().snapshot)
                                                .scroll_controller(scroll_editor)
                                                .a11y_auto_focus(true)
                                                .on_pre_key_down(move |e: Event<KeyboardEventData>| {
                                                    if let Some(acao) = acao_de_tecla_com_foco(
                                                        &e.key,
                                                        e.modifiers,
                                                        false,
                                                    ) {
                                                        e.stop_propagation();
                                                        e.prevent_default();
                                                        on_acao(acao);
                                                        return false;
                                                    }
                                                    let seguir =
                                                        tratar_tecla(&e, &mut editor.write());
                                                    if seguir
                                                        && tecla_pede_indent(&e.key, e.modifiers)
                                                    {
                                                        indentar_pendente.set(true);
                                                    }
                                                    seguir
                                                }),
                                        ),
                                ),
                        )
                        .maybe_child(
                            (graficos_dock.visivel_cartesiano() && !graficos_dock.destacado).then(
                                move || {
                                    painel_graficos(
                                        graficos_dock.clone(),
                                        paleta,
                                        false,
                                        move || destacar_graficos(graficos),
                                    )
                                },
                            ),
                        )
                        .maybe_child(repl_agora.visivel.then(move || {
                            painel_repl(
                                repl_agora.clone(),
                                paleta,
                                repl_entrada,
                                move || enviar_repl(repl, repl_entrada, status),
                                move || parar_repl(repl, status),
                                move || limpar_repl(repl),
                                move || reiniciar_repl(repl, status),
                                move || fechar_repl(repl),
                            )
                        }))
                        .child(alca_horizontal(paleta, altura_console, arrasto_console))
                        .child(painel_console(
                            nucleo_term.read().1.clone(),
                            paleta,
                            font_size,
                            *altura_console.read(),
                            console.read().ponte.leia_aberta(),
                            move |linha| {
                                ir_para_saida(
                                    &linha,
                                    editor,
                                    prefs,
                                    abas,
                                    status,
                                    projeto,
                                    dialogo,
                                )
                            },
                        )),
                )
                .maybe_child(depuracao_agora.visivel.then(move || {
                    rect()
                        .direction(Direction::Horizontal)
                        .height(Size::fill())
                        .child(alca_vertical(paleta, largura_debug, true, arrasto_debug))
                        .child(painel_debug(
                            depuracao_agora.clone(),
                            paleta,
                            *largura_debug.read(),
                            observado_campo,
                            move || {
                                adicionar_observado(
                                    &observado_campo.read(),
                                    depuracao,
                                    observado_campo,
                                )
                            },
                            move |n| remover_observado(n, depuracao),
                            move |arq, linha| {
                                ir_para_pausa(
                                    &arq, linha, editor, prefs, abas, status, projeto, dialogo,
                                )
                            },
                        ))
                })),
        )

        .maybe_child(busca_aberta().then(move || {
            barra_busca(
                paleta,
                busca_termo,
                move || achar(true, editor, status, busca_termo),
                move || achar(false, editor, status, busca_termo),
                move || {
                    let mut busca_aberta = busca_aberta;
                    let mut substituir_aberta = substituir_aberta;
                    busca_aberta.set(false);
                    substituir_aberta.set(false);
                },
                substituir_aberta().then_some((
                    substituir_por,
                    move || substituir_uma(editor, status, busca_termo, substituir_por),
                    move || {
                        substituir_todas_no_arquivo(editor, status, busca_termo, substituir_por)
                    },
                )),
            )
        }))
        .child(barra_status(linha, coluna, &pasta, &recado, paleta))
        .maybe_child(atalhos_visiveis().then(move || {
            painel_atalhos(paleta, move || atalhos_visiveis.set(false))
        }))
        .maybe_child(ajuda.read().clone().map(|texto| {
            painel_ficha(texto, paleta, move || {
                let mut ajuda = ajuda;
                ajuda.set(None);
            })
        }))
        .maybe_child(catalogo_aberto().then(move || {
            painel_catalogo(
                catalogo(),
                paleta,
                move |nome| {
                    let mut ajuda = ajuda;
                    ajuda.set(ficha(&nome));
                },
                move || {
                    let mut catalogo_aberto = catalogo_aberto;
                    catalogo_aberto.set(false);
                },
            )
        }))
        .maybe_child({
            let texto = editor.read().rope.to_string();
            let byte = crate::editor::byte_do_cursor(&editor.read());
            let dica = if assinatura_oculta() || !sugestoes.read().is_empty() {
                None
            } else {
                assinatura(&texto, byte)
            };
            dica.map(|texto| painel_assinatura(texto, paleta))
        })
        .maybe_child((!sugestoes.read().is_empty()).then(move || {
            painel_sugestoes(sugestoes.read().clone(), paleta, move |s| {
                aplicar_sugestao(s, editor, sugestoes)
            })
        }))
        .maybe_child(
            if let Dialogo::Confirmar { id, .. } = &dialogo_agora {
                abas_agora
                    .itens
                    .iter()
                    .find(|a| a.id == *id)
                    .map(|a| a.nome.clone())
            } else {
                None
            }
            .map(|nome| {
                dialogo_confirmar(
                    paleta,
                    nome,
                    move || confirmar_descarte(true, editor, prefs, abas, status, dialogo, turma, projeto),
                    move || confirmar_descarte(false, editor, prefs, abas, status, dialogo, turma, projeto),
                    move || {
                        let mut dialogo = dialogo;
                        dialogo.set(Dialogo::Nenhum);
                    },
                )
            }),
        )
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
                    move || confirmar_nome(&nome_campo.read(), editor, prefs, abas, status, dialogo, projeto, turma),
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
                    move || confirmar_apagar(status, dialogo, projeto, turma),
                    move || {
                        let mut dialogo = dialogo;
                        dialogo.set(Dialogo::Nenhum);
                    },
                )
            }),
        )
        .maybe_child(matches!(dialogo_agora, Dialogo::Servidor).then(move || {
            dialogo_servidor(
                servidor_campo,
                aluno_campo,
                move || {
                    conectar_servidor(
                        &servidor_campo.read(),
                        &aluno_campo.read(),
                        turma,
                        prefs,
                        status,
                        dialogo,
                    )
                },
                move || {
                    let mut dialogo = dialogo;
                    dialogo.set(Dialogo::Nenhum);
                },
            )
        }))
        .maybe_child(matches!(dialogo_agora, Dialogo::Projetos).then(move || {
            dialogo_projetos(
                turma.read().projetos.clone(),
                nome_campo,
                move |nome| {
                    let _ = abrir_projeto_servidor(
                        nome, editor, prefs, abas, status, projeto, turma, dialogo,
                    );
                },
                move || {
                    criar_projeto_servidor(
                        &nome_campo.read(),
                        editor,
                        prefs,
                        abas,
                        status,
                        projeto,
                        turma,
                        dialogo,
                    )
                },
                move || {
                    let mut dialogo = dialogo;
                    dialogo.set(Dialogo::Nenhum);
                },
            )
        }))
        .maybe_child(
            if let Dialogo::Recuperar { itens } = &dialogo_agora {
                Some(itens.iter().map(|r| r.nome.clone()).collect::<Vec<_>>())
            } else {
                None
            }
            .map(|nomes| {
                dialogo_recuperar(
                    nomes,
                    move || recuperar_rascunhos(editor, prefs, abas, status, dialogo),
                    move || descartar_rascunhos(dialogo, status),
                    move || fechar_dialogo_rascunhos(dialogo),
                )
            }),
        )
        .maybe_child(
            if let Dialogo::Conflito { nome, .. } = &dialogo_agora {
                Some(nome.clone())
            } else {
                None
            }
            .map(|nome| {
                dialogo_conflito(
                    nome,
                    move || resolver_conflito("comparar", editor, prefs, abas, status, dialogo, turma, projeto),
                    move || resolver_conflito("recarregar", editor, prefs, abas, status, dialogo, turma, projeto),
                    move || resolver_conflito("salvar_como", editor, prefs, abas, status, dialogo, turma, projeto),
                    move || resolver_conflito("substituir", editor, prefs, abas, status, dialogo, turma, projeto),
                    move || resolver_conflito("cancelar", editor, prefs, abas, status, dialogo, turma, projeto),
                )
            }),
        )
        .maybe_child(
            if let Dialogo::Comparar { nome, aba, disco } = &dialogo_agora {
                Some((nome.clone(), aba.clone(), disco.clone()))
            } else {
                None
            }
            .map(|(nome, aba, disco)| {
                dialogo_comparar(nome, aba, disco, move || {
                    let mut dialogo = dialogo;
                    dialogo.set(Dialogo::Nenhum);
                })
            }),
        )
}
