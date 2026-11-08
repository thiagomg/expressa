//! Menubar, status bar and resizable panes.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use freya::{
    clipboard::Clipboard,
    code_editor::{CodeEditorData, CodeEditorThemeExt, EditorSyntaxTheme},
    engine::prelude::AlphaType,
    prelude::*,
    terminal::{
        SelectionType, Terminal, TerminalHandle, TerminalMouseButton,
    },
};

use crate::{
    ajuda::Sugestao,
    atalhos::{Acao, ATALHOS},
    prefs::Recente,
    debug::{Depuracao, MarcaGutter, marca_gutter, valor_observado},
    documentos::Abas,
    editor::{fonte_familia, new_editor},
    grafico::{Graficos, rgba_folha, rgba_tela, tecla_de_freya},
    projeto::Projeto,
    repl::{Repl, TipoLinha},
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AbaFundo {
    #[default]
    Saida,
    Repl,
}

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
    let mut pode_fechar = use_state(|| false);
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
        .on_global_pointer_down(move |_| {
            if aberto() != MenuAberto::Nenhum {
                pode_fechar.set(true);
            }
        })
        .child(menu_arquivo(
            aberto,
            pode_fechar,
            paleta,
            on_acao,
            on_recente,
            recentes,
        ))
        .child(botao_menu(
            "Executar",
            MenuAberto::Executar,
            aberto,
            pode_fechar,
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
            pode_fechar,
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
            pode_fechar,
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
                ("Substituir…", "Ctrl+H", Acao::Substituir),
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
            pode_fechar,
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

/// Freya tem [`Menu`]/[`MenuButton`]/[`SubMenu`]. [`ContextMenu`] ancora no
/// cursor (clique direito). O menubar ancora o [`Menu`] na caixa do título.
fn entrada_menu(
    titulo: &'static str,
    id: MenuAberto,
    mut aberto: State<MenuAberto>,
    mut pode_fechar: State<bool>,
    paleta: Paleta,
    construir: impl Fn() -> Menu + Clone + 'static,
) -> impl IntoElement {
    let mut area = use_state(|| None::<Area>);
    let este = aberto() == id;
    rect()
        .height(Size::fill())
        .main_align(Alignment::Center)
        .background(if este { paleta.botao } else { paleta.barra })
        .on_sized(move |e: Event<SizedEventData>| {
            area.set(Some(e.area));
        })
        .on_pointer_enter(move |_| {
            if aberto() != MenuAberto::Nenhum && aberto() != id {
                aberto.set(id);
                pode_fechar.set(false);
            }
        })
        .child(
            Button::new()
                .flat()
                .compact()
                .on_press(move |_| {
                    if aberto() == id {
                        aberto.set(MenuAberto::Nenhum);
                    } else {
                        aberto.set(id);
                        pode_fechar.set(false);
                    }
                })
                .child(titulo),
        )
        .maybe_child(este.then(move || {
            let a = area().unwrap_or_default();
            let x = a.origin.x.round();
            let y = (a.origin.y + a.size.height).round();
            rect()
                .layer(Layer::Overlay)
                .position(Position::new_global().left(x).top(y))
                .child(
                    construir()
                        .on_close(move |_| {
                            if pode_fechar() {
                                aberto.set(MenuAberto::Nenhum);
                            }
                        })
                        .on_escape(move |_| {
                            aberto.set(MenuAberto::Nenhum);
                        }),
                )
        }))
}

fn item_menu(
    nome: &'static str,
    atalho: &'static str,
    acao: Acao,
    marcado: bool,
    on_acao: impl Fn(Acao) + Copy + 'static,
    mut aberto: State<MenuAberto>,
) -> MenuButton {
    MenuButton::new()
        .on_press(move |_| {
            aberto.set(MenuAberto::Nenhum);
            on_acao(acao);
        })
        .child(rotulo_menu(nome, atalho, marcado))
}

fn menu_arquivo(
    mut aberto: State<MenuAberto>,
    pode_fechar: State<bool>,
    paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
    on_recente: impl Fn(Recente) + Copy + 'static,
    recentes: Vec<Recente>,
) -> impl IntoElement {
    entrada_menu(
        "Arquivo",
        MenuAberto::Arquivo,
        aberto,
        pode_fechar,
        paleta,
        move || {
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
            let mut menu = Menu::new();
            for (nome, atalho, acao) in itens {
                menu = menu.child(item_menu(nome, atalho, acao, false, on_acao, aberto));
            }
            let mut sub = SubMenu::new().label("Recentes");
            if recentes.is_empty() {
                sub = sub.child(MenuButton::new().child("Nenhum recente"));
            } else {
                for r in recentes.clone() {
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
                menu = menu.child(item_menu(nome, atalho, acao, false, on_acao, aberto));
            }
            menu
        },
    )
}

fn botao_menu(
    titulo: &'static str,
    id: MenuAberto,
    aberto: State<MenuAberto>,
    pode_fechar: State<bool>,
    paleta: Paleta,
    on_acao: impl Fn(Acao) + Copy + 'static,
    itens: &'static [(&'static str, &'static str, Acao)],
    marcado: impl Fn(Acao) -> bool + Copy + 'static,
) -> impl IntoElement {
    entrada_menu(titulo, id, aberto, pode_fechar, paleta, move || {
        let mut menu = Menu::new();
        for (nome, atalho, acao) in itens {
            let acao = *acao;
            menu = menu.child(item_menu(
                nome,
                atalho,
                acao,
                marcado(acao),
                on_acao,
                aberto,
            ));
        }
        menu
    })
}

pub fn barra_argumentos(
    paleta: Paleta,
    args: State<String>,
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
        .child(botao("A−", Acao::ZoomMenos, on_acao))
        .child(botao("A+", Acao::ZoomMais, on_acao))
}

pub fn abas_fundo(
    paleta: Paleta,
    atual: AbaFundo,
    on_saida: impl Fn() + Copy + 'static,
    on_repl: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    fn aba(
        nome: &'static str,
        esta: bool,
        paleta: Paleta,
        on_press: impl Fn() + Copy + 'static,
    ) -> impl IntoElement {
        rect()
            .padding((4.0, 10.0, 4.0, 10.0))
            .background(if esta { paleta.status } else { paleta.barra })
            .on_press(move |_| on_press())
            .child(
                label()
                    .font_size(12.0)
                    .font_weight(if esta {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text(nome),
            )
    }
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::px(28.0))
        .background(paleta.barra)
        .color(paleta.texto)
        .cross_align(Alignment::Center)
        .padding((0.0, 4.0, 0.0, 4.0))
        .spacing(4.0)
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
        .child(aba("Saída", atual == AbaFundo::Saida, paleta, on_saida))
        .child(aba("REPL", atual == AbaFundo::Repl, paleta, on_repl))
}

pub fn painel_console(
    handle: Option<TerminalHandle>,
    paleta: Paleta,
    fonte: f32,
    leia_aberta: bool,
    on_linha: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let a11y_id = use_a11y();
    let mut dimensions = use_state(|| (8.0f32, 16.0f32));
    let mut click_origin = use_state(|| None::<(usize, usize)>);
    let mut pediu_foco = use_state(|| false);
    if leia_aberta {
        if !*pediu_foco.read() {
            a11y_id.request_focus();
            pediu_foco.set(true);
        }
    } else if *pediu_foco.read() {
        pediu_foco.set(false);
    }

    let mut painel = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::fill())
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
        );
    if let Some(handle) = handle {
        painel = painel.child(
            Terminal::new(handle.clone())
                .font_family(fonte_familia().to_string())
                .font_size(fonte)
                .foreground(paleta.texto)
                .background(paleta.status)
                .a11y_id(a11y_id)
                .a11y_auto_focus(false)
                .on_measured(move |(char_width, line_height)| {
                    dimensions.set((char_width, line_height));
                })
                .on_mouse_down({
                    let handle = handle.clone();
                    move |e: Event<MouseEventData>| {
                        a11y_id.request_focus();
                        let (char_width, line_height) = dimensions();
                        if char_width <= 0.0 || line_height <= 0.0 {
                            return;
                        }
                        let col = (e.element_location.x / char_width as f64) as f32;
                        let row = (e.element_location.y / line_height as f64) as f32;
                        click_origin.set(Some((row as usize, col as usize)));
                        let button = match e.button {
                            Some(MouseButton::Middle) => TerminalMouseButton::Middle,
                            Some(MouseButton::Right) => TerminalMouseButton::Right,
                            _ => TerminalMouseButton::Left,
                        };
                        handle.mouse_down(row, col, button, SelectionType::Simple);
                    }
                })
                .on_mouse_move({
                    let handle = handle.clone();
                    move |e: Event<MouseEventData>| {
                        let (char_width, line_height) = dimensions();
                        if char_width <= 0.0 || line_height <= 0.0 {
                            return;
                        }
                        let col = (e.element_location.x / char_width as f64) as f32;
                        let row = (e.element_location.y / line_height as f64) as f32;
                        handle.mouse_move(row, col);
                    }
                })
                .on_mouse_up({
                    let handle = handle.clone();
                    move |e: Event<MouseEventData>| {
                        let (char_width, line_height) = dimensions();
                        if char_width <= 0.0 || line_height <= 0.0 {
                            return;
                        }
                        let col = (e.element_location.x / char_width as f64) as f32;
                        let row = (e.element_location.y / line_height as f64) as f32;
                        let button = match e.button {
                            Some(MouseButton::Middle) => TerminalMouseButton::Middle,
                            Some(MouseButton::Right) => TerminalMouseButton::Right,
                            _ => TerminalMouseButton::Left,
                        };
                        handle.mouse_up(row, col, button);
                        let origin = click_origin();
                        click_origin.set(None);
                        if button != TerminalMouseButton::Left {
                            return;
                        }
                        if let Some(texto) = handle.get_selected_text() {
                            on_linha(texto);
                        } else if origin == Some((row as usize, col as usize)) {
                            if let Some(url) = handle.hyperlink_at(row, col) {
                                on_linha(url);
                            }
                        }
                    }
                })
                .on_wheel({
                    let handle = handle.clone();
                    move |e: Event<WheelEventData>| {
                        let (char_width, line_height) = dimensions();
                        let (mouse_x, mouse_y) = e.element_location.to_tuple();
                        let col = (mouse_x / char_width as f64) as f32;
                        let row = (mouse_y / line_height as f64) as f32;
                        handle.wheel(e.delta_y, row, col);
                    }
                })
                .on_key_down({
                    let handle = handle.clone();
                    move |e: Event<KeyboardEventData>| {
                        let ctrl_shift =
                            e.modifiers.contains(Modifiers::CONTROL | Modifiers::SHIFT);
                        match &e.key {
                            Key::Character(ch)
                                if ctrl_shift && ch.eq_ignore_ascii_case("c") =>
                            {
                                if let Some(text) = handle.get_selected_text() {
                                    let _ = Clipboard::set(text);
                                }
                            }
                            Key::Character(ch)
                                if ctrl_shift && ch.eq_ignore_ascii_case("v") =>
                            {
                                if let Ok(text) = Clipboard::get() {
                                    let _ = handle.paste(&text);
                                }
                            }
                            _ => {
                                let _ = handle.write_key(&e.key, e.modifiers);
                            }
                        }
                    }
                }),
        );
    } else {
        painel = painel.child(
            label()
                .font_size(13.0)
                .padding(8.0)
                .text("Terminal indisponível"),
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

pub const TITULO_ATALHOS: &str = "Atalhos do teclado";

pub fn janela_atalhos(paleta: Paleta) -> impl IntoElement {
    let mut corpo = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(8.0)
        .padding((12.0, 16.0, 16.0, 16.0));
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
                    .width(Size::fill())
                    .spacing(24.0)
                    .child(
                        label()
                            .font_size(13.0)
                            .font_family(fonte_familia())
                            .width(Size::px(200.0))
                            .text((*teclas).to_string()),
                    )
                    .child(label().font_size(13.0).text((*o_que).to_string())),
            );
        }
    }
    rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::fill())
        .content(Content::flex())
        .background(paleta.fundo)
        .color(paleta.texto)
        .a11y_focusable(true)
        .on_key_down(move |e: Event<KeyboardEventData>| {
            if matches!(e.key, Key::Named(NamedKey::Escape)) {
                e.stop_propagation();
                Platform::get().close_window(Platform::window_id());
            }
        })
        .child(
            ScrollView::new()
                .height(Size::flex(1.0))
                .child(corpo),
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
    let mut lista = ScrollView::new()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::fill())
        .spacing(4.0);
    for (i, aba) in abas.itens.iter().enumerate() {
        let id = aba.id;
        let atual = i == abas.atual;
        let fundo = if atual { paleta.botao } else { paleta.barra };
        lista = lista.child(
            rect()
                .key(DiffKey::U64(id))
                .direction(Direction::Horizontal)
                .background(fundo)
                .corner_radius(4.0)
                .padding((4.0, 8.0, 4.0, 8.0))
                .spacing(6.0)
                .cross_align(Alignment::Center)
                .on_press(move |_| on_selecionar(i))
                .on_pointer_down(move |e: Event<PointerEventData>| {
                    if e.data().button() == Some(MouseButton::Middle) {
                        e.stop_propagation();
                        on_fechar(id);
                    }
                })
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
    rect()
        .width(Size::fill())
        .height(Size::px(36.0))
        .background(paleta.barra)
        .color(paleta.texto)
        .padding((2.0, 6.0, 2.0, 6.0))
        .child(lista)
}

fn borda_janela(paleta: Paleta) -> Border {
    Border::new()
        .width(BorderWidth {
            top: 1.0,
            right: 1.0,
            bottom: 1.0,
            left: 1.0,
        })
        .fill(paleta.borda)
}

/// Diálogo modal com borda e barra de título, para não misturar com o editor.
fn quadro_dialogo(
    paleta: Paleta,
    titulo: String,
    largura: f32,
    on_fechar: impl Fn() + Copy + 'static,
    corpo: impl IntoElement,
    botoes: impl IntoElement,
) -> impl IntoElement {
    rect()
        .layer(Layer::Overlay)
        .position(Position::new_global().top(0.).left(0.))
        .width(Size::window_percent(100.))
        .height(Size::window_percent(100.))
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            if matches!(e.key, Key::Named(NamedKey::Escape)) {
                e.stop_propagation();
                on_fechar();
            }
        })
        .child(
            rect()
                .position(Position::new_global().top(0.).left(0.))
                .width(Size::window_percent(100.))
                .height(Size::window_percent(100.))
                .background((0, 0, 0, 150))
                .on_press(move |_| on_fechar()),
        )
        .child(
            rect()
                .position(Position::new_global().top(0.).left(0.))
                .width(Size::window_percent(100.))
                .height(Size::window_percent(100.))
                .center()
                .child(
                    rect()
                        .direction(Direction::Vertical)
                        .width(Size::px(largura))
                        .background(paleta.fundo)
                        .color(paleta.texto)
                        .corner_radius(8.0)
                        .border(borda_janela(paleta))
                        .shadow(Shadow::new().y(6.).blur(16.).color((0, 0, 0, 90)))
                        .on_press(|e: Event<PressEventData>| e.stop_propagation())
                        .child(
                            rect()
                                .direction(Direction::Horizontal)
                                .width(Size::fill())
                                .background(paleta.barra)
                                .padding((8.0, 10.0, 8.0, 10.0))
                                .main_align(Alignment::SpaceBetween)
                                .cross_align(Alignment::Center)
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
                                .child(
                                    label()
                                        .font_size(14.0)
                                        .font_weight(FontWeight::BOLD)
                                        .text(titulo),
                                )
                                .child(
                                    Button::new()
                                        .flat()
                                        .compact()
                                        .on_press(move |_| on_fechar())
                                        .child("×"),
                                ),
                        )
                        .child(rect().width(Size::fill()).padding((12.0, 14.0, 12.0, 14.0)).child(corpo))
                        .child(
                            rect()
                                .direction(Direction::Horizontal)
                                .width(Size::fill())
                                .padding((0.0, 12.0, 12.0, 12.0))
                                .spacing(8.0)
                                .main_align(Alignment::End)
                                .child(botoes),
                        ),
                ),
        )
}

pub fn dialogo_confirmar(
    paleta: Paleta,
    nome: String,
    on_salvar: impl Fn() + Copy + 'static,
    on_descartar: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        "Alterações não salvas".into(),
        420.0,
        on_cancelar,
        label().font_size(13.0).text(format!("Salvar as alterações de {nome}?")),
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    substituir: Option<(
        State<String>,
        impl Fn() + Copy + 'static,
        impl Fn() + Copy + 'static,
    )>,
) -> impl IntoElement {
    let altura = if substituir.is_some() { 68.0 } else { 36.0 };
    let mut barra = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::px(altura))
        .background(paleta.botao)
        .color(paleta.texto)
        .padding((4.0, 8.0, 4.0, 8.0))
        .spacing(4.0);
    barra = barra.child(
        rect()
            .direction(Direction::Horizontal)
            .width(Size::fill())
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
            ),
    );
    if let Some((novo, on_uma, on_todas)) = substituir {
        barra = barra.child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .spacing(8.0)
                .cross_align(Alignment::Center)
                .child(label().font_size(13.0).text("Trocar"))
                .child(
                    CampoTexto::new(novo)
                        .placeholder("substituição")
                        .width(Size::px(240.0)),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_uma())
                        .child("Substituir"),
                )
                .child(
                    Button::new()
                        .compact()
                        .on_press(move |_| on_todas())
                        .child("Todas"),
                ),
        );
    }
    barra
}

