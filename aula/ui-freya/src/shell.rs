//! Menubar, toolbar and status bar.

use freya::{
    code_editor::{CodeEditorData, CodeEditorThemeExt, EditorSyntaxTheme},
    prelude::*,
};

use crate::{
    ajuda::Sugestao,
    atalhos::{Acao, ATALHOS},
    console::Console,
    debug::{Depuracao, valor_observado},
    documentos::Abas,
    editor::new_editor,
    grafico::{Graficos, png_folha, png_tela},
    projeto::Projeto,
    repl::{Repl, TipoLinha},
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAberto {
    #[default]
    Nenhum,
    Arquivo,
    Executar,
    Depurar,
    Ver,
    Ajuda,
}

#[derive(Clone, Copy)]
pub struct Paleta {
    pub fundo: (u8, u8, u8),
    pub texto: (u8, u8, u8),
    pub barra: (u8, u8, u8),
    pub borda: (u8, u8, u8),
    pub botao: (u8, u8, u8),
    pub status: (u8, u8, u8),
}

impl Paleta {
    pub fn de(dark: bool) -> Self {
        if dark {
            Self {
                fundo: (30, 30, 36),
                texto: (232, 230, 227),
                barra: (36, 36, 42),
                borda: (58, 58, 68),
                botao: (48, 48, 56),
                status: (28, 28, 34),
            }
        } else {
            Self {
                fundo: (246, 245, 244),
                texto: (61, 56, 70),
                barra: (246, 245, 244),
                borda: (211, 207, 200),
                botao: (233, 231, 228),
                status: (236, 234, 231),
            }
        }
    }
}

pub fn tema_freya(dark: bool) -> Theme {
    if dark {
        dark_theme().with_dark_code_editor()
    } else {
        light_theme().with_light_code_editor()
    }
}

pub fn aplicar_tema_editor(editor: &mut CodeEditorData, dark: bool) {
    editor.set_theme(if dark {
        EditorSyntaxTheme::dark()
    } else {
        EditorSyntaxTheme::light()
    });
    editor.parse();
}

pub fn barra_menus(
    aberto: State<MenuAberto>,
    paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
) -> impl IntoElement {
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(32.0))
        .background(paleta.barra)
        .color(paleta.texto)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 0.0,
                    right: 0.0,
                    bottom: 1.0,
                    left: 0.0,
                })
                .fill(paleta.borda),
        )
        .padding((0.0, 6.0, 0.0, 6.0))
        .cross_align(Alignment::Center)
        .child(botao_menu(
            "Arquivo",
            MenuAberto::Arquivo,
            aberto,
            paleta,
            on_acao,
            &[
                ("Novo", "Ctrl+N", Acao::Novo),
                ("Abrir…", "", Acao::Abrir),
                ("Abrir projeto…", "", Acao::AbrirProjeto),
                ("Salvar", "Ctrl+S", Acao::Salvar),
                ("Salvar como…", "Ctrl+Shift+S", Acao::SalvarComo),
                ("Sair", "Ctrl+Q", Acao::Sair),
            ],
        ))
        .child(botao_menu(
            "Executar",
            MenuAberto::Executar,
            aberto,
            paleta,
            on_acao,
            &[
                ("Executar", "F5", Acao::Executar),
                ("Parar", "", Acao::Parar),
            ],
        ))
        .child(botao_menu(
            "Depurar",
            MenuAberto::Depurar,
            aberto,
            paleta,
            on_acao,
            &[
                ("Depurar", "F6", Acao::Depurar),
                ("Ponto de parada", "F9", Acao::Ponto),
                ("Próximo", "F10", Acao::Proximo),
                ("Entrar", "F11", Acao::Entrar),
                ("Sair da função", "Shift+F11", Acao::SairFuncao),
            ],
        ))
        .child(botao_menu(
            "Ver",
            MenuAberto::Ver,
            aberto,
            paleta,
            on_acao,
            &[
                ("Arquivos", "F7", Acao::MostrarArquivos),
                ("Painel", "F8", Acao::MostrarPainel),
                ("Claro", "", Acao::TemaClaro),
                ("Escuro", "", Acao::TemaEscuro),
                ("Letra maior", "Ctrl+=", Acao::ZoomMais),
                ("Letra menor", "Ctrl+-", Acao::ZoomMenos),
                ("Letra normal", "Ctrl+0", Acao::ZoomNormal),
                ("Ir para linha…", "Ctrl+G", Acao::IrParaLinha),
                ("Buscar…", "Ctrl+F", Acao::Buscar),
                ("REPL", "", Acao::Repl),
            ],
        ))
        .child(botao_menu(
            "Ajuda",
            MenuAberto::Ajuda,
            aberto,
            paleta,
            on_acao,
            &[
                ("Funções nativas", "F1", Acao::Catalogo),
                ("Atalhos", "Ctrl+?", Acao::Atalhos),
                ("Tutoriais", "", Acao::Tutoriais),
            ],
        ))
}

