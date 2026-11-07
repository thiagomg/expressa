//! Menubar, toolbar and status bar.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use freya::{
    code_editor::{CodeEditorData, CodeEditorThemeExt, EditorSyntaxTheme},
    engine::prelude::AlphaType,
    prelude::*,
};

use crate::{
    ajuda::Sugestao,
    atalhos::{Acao, ATALHOS},
    prefs::Recente,
    console::Console,
    debug::{Depuracao, MarcaGutter, marca_gutter, valor_observado},
    documentos::Abas,
    editor::new_editor,
    grafico::{Graficos, rgba_folha, rgba_tela, tecla_de_freya},
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

/// Accessibility ids of text fields, so IDE shortcuts skip Ctrl+S/W/F while typing.
#[derive(Clone, Default)]
pub struct FocoCampos {
    ids: Rc<RefCell<HashSet<AccessibilityId>>>,
}

impl FocoCampos {
    pub fn tem_foco(&self) -> bool {
        self.ids.borrow().iter().any(|id| id.is_focused())
    }
}

#[derive(Clone, PartialEq)]
struct CampoTexto {
    valor: State<String>,
    placeholder: &'static str,
    width: Size,
    auto_focus: bool,
    on_submit: Option<EventHandler<String>>,
}

impl CampoTexto {
    fn new(valor: State<String>) -> Self {
        Self {
            valor,
            placeholder: "",
            width: Size::px(150.0),
            auto_focus: false,
            on_submit: None,
        }
    }

    fn placeholder(mut self, placeholder: &'static str) -> Self {
        self.placeholder = placeholder;
        self
    }

    fn width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    fn on_submit(mut self, on_submit: impl Into<EventHandler<String>>) -> Self {
        self.on_submit = Some(on_submit.into());
        self
    }
}

impl Component for CampoTexto {
    fn render(&self) -> impl IntoElement {
        let a11y = use_a11y();
        let foco = try_consume_root_context::<FocoCampos>();
        use_hook(move || {
            if let Some(foco) = &foco {
                foco.ids.borrow_mut().insert(a11y);
            }
        });
        let mut campo = Input::new(self.valor)
            .placeholder(self.placeholder)
            .width(self.width.clone())
            .a11y_id(a11y)
            .auto_focus(self.auto_focus);
        if let Some(on_submit) = &self.on_submit {
            campo = campo.on_submit(on_submit.clone());
        }
        campo
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarcasVer {
    pub arquivos: bool,
    pub painel: bool,
    pub escuro: bool,
    pub repl: bool,
}

pub fn rotulo_menu(nome: &str, atalho: &str, marcado: bool) -> String {
    let marca = if marcado { "✓ " } else { "" };
    if atalho.is_empty() {
        format!("{marca}{nome}")
    } else {
        format!("{marca}{nome}    {atalho}")
    }
}

pub fn barra_menus(
    aberto: State<MenuAberto>,
    paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
    on_recente: impl Fn(Recente) + Copy + 'static,
    recentes: Vec<Recente>,
    marcas: MarcasVer,
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
        .child(menu_arquivo(
            aberto,
            paleta,
            on_acao,
            on_recente,
            recentes,
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
            |_| false,
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
            |_| false,
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
            move |acao| match acao {
                Acao::MostrarArquivos => marcas.arquivos,
                Acao::MostrarPainel => marcas.painel,
                Acao::TemaEscuro => marcas.escuro,
                Acao::TemaClaro => !marcas.escuro,
                Acao::Repl => marcas.repl,
                _ => false,
            },
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
            |_| false,
        ))
}

fn menu_arquivo(
    mut aberto: State<MenuAberto>,
    _paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
    on_recente: impl Fn(Recente) + Copy + 'static,
    recentes: Vec<Recente>,
) -> impl IntoElement {
    let id = MenuAberto::Arquivo;
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
                .child("Arquivo"),
        )
        .maybe_child(esta.then(move || {
            let itens = [
                ("Novo", "Ctrl+N", Acao::Novo),
                ("Abrir…", "", Acao::Abrir),
                ("Abrir projeto…", "", Acao::AbrirProjeto),
            ];
            let depois = [
                ("Servidor…", "", Acao::Servidor),
                ("Salvar", "Ctrl+S", Acao::Salvar),
                ("Salvar como…", "Ctrl+Shift+S", Acao::SalvarComo),
                ("Sair", "Ctrl+Q", Acao::Sair),
            ];
            let mut menu = Menu::new().on_close(move |_| aberto.set(MenuAberto::Nenhum));
            for (nome, atalho, acao) in itens {
                menu = menu.child(
                    MenuButton::new()
                        .on_press(move |_| {
                            aberto.set(MenuAberto::Nenhum);
                            on_acao(acao);
                        })
                        .child(rotulo_menu(nome, atalho, false)),
                );
            }
            let mut sub = SubMenu::new().label("Recentes");
            if recentes.is_empty() {
                sub = sub.child(MenuButton::new().child("Nenhum recente"));
            } else {
                for r in recentes {
                    let escolhido = r.clone();
                    sub = sub.child(
                        MenuButton::new()
                            .on_press(move |_| {
                                aberto.set(MenuAberto::Nenhum);
                                on_recente(escolhido.clone());
                            })
                            .child(r.rotulo()),
                    );
                }
            }
            menu = menu.child(sub);
            for (nome, atalho, acao) in depois {
                menu = menu.child(
                    MenuButton::new()
                        .on_press(move |_| {
                            aberto.set(MenuAberto::Nenhum);
                            on_acao(acao);
                        })
                        .child(rotulo_menu(nome, atalho, false)),
                );
            }
            menu
        }))
}

fn botao_menu(
    titulo: &'static str,
    id: MenuAberto,
    mut aberto: State<MenuAberto>,
    _paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
    itens: &'static [(&'static str, &'static str, Acao)],
    marcado: impl Fn(Acao) -> bool + Copy + 'static,
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
                let texto = rotulo_menu(nome, atalho, marcado(acao));
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
            CampoTexto::new(args)
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
                let lab = label()
                    .font_size(13.0)
                    .font_family("DejaVu Sans Mono")
                    .color(cor)
                    .font_weight(if t.negrito {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text(t.texto.clone());
                row = row.child(match t.bg {
                    Some(bg) => rect().background(bg).child(lab),
                    None => rect().child(lab),
                });
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
                    CampoTexto::new(entrada)
                        .placeholder("leia")
                        .width(Size::fill())
                        .auto_focus(true)
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
    nome: String,
    on_salvar: impl Fn() + Copy + 'static,
    on_descartar: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(420.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new("Alterações não salvas".to_string()))
        .child(PopupContent::new().child(format!("Salvar as alterações de {nome}?")))
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
            CampoTexto::new(termo)
                .placeholder("texto")
                .width(Size::px(240.0))
                .auto_focus(true)
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

pub const LARGURA_ARVORE: f32 = 240.0;
pub const LARGURA_DEBUG: f32 = 280.0;
pub const LARGURA_PAINEL_MIN: f32 = 140.0;
pub const LARGURA_PAINEL_MAX: f32 = 480.0;

pub fn alca_vertical(
    paleta: Paleta,
    largura: State<f32>,
    inverter: bool,
    mut arrasto: State<Option<f32>>,
) -> impl IntoElement {
    rect()
        .width(Size::px(6.0))
        .height(Size::fill())
        .background(paleta.borda)
        .on_pointer_down(move |e: Event<PointerEventData>| {
            let x = e.data().global_location().x as f32;
            let atual = *largura.read();
            arrasto.set(Some(if inverter { x + atual } else { x - atual }));
        })
}

pub fn linha_gutter(
    indice: usize,
    arquivo: &str,
    pontos: &[(String, u32)],
    pausado: bool,
    pausa_arquivo: &str,
    pausa_linha: u32,
    font_size: f32,
    line_height: f32,
    paleta: Paleta,
    on_linha: impl Fn(u32) + Copy + 'static,
) -> impl IntoElement {
    let linha = (indice + 1) as u32;
    let marca = marca_gutter(pontos, arquivo, linha, pausado, pausa_arquivo, pausa_linha);
    let fundo = match marca {
        MarcaGutter::Atual | MarcaGutter::PontoAtual => (80, 70, 20, 255),
        _ => (0, 0, 0, 0),
    };
    let simbolo = match marca {
        MarcaGutter::Ponto | MarcaGutter::PontoAtual => "●",
        _ => " ",
    };
    let cor_ponto = (198, 40, 40);
    rect()
        .height(Size::px(line_height))
        .width(Size::fill())
        .background(fundo)
        .direction(Direction::Horizontal)
        .cross_align(Alignment::Center)
        .padding((0., 4.0, 0., 4.0))
        .on_press(move |_| on_linha(linha))
        .child(
            label()
                .font_size(font_size * 0.7)
                .color(cor_ponto)
                .text(simbolo),
        )
        .child(
            label()
                .font_size(font_size * 0.75)
                .color(paleta.texto)
                .text(format!("{linha}")),
        )
}

pub fn aplicar_arrasto(arrasto: Option<f32>, x: f32, inverter: bool) -> Option<f32> {
    arrasto.map(|origem| {
        let bruto = if inverter { origem - x } else { x - origem };
        bruto.clamp(LARGURA_PAINEL_MIN, LARGURA_PAINEL_MAX)
    })
}

pub fn painel_arvore(
    projeto: Projeto,
    paleta: Paleta,
    largura: f32,
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
        .width(Size::px(largura))
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
                .child(
                    label()
                        .font_size(12.0)
                        .font_weight(FontWeight::BOLD)
                        .text(if projeto.remoto {
                            "Arquivos (servidor)"
                        } else {
                            "Arquivos"
                        }),
                )
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
                CampoTexto::new(valor)
                    .placeholder("número da linha")
                    .width(Size::fill())
                    .auto_focus(true)
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
                CampoTexto::new(valor)
                    .placeholder("nome")
                    .width(Size::fill())
                    .auto_focus(true)
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

pub fn painel_assinatura(texto: String, paleta: Paleta) -> impl IntoElement {
    rect()
        .position(Position::new_global().top(72.).left(280.))
        .content(Content::fit())
        .background(paleta.fundo)
        .color(paleta.texto)
        .corner_radius(6.0)
        .padding(8.0)
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
        .child(label().font_size(13.0).text(texto))
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

pub fn dialogo_servidor(
    url: State<String>,
    aluno: State<String>,
    on_ok: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(440.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new("Servidor da turma".to_string()))
        .child(
            PopupContent::new()
                .child(
                    rect()
                        .direction(Direction::Vertical)
                        .spacing(8.0)
                        .width(Size::fill())
                        .child(label().font_size(13.0).text("URL (em casa: 127.0.0.1:50051)"))
                        .child(
                            CampoTexto::new(url)
                                .placeholder("http://127.0.0.1:50051")
                                .width(Size::fill())
                                .auto_focus(true),
                        )
                        .child(label().font_size(13.0).text("Aluno"))
                        .child(
                            CampoTexto::new(aluno)
                                .placeholder("local")
                                .width(Size::fill())
                                .on_submit(move |_| on_ok()),
                        ),
                ),
        )
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_ok())
                        .filled()
                        .child("Conectar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn dialogo_projetos(
    nomes: Vec<String>,
    novo: State<String>,
    on_abrir: impl Fn(String) + Copy + 'static,
    on_criar: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect()
        .direction(Direction::Vertical)
        .spacing(4.0)
        .width(Size::fill());
    if nomes.is_empty() {
        lista = lista.child(label().font_size(13.0).text("Nenhum projeto ainda."));
    }
    for nome in nomes {
        let n = nome.clone();
        lista = lista.child(
            Button::new()
                .on_press(move |_| on_abrir(n.clone()))
                .child(nome),
        );
    }
    Popup::new()
        .width(Size::px(420.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new("Projetos no servidor".to_string()))
        .child(
            PopupContent::new().child(
                rect()
                    .direction(Direction::Vertical)
                    .spacing(8.0)
                    .width(Size::fill())
                    .child(ScrollView::new().height(Size::px(180.0)).child(lista))
                    .child(label().font_size(13.0).text("Novo projeto"))
                    .child(
                        CampoTexto::new(novo)
                            .placeholder("nome")
                            .width(Size::fill())
                            .auto_focus(true)
                            .on_submit(move |_| on_criar()),
                    ),
            ),
        )
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_criar())
                        .filled()
                        .child("Criar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn dialogo_recuperar(
    nomes: Vec<String>,
    on_ok: impl Fn() + Copy + 'static,
    on_descartar: impl Fn() + Copy + 'static,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let lista = if nomes.is_empty() {
        "Nenhum rascunho.".to_string()
    } else {
        format!(
            "Há rascunhos não salvos:\n{}",
            nomes
                .into_iter()
                .take(8)
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    Popup::new()
        .width(Size::px(440.0))
        .on_close_request(move |_| on_fechar())
        .child(PopupTitle::new("Recuperar rascunhos".to_string()))
        .child(PopupContent::new().child(lista))
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_ok())
                        .filled()
                        .child("Recuperar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_descartar())
                        .child("Descartar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_fechar())
                        .child("Cancelar"),
                ),
        )
}

pub fn dialogo_conflito(
    nome: String,
    on_comparar: impl Fn() + Copy + 'static,
    on_recarregar: impl Fn() + Copy + 'static,
    on_salvar_como: impl Fn() + Copy + 'static,
    on_substituir: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(520.0))
        .on_close_request(move |_| on_cancelar())
        .child(PopupTitle::new(format!("Conflito ao salvar {nome}")))
        .child(PopupContent::new().child(
            "O arquivo mudou desde que esta aba foi aberta. Suas alterações continuam na aba. Substituir grava o texto da aba sobre a versão deste aviso.",
        ))
        .child(
            PopupButtons::new()
                .child(
                    Button::new()
                        .on_press(move |_| on_comparar())
                        .filled()
                        .child("Comparar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_recarregar())
                        .child("Recarregar"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_salvar_como())
                        .child("Salvar como"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_substituir())
                        .child("Substituir"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| on_cancelar())
                        .child("Cancelar"),
                ),
        )
}

pub fn dialogo_comparar(
    nome: String,
    aba: String,
    disco: String,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    Popup::new()
        .width(Size::px(640.0))
        .on_close_request(move |_| on_fechar())
        .child(PopupTitle::new(format!("Comparar — {nome}")))
        .child(
            PopupContent::new().child(
                rect()
                    .direction(Direction::Horizontal)
                    .spacing(12.0)
                    .width(Size::fill())
                    .height(Size::px(280.0))
                    .child(
                        ScrollView::new().width(Size::fill()).child(
                            rect()
                                .direction(Direction::Vertical)
                                .spacing(4.0)
                                .child(label().font_weight(FontWeight::BOLD).text("Sua aba"))
                                .child(label().font_size(12.0).text(aba)),
                        ),
                    )
                    .child(
                        ScrollView::new().width(Size::fill()).child(
                            rect()
                                .direction(Direction::Vertical)
                                .spacing(4.0)
                                .child(label().font_weight(FontWeight::BOLD).text("Arquivo salvo"))
                                .child(label().font_size(12.0).text(disco)),
                        ),
                    ),
            ),
        )
        .child(
            PopupButtons::new().child(
                Button::new()
                    .on_press(move |_| on_fechar())
                    .child("Fechar"),
            ),
        )
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
    largura: f32,
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
        .width(Size::px(largura))
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
                    CampoTexto::new(campo)
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
                    CampoTexto::new(entrada)
                        .placeholder("2 + 3")
                        .width(Size::fill())
                        .auto_focus(true)
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

fn canvas_pixels(w: u32, h: u32, rgba: Vec<u8>) -> Element {
    let w = w.max(1);
    let h = h.max(1);
    if let Some(handle) =
        freya::elements::image::ImageHandle::from_rgba(w, h, Bytes::from(rgba), AlphaType::Unpremul)
    {
        image(handle)
            .width(Size::px(w as f32))
            .height(Size::px(h as f32))
            .sampling_mode(SamplingMode::Nearest)
            .into()
    } else {
        rect()
            .width(Size::px(w as f32))
            .height(Size::px(h as f32))
            .into()
    }
}

pub fn painel_graficos(
    g: Graficos,
    paleta: Paleta,
    destacado: bool,
    on_destacar: impl Fn() + Copy + 'static,
    on_fechar_tela: impl Fn(u64) + Copy + 'static,
    on_clique: impl Fn(u64, f64, f64, f64, f64) + Copy + 'static,
    on_tecla: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let mut corpo = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(6.0)
        .padding(6.0);
    for folha in &g.folhas {
        let (w, h, rgba) = rgba_folha(folha);
        corpo = corpo.child(canvas_pixels(w, h, rgba));
    }
    for tela in &g.telas {
        let id = tela.id;
        let (w, h, rgba) = rgba_tela(tela);
        let wf = w as f64;
        let hf = h as f64;
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
                        .a11y_focusable(true)
                        .on_press(move |e: Event<PressEventData>| {
                            if let PressEventData::Mouse(m) = &*e {
                                let p = m.element_location;
                                on_clique(id, p.x, p.y, wf, hf);
                            }
                        })
                        .on_key_down(move |e: Event<KeyboardEventData>| {
                            let nome = tecla_de_freya(&e.key);
                            if nome.is_empty() {
                                return;
                            }
                            e.stop_propagation();
                            on_tecla(nome);
                        })
                        .child(canvas_pixels(w, h, rgba)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ver_marca_o_tema_ativo() {
        assert_eq!(rotulo_menu("Escuro", "", true), "✓ Escuro");
        assert_eq!(rotulo_menu("Claro", "", false), "Claro");
        assert_eq!(rotulo_menu("Arquivos", "F7", true), "✓ Arquivos    F7");
    }

    #[test]
    fn arrasto_respeita_minimo_e_maximo() {
        assert_eq!(aplicar_arrasto(Some(0.0), 200.0, false), Some(200.0));
        assert_eq!(
            aplicar_arrasto(Some(0.0), 10.0, false),
            Some(LARGURA_PAINEL_MIN)
        );
        assert_eq!(
            aplicar_arrasto(Some(0.0), 900.0, false),
            Some(LARGURA_PAINEL_MAX)
        );
        assert_eq!(aplicar_arrasto(Some(500.0), 220.0, true), Some(280.0));
    }
}