pub const LARGURA_ARVORE: f32 = 240.0;
pub const LARGURA_DEBUG: f32 = 280.0;
pub const LARGURA_PAINEL_MIN: f32 = 140.0;
pub const LARGURA_PAINEL_MAX: f32 = 480.0;
pub const ALTURA_CONSOLE: f32 = 160.0;
pub const ALTURA_CONSOLE_MIN: f32 = 64.0;
pub const ALTURA_CONSOLE_MAX: f32 = 700.0;

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
        .cursor(CursorIcon::EwResize)
        .on_pointer_down(move |e: Event<PointerEventData>| {
            let x = e.data().global_location().x as f32;
            let atual = *largura.read();
            arrasto.set(Some(if inverter { x + atual } else { x - atual }));
        })
}

pub fn alca_horizontal(
    paleta: Paleta,
    altura: State<f32>,
    mut arrasto: State<Option<f32>>,
) -> impl IntoElement {
    rect()
        .width(Size::fill())
        .height(Size::px(6.0))
        .background(paleta.borda)
        .cursor(CursorIcon::NsResize)
        .on_pointer_down(move |e: Event<PointerEventData>| {
            let y = e.data().global_location().y as f32;
            let atual = *altura.read();
            arrasto.set(Some(y + atual));
        })
}