fn botao_menu(
    titulo: &'static str,
    id: MenuAberto,
    mut aberto: State<MenuAberto>,
    _paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
    itens: &'static [(&'static str, &'static str, Acao)],
) -> impl IntoElement {
    let esta = aberto() == id;
    rect()
        .content(Content::fit())
        .child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| {
                    if aberto() == id {
                        aberto.set(MenuAberto::Nenhum);
                    } else {
                        aberto.set(id);
                    }
                })
                .child(titulo),
        )
        .maybe_child(esta.then(move || {
            let mut menu = Menu::new().on_close(move |_| aberto.set(MenuAberto::Nenhum));
            for (nome, atalho, acao) in itens {
                let acao = *acao;
                let texto = if atalho.is_empty() {
                    (*nome).to_string()
                } else {
                    format!("{nome}    {atalho}")
                };
                menu = menu.child(
                    MenuButton::new()
                        .on_press(move |_| {
                            aberto.set(MenuAberto::Nenhum);
                            on_acao(acao);
                        })
                        .child(texto),
                );
            }
            menu
        }))
}

pub fn barra_ferramentas(
    paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
) -> impl IntoElement {
    fn botao(
        texto: &'static str,
        acao: Acao,
        on_acao: impl Fn(Acao) + Copy + 'static,
    ) -> Button {
        Button::new()
            .compact()
            .on_press(move |_| on_acao(acao))
            .child(texto)
    }
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(36.0))
        .background(paleta.botao)
        .color(paleta.texto)
        .padding((4.0, 8.0, 4.0, 8.0))
        .spacing(6.0)
        .cross_align(Alignment::Center)
        .child(botao("Novo", Acao::Novo, on_acao))
        .child(botao("Abrir", Acao::Abrir, on_acao))
        .child(botao("Salvar", Acao::Salvar, on_acao))
        .child(botao("Rodar", Acao::Executar, on_acao))
        .child(botao("Parar", Acao::Parar, on_acao))
        .child(botao("Depurar", Acao::Depurar, on_acao))
        .child(botao("A−", Acao::ZoomMenos, on_acao))
        .child(botao("A+", Acao::ZoomMais, on_acao))
}

pub fn barra_argumentos(
    paleta: Paleta,
    args: State<String>,
) -> impl IntoElement {
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(32.0))
        .background(paleta.botao)
        .color(paleta.texto)
        .padding((2.0, 8.0, 2.0, 8.0))
        .spacing(8.0)
        .cross_align(Alignment::Center)
        .child(label().font_size(12.0).text("argumentos:"))
        .child(
            Input::new(args)
                .placeholder("ex.: Ana --ajuda")
                .width(Size::fill()),
        )
}

