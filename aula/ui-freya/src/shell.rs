//! Menubar, toolbar and status bar.

use freya::{
    code_editor::{CodeEditorData, CodeEditorThemeExt, EditorSyntaxTheme},
    prelude::*,
};

use crate::{
    atalhos::{Acao, ATALHOS},
    documentos::Abas,
    editor::new_editor,
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