pub fn linha_gutter(
    indice: usize,
    arquivo: &str,
    pontos: &[(String, u32)],
    pausado: bool,
    pausa_arquivo: &str,
    pausa_linha: u32,
    par: bool,
    erro: bool,
    font_size: f32,
    line_height: f32,
    paleta: Paleta,
    on_linha: impl Fn(u32) + Copy + 'static,
) -> impl IntoElement {
    let linha = (indice + 1) as u32;
    let marca = marca_gutter(pontos, arquivo, linha, pausado, pausa_arquivo, pausa_linha);
    let fundo = match marca {
        MarcaGutter::Atual | MarcaGutter::PontoAtual => (80, 70, 20, 255),
        _ if erro => (90, 30, 30, 255),
        _ if par => (30, 70, 90, 255),
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
        .padding((0., 4.0, 0., 2.0))
        .spacing(2.0)
        .on_press(move |_| on_linha(linha))
        .child(
            label()
                .font_size(font_size * 0.7)
                .width(Size::px(11.0))
                .color(cor_ponto)
                .text(simbolo),
        )
        .child(
            rect()
                .width(Size::fill())
                .main_align(Alignment::End)
                .child(
                    label()
                        .font_size(font_size * 0.75)
                        .color(paleta.texto)
                        .text(format!("{linha}")),
                ),
        )
}

pub fn aplicar_arrasto(arrasto: Option<f32>, x: f32, inverter: bool) -> Option<f32> {
    arrasto.map(|origem| {
        let bruto = if inverter { origem - x } else { x - origem };
        bruto.clamp(LARGURA_PAINEL_MIN, LARGURA_PAINEL_MAX)
    })
}

pub fn aplicar_arrasto_v(arrasto: Option<f32>, y: f32) -> Option<f32> {
    arrasto.map(|origem| (origem - y).clamp(ALTURA_CONSOLE_MIN, ALTURA_CONSOLE_MAX))
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
    on_atualizar: impl Fn() + Copy + 'static,
    on_contexto: impl Fn(String) + Copy + 'static,
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
                .on_press({
                    let rel = rel.clone();
                    move |_| on_abrir(rel.clone())
                })
                .on_secondary_down({
                    let rel = rel.clone();
                    move |_| on_contexto(rel.clone())
                })
                .child(label().font_size(13.0).text(format!("{marca}{}", no.nome))),
        );
    }
    rect()
        .direction(Direction::Vertical)
        .width(Size::px(largura))
        .height(Size::fill())
        .content(Content::flex())
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
                    rect()
                        .direction(Direction::Horizontal)
                        .spacing(4.0)
                        .child(
                            Button::new()
                                .flat()
                                .compact()
                                .on_press(move |_| on_atualizar())
                                .child("↻"),
                        )
                        .child(
                            Button::new()
                                .flat()
                                .compact()
                                .on_press(move |_| on_projeto())
                                .child("…"),
                        ),
                ),
        )
        .child(ScrollView::new().height(Size::flex(1.0)).child(lista))
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
    paleta: Paleta,
    valor: State<String>,
    on_ir: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        "Ir para linha".into(),
        360.0,
        on_cancelar,
        CampoTexto::new(valor)
            .placeholder("número da linha")
            .width(Size::fill())
            .auto_focus(true)
            .on_submit(move |_| on_ir()),
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    paleta: Paleta,
    titulo: String,
    valor: State<String>,
    on_ok: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        titulo,
        360.0,
        on_cancelar,
        CampoTexto::new(valor)
            .placeholder("nome")
            .width(Size::fill())
            .auto_focus(true)
            .on_submit(move |_| on_ok()),
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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