pub fn painel_console(
    console: Console,
    paleta: Paleta,
    entrada: State<String>,
    on_leia: impl Fn() + Copy + 'static,
    on_linha: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .padding(6.0);
    for (i, linha) in console.linhas.iter().enumerate() {
        let texto = console.texto_linha(i);
        let mut row = rect()
            .direction(Direction::Horizontal)
            .width(Size::fill());
        if linha.is_empty() {
            row = row.child(label().font_size(13.0).font_family("DejaVu Sans Mono").text(" "));
        } else {
            for t in linha {
                let cor = if t.stderr {
                    (229, 57, 53)
                } else {
                    t.fg.unwrap_or(paleta.texto)
                };
                row = row.child(
                    label()
                        .font_size(13.0)
                        .font_family("DejaVu Sans Mono")
                        .color(cor)
                        .font_weight(if t.negrito {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text(t.texto.clone()),
                );
            }
        }
        row = row.on_press(move |_| on_linha(texto.clone()));
        lista = lista.child(row);
    }
    let mut painel = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::px(160.0))
        .background(paleta.status)
        .color(paleta.texto)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .fill(paleta.borda),
        )
        .child(ScrollView::new().height(Size::fill()).child(lista));
    if console.leia_prompt.is_some() {
        let prompt = console.leia_prompt.clone().unwrap_or_default();
        painel = painel.child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 6.0, 4.0, 6.0))
                .spacing(6.0)
                .cross_align(Alignment::Center)
                .child(label().font_size(13.0).text(prompt))
                .child(
                    Input::new(entrada)
                        .placeholder("leia")
                        .width(Size::fill())
                        .on_submit(move |_| on_leia()),
                ),
        );
    }
    painel
}

pub fn barra_status(linha: usize, coluna: usize, projeto: &str, recado: &str, paleta: Paleta) -> impl IntoElement {
    let pos = format!("{}:{}", linha + 1, coluna + 1);
    let caminho = if projeto.is_empty() {
        String::new()
    } else {
        crate::prefs::path_for_display(std::path::Path::new(projeto))
    };
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(24.0))
        .background(paleta.status)
        .color(paleta.texto)
        .padding((0.0, 8.0, 0.0, 8.0))
        .main_align(Alignment::SpaceBetween)
        .cross_align(Alignment::Center)
        .child(label().font_size(12.0).text(format!("{pos}   {recado}")))
        .child(label().font_size(12.0).text(caminho))
}

pub fn painel_atalhos(paleta: Paleta, mut fechar: impl FnMut() + Clone + 'static) -> impl IntoElement {
    let mut corpo = rect()
        .direction(Direction::Vertical)
        .spacing(8.0)
        .padding(16.0);
    for (secao, itens) in ATALHOS {
        corpo = corpo.child(
            label()
                .font_size(14.0)
                .font_weight(FontWeight::BOLD)
                .text((*secao).to_string()),
        );
        for (teclas, o_que) in *itens {
            corpo = corpo.child(
                rect()
                    .direction(Direction::Horizontal)
                    .spacing(16.0)
                    .child(label().font_size(13.0).text((*teclas).to_string()))
                    .child(label().font_size(13.0).text((*o_que).to_string())),
            );
        }
    }
    rect()
        .position(Position::new_global().top(0.).left(0.))
        .width(Size::fill())
        .height(Size::fill())
        .background((0, 0, 0, 120))
        .center()
        .on_press(move |_| fechar())
        .child(
            rect()
                .width(Size::px(520.0))
                .height(Size::px(560.0))
                .background(paleta.fundo)
                .color(paleta.texto)
                .corner_radius(8.0)
                .on_press(|e: Event<PressEventData>| e.stop_propagation())
                .child(ScrollView::new().child(corpo)),
        )
}

/// Rebuild the editor buffer after a theme change at first load.
pub fn editor_inicial(texto: &str, dark: bool) -> CodeEditorData {
    let mut editor = new_editor(texto);
    aplicar_tema_editor(&mut editor, dark);
    editor
}