pub fn janela_ajuda(
    escolhida: State<Option<String>>,
    paleta: Paleta,
) -> impl IntoElement {
    let filtro = use_state(String::new);
    let q = filtro.read().to_lowercase();
    let entradas = crate::ajuda::entradas_catalogo();
    let mut lista = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(1.0)
        .padding((4.0, 6.0, 4.0, 6.0));
    let mut grupo_atual = String::new();
    for e in &entradas {
        if !q.is_empty()
            && !e.chave.to_lowercase().contains(&q)
            && !e.rotulo.to_lowercase().contains(&q)
        {
            continue;
        }
        if e.grupo != grupo_atual {
            grupo_atual = e.grupo.clone();
            lista = lista.child(
                label()
                    .font_size(11.0)
                    .font_weight(FontWeight::BOLD)
                    .color((140, 140, 148))
                    .text(e.grupo.clone()),
            );
        }
        let chave = e.chave.clone();
        let esta = escolhida.read().as_deref() == Some(chave.as_str());
        lista = lista.child(
            rect()
                .width(Size::fill())
                .padding((3.0, 6.0, 3.0, 6.0))
                .background(if esta { paleta.botao } else { paleta.fundo })
                .on_press({
                    let chave = chave.clone();
                    move |_| {
                        let mut escolhida = escolhida;
                        escolhida.set(Some(chave.clone()));
                    }
                })
                .child(
                    label()
                        .font_size(13.0)
                        .font_family(fonte_familia())
                        .text(e.rotulo.clone()),
                ),
        );
    }
    let ficha = escolhida
        .read()
        .as_deref()
        .and_then(crate::ajuda::ficha)
        .unwrap_or_else(|| "Escolha uma função à esquerda.".into());
    rect()
        .direction(Direction::Horizontal)
        .width(Size::fill())
        .height(Size::fill())
        .content(Content::flex())
        .background(paleta.fundo)
        .color(paleta.texto)
        .child(
            rect()
                .direction(Direction::Vertical)
                .width(Size::px(280.0))
                .height(Size::fill())
                .content(Content::flex())
                .background(paleta.barra)
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
                        .width(Size::fill())
                        .padding(8.0)
                        .child(
                            CampoTexto::new(filtro)
                                .placeholder("filtrar (escreva, matriz::zeros, …)")
                                .width(Size::fill()),
                        ),
                )
                .child(
                    ScrollView::new()
                        .height(Size::flex(1.0))
                        .child(lista),
                ),
        )
        .child(
            ScrollView::new()
                .width(Size::flex(1.0))
                .height(Size::fill())
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(12.0)
                        .child(
                            label()
                                .font_size(13.0)
                                .font_family(fonte_familia())
                                .text(ficha),
                        ),
                ),
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
        .child(lista)
}