pub fn barra_abas(
    abas: &Abas,
    paleta: Paleta,
    on_selecionar: impl Fn(usize) + Copy + 'static,
    on_fechar: impl Fn(u64) + Copy + 'static,
) -> impl IntoElement {
    let mut barra = rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(32.0))
        .background(paleta.barra)
        .color(paleta.texto)
        .spacing(4.0)
        .padding((2.0, 6.0, 2.0, 6.0))
        .cross_align(Alignment::Center);
    for (i, aba) in abas.itens.iter().enumerate() {
        let id = aba.id;
        let atual = i == abas.atual;
        let fundo = if atual { paleta.botao } else { paleta.barra };
        barra = barra.child(
            rect()
                .key(DiffKey::U64(id))
                .direction(Direction::Horizontal)
                .background(fundo)
                .corner_radius(4.0)
                .padding((4.0, 8.0, 4.0, 8.0))
                .spacing(6.0)
                .cross_align(Alignment::Center)
                .on_press(move |_| on_selecionar(i))
                .child(label().font_size(13.0).text(Abas::rotulo(aba)))
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| on_fechar(id))
                        .child("×"),
                ),
        );
    }
    barra
}

pub fn dialogo_confirmar(
    _paleta: Paleta,
    on_salvar: impl Fn() + Copy + 'static,
    on_descartar: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(420.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new("Alterações não salvas".to_string()))
        .child(PopupContent::new().child("Salvar as alterações desta aba?"))
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_salvar())
                        .filled()
                        .child("Salvar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_descartar())
                        .child("Descartar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn barra_busca(
    paleta: Paleta,
    termo: State<String>,
    on_proxima: impl Fn() + Copy + 'static,
    on_anterior: impl Fn() + Copy + 'static,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(36.0))
        .background(paleta.botao)
        .color(paleta.texto)
        .padding((4.0, 8.0, 4.0, 8.0))
        .spacing(8.0)
        .cross_align(Alignment::Center)
        .child(label().font_size(13.0).text("Buscar"))
        .child(
            Input::new(termo)
                .placeholder("texto")
                .width(Size::px(240.0))
                .on_submit(move |_| on_proxima()),
        )
        .child(
            Button::new()
                .compact()
                .on_press(move |_| on_proxima())
                .child("↓"),
        )
        .child(
            Button::new()
                .compact()
                .on_press(move |_| on_anterior())
                .child("↑"),
        )
        .child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| on_fechar())
                .child("×"),
        )
}

pub fn painel_arvore(
    projeto: Projeto,
    paleta: Paleta,
    on_abrir: impl Fn(String) + Copy + 'static,
    on_novo: impl Fn() + Copy + 'static,
    on_pasta: impl Fn() + Copy + 'static,
    on_renomear: impl Fn() + Copy + 'static,
    on_apagar: impl Fn() + Copy + 'static,
    on_projeto: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(1.0);
    for no in projeto.visiveis() {
        let rel = no.rel.clone();
        let atual = projeto.selecionado.as_deref() == Some(rel.as_str());
        let fundo = if atual { paleta.botao } else { paleta.barra };
        let marca = if no.is_dir {
            if projeto.abertos.contains(&rel) {
                "▾ "
            } else {
                "▸ "
            }
        } else {
            "  "
        };
        let pad = 8.0 + (no.profundidade as f32) * 12.0;
        lista = lista.child(
            rect()
                .key(DiffKey::from(&rel))
                .width(Size::fill())
                .background(fundo)
                .padding((2.0, 6.0, 2.0, pad))
                .on_press(move |_| on_abrir(rel.clone()))
                .child(label().font_size(13.0).text(format!("{marca}{}", no.nome))),
        );
    }
    rect()
        .direction(Direction::Vertical)
        .width(Size::px(240.0))
        .height(Size::fill())
        .background(paleta.barra)
        .color(paleta.texto)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 0.0,
                    right: 1.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .fill(paleta.borda),
        )
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 6.0, 4.0, 6.0))
                .main_align(Alignment::SpaceBetween)
                .cross_align(Alignment::Center)
                .child(label().font_size(12.0).font_weight(FontWeight::BOLD).text("Arquivos"))
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| on_projeto())
                        .child("…"),
                ),
        )
        .child(ScrollView::new().height(Size::fill()).child(lista))
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 4.0, 4.0, 4.0))
                .spacing(4.0)
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_novo())
                        .child("Novo"),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_pasta())
                        .child("Pasta"),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_renomear())
                        .child("Nome"),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_apagar())
                        .child("Apagar"),
                ),
        )
}

pub fn dialogo_linha(
    _paleta: Paleta,
    valor: State<String>,
    on_ir: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(320.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new("Ir para linha".to_string()))
        .child(
            PopupContent::new().child(
                Input::new(valor)
                    .placeholder("número da linha")
                    .width(Size::fill())
                    .on_submit(move |_| on_ir()),
            ),
        )
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_ir())
                        .filled()
                        .child("Ir"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn dialogo_nome(
    titulo: String,
    valor: State<String>,
    on_ok: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(360.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new(titulo))
        .child(
            PopupContent::new().child(
                Input::new(valor)
                    .placeholder("nome")
                    .width(Size::fill())
                    .on_submit(move |_| on_ok()),
            ),
        )
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_ok())
                        .filled()
                        .child("OK"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn painel_ficha(
    texto: String,
    paleta: Paleta,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    rect()
        .position(Position::new_global().top(80.).right(16.))
        .width(Size::px(420.0))
        .height(Size::px(360.0))
        .background(paleta.fundo)
        .color(paleta.texto)
        .corner_radius(8.0)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 1.0,
                    right: 1.0,
                    bottom: 1.0,
                    left: 1.0,
                })
                .fill(paleta.borda),
        )
        .child(
            rect()
                .direction(Direction::Vertical)
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    rect()
                        .direction(Direction::Horizontal)
                        .width(Size::fill())
                        .main_align(Alignment::SpaceBetween)
                        .padding(8.0)
                        .child(label().font_weight(FontWeight::BOLD).text("Ajuda"))
                        .child(
                            Button::new()
                                .flat()
                                .compact()
                                .on_press(move |_| on_fechar())
                                .child("×"),
                        ),
                )
                .child(ScrollView::new().child(label().font_size(13.0).text(texto))),
        )
}

pub fn painel_catalogo(
    itens: Vec<(String, String)>,
    paleta: Paleta,
    on_item: impl Fn(String) + Copy + 'static,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect().direction(Direction::Vertical).spacing(2.0).padding(8.0);
    for (nome, _) in itens {
        let n = nome.clone();
        lista = lista.child(
            Button::new()
                .flat()
                .on_press(move |_| on_item(n.clone()))
                .child(nome),
        );
    }
    rect()
        .position(Position::new_global().top(80.).left(260.))
        .width(Size::px(360.0))
        .height(Size::px(480.0))
        .background(paleta.fundo)
        .color(paleta.texto)
        .corner_radius(8.0)
        .child(
            rect()
                .direction(Direction::Vertical)
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    rect()
                        .direction(Direction::Horizontal)
                        .width(Size::fill())
                        .main_align(Alignment::SpaceBetween)
                        .padding(8.0)
                        .child(label().font_weight(FontWeight::BOLD).text("Funções nativas"))
                        .child(
                            Button::new()
                                .flat()
                                .compact()
                                .on_press(move |_| on_fechar())
                                .child("×"),
                        ),
                )
                .child(ScrollView::new().child(lista)),
        )
}

pub fn painel_sugestoes(
    itens: Vec<Sugestao>,
    paleta: Paleta,
    on_item: impl Fn(Sugestao) + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect().direction(Direction::Vertical).spacing(2.0).padding(6.0);
    for s in itens {
        let escolhida = s.clone();
        lista = lista.child(
            Button::new()
                .flat()
                .on_press(move |_| on_item(escolhida.clone()))
                .child(s.rotulo),
        );
    }
    rect()
        .position(Position::new_global().top(120.).left(280.))
        .width(Size::px(320.0))
        .content(Content::fit())
        .background(paleta.fundo)
        .color(paleta.texto)
        .corner_radius(6.0)
        .child(lista)
}