pub fn dialogo_servidor(
    paleta: Paleta,
    url: State<String>,
    aluno: State<String>,
    on_ok: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        "Servidor da turma".into(),
        440.0,
        on_cancelar,
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
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    paleta: Paleta,
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
    quadro_dialogo(
        paleta,
        "Projetos no servidor".into(),
        420.0,
        on_cancelar,
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
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    paleta: Paleta,
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
            nomes.into_iter().take(8).collect::<Vec<_>>().join("\n")
        )
    };
    quadro_dialogo(
        paleta,
        "Recuperar rascunhos".into(),
        440.0,
        on_fechar,
        label().font_size(13.0).text(lista),
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    paleta: Paleta,
    nome: String,
    on_comparar: impl Fn() + Copy + 'static,
    on_recarregar: impl Fn() + Copy + 'static,
    on_salvar_como: impl Fn() + Copy + 'static,
    on_substituir: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        format!("Conflito ao salvar {nome}"),
        560.0,
        on_cancelar,
        label().font_size(13.0).text(
            "O arquivo mudou desde que esta aba foi aberta. Suas alterações continuam na aba. Substituir grava o texto da aba sobre a versão deste aviso.",
        ),
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    paleta: Paleta,
    nome: String,
    aba: String,
    disco: String,
    on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        format!("Comparar — {nome}"),
        720.0,
        on_fechar,
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
                        .child(label().font_size(12.0).font_family(fonte_familia()).text(aba)),
                ),
            )
            .child(
                ScrollView::new().width(Size::fill()).child(
                    rect()
                        .direction(Direction::Vertical)
                        .spacing(4.0)
                        .child(label().font_weight(FontWeight::BOLD).text("Arquivo salvo"))
                        .child(label().font_size(12.0).font_family(fonte_familia()).text(disco)),
                ),
            ),
        Button::new()
            .on_press(move |_| on_fechar())
            .child("Fechar"),
    )
}