pub fn dialogo_apagar(
    rel: String,
    on_ok: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(400.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new("Apagar".to_string()))
        .child(PopupContent::new().child(format!("Apagar {rel}?")))
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_ok())
                        .filled()
                        .child("Apagar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn painel_debug(
    dep: Depuracao,
    paleta: Paleta,
    campo: State<String>,
    on_observar: impl Fn() + Copy + 'static,
    on_tirar: impl Fn(String) + Copy + 'static,
    on_quadro: impl Fn(String, u32) + Copy + 'static,
) -> impl IntoElement {
    let mut observados = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(2.0);
    for nome in &dep.observados {
        let n = nome.clone();
        let valor = valor_observado(nome, &dep.vars);
        observados = observados.child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .main_align(Alignment::SpaceBetween)
                .child(label().font_size(12.0).text(format!("{nome} = {valor}")))
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| on_tirar(n.clone()))
                        .child("×"),
                ),
        );
    }
    let mut vars = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(1.0);
    for v in &dep.vars {
        vars = vars.child(
            label()
                .font_size(12.0)
                .font_family("DejaVu Sans Mono")
                .text(format!("[{}] {} = {}", v.escopo, v.nome, v.valor)),
        );
    }
    let mut pilha = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(1.0);
    for q in &dep.pilha {
        let arq = q.arquivo.clone();
        let linha = q.linha;
        pilha = pilha.child(
            rect()
                .width(Size::fill())
                .on_press(move |_| on_quadro(arq.clone(), linha))
                .child(
                    label()
                        .font_size(12.0)
                        .font_family("DejaVu Sans Mono")
                        .text(format!("{}  {}:{}", q.nome, q.arquivo, q.linha)),
                ),
        );
    }
    let mut pontos = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(1.0);
    for (arq, linha) in &dep.pontos {
        pontos = pontos.child(
            label()
                .font_size(12.0)
                .text(format!("{arq}:{linha}")),
        );
    }
    let titulo = if dep.pausado && dep.linha > 0 {
        format!("pausado {}:{}", dep.arquivo, dep.linha)
    } else if dep.pausado {
        "pausado".into()
    } else {
        "Depurador".into()
    };
    rect()
        .direction(Direction::Vertical)
        .width(Size::px(280.0))
        .height(Size::fill())
        .background(paleta.barra)
        .color(paleta.texto)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 1.0,
                })
                .fill(paleta.borda),
        )
        .padding(6.0)
        .spacing(6.0)
        .child(
            label()
                .font_size(12.0)
                .font_weight(FontWeight::BOLD)
                .text(titulo),
        )
        .maybe_child((!dep.trecho.is_empty()).then(|| {
            label()
                .font_size(12.0)
                .font_family("DejaVu Sans Mono")
                .text(dep.trecho.clone())
        }))
        .child(
            label()
                .font_size(11.0)
                .font_weight(FontWeight::BOLD)
                .text("Pontos"),
        )
        .child(pontos)
        .child(
            label()
                .font_size(11.0)
                .font_weight(FontWeight::BOLD)
                .text("Observados"),
        )
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .spacing(4.0)
                .child(
                    Input::new(campo)
                        .placeholder("nome")
                        .width(Size::fill())
                        .on_submit(move |_| on_observar()),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_observar())
                        .child("+"),
                ),
        )
        .child(observados)
        .child(
            label()
                .font_size(11.0)
                .font_weight(FontWeight::BOLD)
                .text("Variáveis"),
        )
        .child(ScrollView::new().height(Size::fill()).child(vars))
        .child(
            label()
                .font_size(11.0)
                .font_weight(FontWeight::BOLD)
                .text("Pilha"),
        )
        .child(ScrollView::new().height(Size::px(120.0)).child(pilha))
}