pub fn dialogo_apagar(
    paleta: Paleta,
    rel: String,
    on_ok: impl Fn() + Copy + 'static,
    on_cancelar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    quadro_dialogo(
        paleta,
        "Apagar".into(),
        400.0,
        on_cancelar,
        label().font_size(13.0).text(format!("Apagar {rel}?")),
        rect()
            .direction(Direction::Horizontal)
            .spacing(8.0)
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
    if dep.vars.is_empty() {
        vars = vars.child(
            label()
                .font_size(12.0)
                .color((140, 140, 148))
                .text(if dep.pausado {
                    "(nenhuma variável neste ponto)"
                } else {
                    "(depurar com F6 para ver as variáveis)"
                }),
        );
    }
    for v in &dep.vars {
        vars = vars.child(
            label()
                .font_size(12.0)
                .font_family(fonte_familia())
                .text(format!("[{}] {} = {}", v.escopo, v.nome, v.valor)),
        );
    }
    let mut pilha = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(1.0);
    if dep.pilha.is_empty() {
        pilha = pilha.child(
            label()
                .font_size(12.0)
                .color((140, 140, 148))
                .text("(pilha vazia)"),
        );
    }
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
                        .font_family(fonte_familia())
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
        .content(Content::flex())
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
                .font_family(fonte_familia())
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
        .child(
            ScrollView::new()
                .height(Size::flex(1.0))
                .child(vars),
        )
        .child(
            label()
                .font_size(11.0)
                .font_weight(FontWeight::BOLD)
                .text("Pilha"),
        )
        .child(
            ScrollView::new()
                .height(Size::flex(1.0))
                .child(pilha),
        )
}

pub fn painel_repl(
    repl: Repl,
    paleta: Paleta,
    entrada: State<String>,
    on_enviar: impl Fn() + Copy + 'static,
    on_parar: impl Fn() + Copy + 'static,
    on_limpar: impl Fn() + Copy + 'static,
    on_reiniciar: impl Fn() + Copy + 'static,
    _on_fechar: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let mut lista = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .spacing(0.0)
        .padding((4.0, 8.0, 4.0, 8.0));
    if repl.linhas.is_empty() {
        lista = lista.child(
            label()
                .font_size(13.0)
                .font_family(fonte_familia())
                .color((140, 140, 148))
                .text("Cada linha roda na hora. As variáveis ficam para as próximas."),
        );
    }
    for l in &repl.linhas {
        let cor = match l.tipo {
            TipoLinha::Eco => (140, 140, 148),
            TipoLinha::Erro => (229, 57, 53),
            TipoLinha::Saida => paleta.texto,
        };
        let texto = if l.texto.is_empty() {
            " ".to_string()
        } else {
            l.texto.clone()
        };
        lista = lista.child(
            label()
                .font_size(13.0)
                .font_family(fonte_familia())
                .color(cor)
                .text(texto),
        );
    }
    rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::fill())
        .content(Content::flex())
        .background(paleta.status)
        .color(paleta.texto)
        .child(
            ScrollView::new()
                .height(Size::flex(1.0))
                .child(lista),
        )
        .child(
            rect()
                .direction(Direction::Horizontal)
                .width(Size::fill())
                .padding((4.0, 6.0, 4.0, 6.0))
                .spacing(6.0)
                .cross_align(Alignment::Center)
                .child(label().font_size(13.0).font_family(fonte_familia()).text(repl.prompt.clone()))
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
    canvas_pixels_com_tamanho(w, h, rgba, Size::px(w.max(1) as f32), Size::px(h.max(1) as f32))
}

fn canvas_pixels_preencher(w: u32, h: u32, rgba: Vec<u8>) -> Element {
    canvas_pixels_com_tamanho(w, h, rgba, Size::fill(), Size::fill())
}

fn canvas_pixels_com_tamanho(w: u32, h: u32, rgba: Vec<u8>, largura: Size, altura: Size) -> Element {
    let w = w.max(1);
    let h = h.max(1);
    if let Some(handle) =
        freya::elements::image::ImageHandle::from_rgba(w, h, Bytes::from(rgba), AlphaType::Unpremul)
    {
        image(handle)
            .width(largura)
            .height(altura)
            .sampling_mode(SamplingMode::Nearest)
            .into()
    } else {
        rect().width(largura).height(altura).into()
    }
}

pub fn painel_graficos(
    g: Graficos,
    paleta: Paleta,
    janela_propria: bool,
    on_destacar: impl Fn() + Copy + 'static,
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
    rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(if janela_propria {
            Size::fill()
        } else {
            Size::px(240.0)
        })
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
                        .child(if janela_propria { "Acoplar" } else { "Destacar" }),
                ),
        )
        .child(ScrollView::new().height(Size::fill()).child(corpo))
}