pub fn painel_repl(
    repl: Repl,
    paleta: Paleta,
    entrada: State<String>,
    on_enviar: impl Fn() + Copy + 'static,
    on_parar: impl Fn() + Copy + 'static,
    on_limpar: impl Fn() + Copy + 'static,
    on_reiniciar: impl Fn() + Copy + 'static,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .padding(6.0);
    for l in &repl.linhas {
        let cor = match l.tipo {
            TipoLinha::Eco => (140, 140, 148),
            TipoLinha::Erro => (229, 57, 53),
            TipoLinha::Saida => paleta.texto,
        };
        lista = lista.child(
            label()
                .font_size(13.0)
                .font_family("DejaVu Sans Mono")
                .color(cor)
                .text(l.texto.clone()),
        );
    }
    rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::px(220.0))
        .background(paleta.status)
        .color(paleta.texto)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .fill(paleta.borda),
        )
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 6.0, 4.0, 6.0))
                .main_align(Alignment::SpaceBetween)
                .cross_align(Alignment::Center)
                .child(
                    label()
                        .font_size(12.0)
                        .font_weight(FontWeight::BOLD)
                        .text("REPL"),
                )
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| on_fechar())
                        .child("×"),
                ),
        )
        .child(ScrollView::new().height(Size::fill()).child(lista))
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 6.0, 4.0, 6.0))
                .spacing(6.0)
                .cross_align(Alignment::Center)
                .child(label().font_size(13.0).font_family("DejaVu Sans Mono").text(repl.prompt.clone()))
                .child(
                    Input::new(entrada)
                        .placeholder("2 + 3")
                        .width(Size::fill())
                        .on_submit(move |_| on_enviar()),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_parar())
                        .child("Parar"),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_limpar())
                        .child("Limpar"),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_reiniciar())
                        .child("Reiniciar"),
                ),
        )
}

pub fn painel_graficos(
    g: Graficos,
    paleta: Paleta,
    destacado: bool,
    on_destacar: impl Fn() + Copy + 'static,
    on_fechar_tela: impl Fn(u64) + Copy + 'static,
    on_clique: impl Fn(u64, f64, f64, f64, f64) + Copy + 'static,
) -> impl IntoElement {
    let mut corpo = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(6.0)
        .padding(6.0);
    for folha in &g.folhas {
        let (w, h, png) = png_folha(folha);
        corpo = corpo.child(
            ImageViewer::new((format!("folha-{}", folha.id), Bytes::from(png)))
                .width(Size::px(w as f32))
                .height(Size::px(h as f32)),
        );
    }
    for tela in &g.telas {
        let id = tela.id;
        let (w, h, png) = png_tela(tela);
        let wf = w as f32;
        let hf = h as f32;
        corpo = corpo.child(
            rect()
                .direction(Direction::Vertical)
                .child(
                    rect()
                        .direction(Direction::Horizontal)
                        .width(Size::fill())
                        .main_align(Alignment::SpaceBetween)
                        .child(label().font_size(12.0).text(format!("tela {id}")))
                        .child(
                            Button::new()
                                .flat()
                                .compact()
                                .on_press(move |_| on_fechar_tela(id))
                                .child("×"),
                        ),
                )
                .child(
                    rect()
                        .on_press(move |e: Event<PressEventData>| {
                            if let PressEventData::Mouse(m) = &*e {
                                let p = m.element_location;
                                on_clique(id, p.x, p.y, wf as f64, hf as f64);
                            }
                        })
                        .child(
                            ImageViewer::new((format!("tela-{id}"), Bytes::from(png)))
                                .width(Size::px(wf))
                                .height(Size::px(hf)),
                        ),
                ),
        );
    }
    let mut caixa = rect()
        .direction(Direction::Vertical)
        .width(if destacado { Size::px(440.0) } else { Size::fill() })
        .height(if destacado { Size::px(360.0) } else { Size::px(240.0) })
        .background(paleta.status)
        .color(paleta.texto)
        .border(
            Border::new()
                .width(BorderWidth {
                    top: 1.0,
                    right: 1.0,
                    bottom: 1.0,
                    left: 1.0,
                })
                .fill(paleta.borda),
        )
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 6.0, 4.0, 6.0))
                .main_align(Alignment::SpaceBetween)
                .child(
                    label()
                        .font_size(12.0)
                        .font_weight(FontWeight::BOLD)
                        .text("Gráficos"),
                )
                .child(
                    Button::new()
                        .flat()
                        .compact()
                        .on_press(move |_| on_destacar())
                        .child(if destacado { "Acoplar" } else { "Destacar" }),
                ),
        )
        .child(ScrollView::new().height(Size::fill()).child(corpo));
    if destacado {
        caixa = caixa.position(Position::new_global().top(80.).right(20.));
    }
    caixa
}