pub fn janela_tela_grafica(
    id: u64,
    graficos: State<crate::grafico::Graficos>,
    paleta: Paleta,
    on_clique: impl Fn(u64, f64, f64, f64, f64) + Copy + 'static,
    on_tecla: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    let tela = graficos.read().telas.iter().find(|t| t.id == id).cloned();
    let size = Platform::get().current_window().root_size;
    let mut caixa = rect()
        .direction(Direction::Vertical)
        .width(Size::fill())
        .height(Size::fill())
        .background(paleta.status)
        .a11y_focusable(true)
        .on_key_down(move |e: Event<KeyboardEventData>| {
            let nome = tecla_de_freya(&e.key);
            if nome.is_empty() {
                return;
            }
            e.stop_propagation();
            on_tecla(nome);
        });
    if let Some(tela) = tela {
        let (w, h, rgba) = rgba_tela(&tela);
        caixa = caixa
            .on_press(move |e: Event<PressEventData>| {
                if let PressEventData::Mouse(m) = &*e {
                    let area = *size.read();
                    on_clique(
                        id,
                        m.element_location.x,
                        m.element_location.y,
                        area.width as f64,
                        area.height as f64,
                    );
                }
            })
            .child(canvas_pixels_preencher(w, h, rgba));
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

    #[test]
    fn arrasto_vertical_cresce_para_cima() {
        assert_eq!(aplicar_arrasto_v(Some(400.0), 240.0), Some(160.0));
        assert_eq!(aplicar_arrasto_v(Some(400.0), 390.0), Some(ALTURA_CONSOLE_MIN));
        assert_eq!(aplicar_arrasto_v(Some(800.0), 0.0), Some(ALTURA_CONSOLE_MAX));
        assert_eq!(aplicar_arrasto_v(None, 100.0), None);
    }
}


