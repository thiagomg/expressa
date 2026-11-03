//! REPL tab of the bottom panel: try an expression without making a file.
//! The interpreter runs on its own thread (`CanalRepl`), so a slow line
//! never freezes the window, and Parar interrupts it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

use expressa::runtime::{
    CanalRepl, ConteudoRepl, EventoRepl, FimRepl, FluxoRepl, IdSessao, REPL_MAX_CODIGO, TextoRepl,
};
use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, Orientation, ScrolledWindow, TextView};

const INTERVALO_CONSULTA: Duration = Duration::from_millis(30);
const TEMPO_POR_QUADRO: Duration = Duration::from_millis(4);
const EVENTOS_POR_QUADRO: usize = 128;
const BYTES_POR_QUADRO: usize = 64 * 1024;
const MAX_CARACTERES_SAIDA: i32 = 256 * 1024;
const MAX_HISTORICO: usize = 100;

type Trechos = Vec<(Option<&'static str>, String)>;

fn acrescentar(trechos: &mut Trechos, texto: String, tag: Option<&'static str>) {
    if texto.is_empty() {
        return;
    }
    if let Some((anterior, conteudo)) = trechos.last_mut()
        && *anterior == tag
    {
        conteudo.push_str(&texto);
    } else {
        trechos.push((tag, texto));
    }
}

pub struct PainelRepl {
    pub widget: GtkBox,
    saida: TextView,
    entrada: Entry,
    prompt: Label,
    sessao: RefCell<Option<CanalRepl>>,
    consulta: RefCell<Option<glib::SourceId>>,
    stdout: RefCell<TextoRepl>,
    stderr: RefCell<TextoRepl>,
    btn_parar: Button,
    /// Lines typed so far of code that is not complete yet (`se x {`).
    pendente: RefCell<String>,
    historico: RefCell<Vec<String>>,
    /// Position while walking the history with ↑/↓ (`len` = new line).
    no_historico: Cell<usize>,
}

impl PainelRepl {
    pub fn new() -> Rc<Self> {
        let saida = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(6)
            .build();
        let tabela = saida.buffer().tag_table();
        let erro = gtk::TextTag::new(Some("erro"));
        erro.set_foreground(Some("#d32f2f"));
        let eco = gtk::TextTag::new(Some("eco"));
        eco.set_foreground_rgba(Some(&gtk::gdk::RGBA::new(0.5, 0.5, 0.5, 1.0)));
        tabela.add(&erro);
        tabela.add(&eco);
        let rolagem = ScrolledWindow::builder()
            .child(&saida)
            .vexpand(true)
            .hexpand(true)
            .build();
        let prompt = Label::new(Some(">"));
        prompt.add_css_class("monospace");
        let entrada = Entry::builder()
            .hexpand(true)
            .placeholder_text("digite uma expressão e Enter (ex.: 2 + 3, escreva(\"oi\"))")
            .build();
        entrada.set_max_length((REPL_MAX_CODIGO - 1) as i32);
        let parar = Button::with_label("Parar");
        parar.set_sensitive(false);
        parar.set_tooltip_text(Some("Interrompe a linha que está rodando"));
        let limpar = Button::with_label("Limpar");
        let reiniciar = Button::with_label("Reiniciar");
        reiniciar.set_tooltip_text(Some("Começa de novo: esquece as variáveis"));
        let linha = GtkBox::new(Orientation::Horizontal, 6);
        linha.set_margin_start(6);
        linha.set_margin_end(6);
        linha.set_margin_bottom(4);
        linha.append(&prompt);
        linha.append(&entrada);
        linha.append(&parar);
        linha.append(&limpar);
        linha.append(&reiniciar);
        let widget = GtkBox::new(Orientation::Vertical, 4);
        widget.append(&rolagem);
        widget.append(&linha);

        let painel = Rc::new(Self {
            widget,
            saida,
            entrada,
            prompt,
            sessao: RefCell::new(None),
            consulta: RefCell::new(None),
            stdout: RefCell::new(TextoRepl::default()),
            stderr: RefCell::new(TextoRepl::default()),
            btn_parar: parar.clone(),
            pendente: RefCell::new(String::new()),
            historico: RefCell::new(Vec::new()),
            no_historico: Cell::new(0),
        });
        painel.escrever(
            "REPL da Expressa: cada linha roda na hora e as variáveis ficam para as próximas.\n",
            Some("eco"),
        );
        {
            let p = Rc::downgrade(&painel);
            painel.entrada.connect_activate(move |_| {
                if let Some(p) = p.upgrade() {
                    p.enviar();
                }
            });
        }
        {
            let p = Rc::downgrade(&painel);
            parar.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.parar();
                }
            });
        }
        {
            let p = Rc::downgrade(&painel);
            limpar.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.saida.buffer().set_text("");
                }
            });
        }
        {
            let p = Rc::downgrade(&painel);
            reiniciar.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.reiniciar();
                }
            });
        }
        painel.historico_com_setas();
        {
            let p = Rc::downgrade(&painel);
            painel.widget.connect_destroy(move |_| {
                if let Some(p) = p.upgrade() {
                    p.encerrar();
                }
            });
        }
        painel.iniciar();
        painel
    }

    pub fn focar(&self) {
        self.entrada.grab_focus();
    }

    fn escrever(&self, texto: &str, tag: Option<&str>) {
        let buffer = self.saida.buffer();
        let mut fim = buffer.end_iter();
        match tag {
            Some(t) => buffer.insert_with_tags_by_name(&mut fim, texto, &[t]),
            None => buffer.insert(&mut fim, texto),
        }
        crate::console::trim_output(&buffer);
        let excesso = buffer.char_count() - MAX_CARACTERES_SAIDA;
        if excesso > 0 {
            let mut inicio = buffer.start_iter();
            let mut fim = buffer.iter_at_offset(excesso);
            buffer.delete(&mut inicio, &mut fim);
        }
        let mut fim = buffer.end_iter();
        self.saida.scroll_to_iter(&mut fim, 0.0, false, 0.0, 0.0);
    }

    fn enviar(self: &Rc<Self>) {
        let linha = self.entrada.text().to_string();
        if self.sessao.borrow().is_none() {
            return;
        }
        if self.pendente.borrow().len() + linha.len() + 1 > REPL_MAX_CODIGO {
            self.escrever(
                "código grande demais para o REPL (limite: 64 KiB)\n",
                Some("erro"),
            );
            return;
        }
        self.entrada.set_text("");
        let primeira = self.pendente.borrow().is_empty();
        if primeira && matches!(linha.trim(), "limpe" | "cls" | "limpe_tela") {
            self.saida.buffer().set_text("");
            return;
        }
        if primeira && linha.trim().is_empty() {
            return;
        }
        {
            let mut h = self.historico.borrow_mut();
            if !linha.trim().is_empty() && h.last() != Some(&linha) {
                h.push(linha.clone());
                if h.len() > MAX_HISTORICO {
                    h.remove(0);
                }
            }
            self.no_historico.set(h.len());
        }
        let eco = if primeira { "> " } else { "... " };
        self.escrever(&format!("{eco}{linha}\n"), Some("eco"));
        let codigo = {
            let mut p = self.pendente.borrow_mut();
            p.push_str(&linha);
            p.push('\n');
            if expressa::parser::needs_more_input(&p) {
                self.prompt.set_text("...");
                return;
            }
            std::mem::take(&mut *p)
        };
        self.prompt.set_text(">");
        let resultado = self
            .sessao
            .borrow_mut()
            .as_mut()
            .unwrap()
            .enviar(codigo.clone());
        if let Err(erro) = resultado {
            // Preserve the complete command for retry when the bounded queue fills.
            self.entrada.set_text(codigo.trim_end_matches('\n'));
            self.escrever(&format!("{erro}\n"), Some("erro"));
        }
        self.atualizar_parada();
    }

    fn iniciar(self: &Rc<Self>) {
        match CanalRepl::nova() {
            Ok(sessao) => {
                let id = sessao.id();
                *self.sessao.borrow_mut() = Some(sessao);
                self.entrada.set_sensitive(true);
                let p = Rc::downgrade(self);
                let fonte = glib::timeout_add_local(INTERVALO_CONSULTA, move || {
                    let Some(p) = p.upgrade() else {
                        return glib::ControlFlow::Break;
                    };
                    let continuar = p.drenar(id);
                    if continuar == glib::ControlFlow::Break
                        && p.sessao.borrow().as_ref().is_some_and(|s| s.id() == id)
                    {
                        p.consulta.borrow_mut().take();
                    }
                    continuar
                });
                *self.consulta.borrow_mut() = Some(fonte);
            }
            Err(e) => {
                self.entrada.set_sensitive(false);
                self.escrever(
                    &format!("não foi possível iniciar o REPL: {e}\n"),
                    Some("erro"),
                );
            }
        }
    }

    fn atualizar_parada(&self) {
        self.btn_parar.set_sensitive(
            self.sessao
                .borrow()
                .as_ref()
                .is_some_and(|s| s.pendentes() > 0),
        );
    }

    /// One source per session. Work, transfer size and display memory are bounded
    /// independently; even an infinite printer must yield to the GTK main loop.
    fn drenar(&self, id: IdSessao) -> glib::ControlFlow {
        if !self.sessao.borrow().as_ref().is_some_and(|s| s.id() == id) {
            return glib::ControlFlow::Break;
        }
        let inicio = Instant::now();
        let mut bytes = 0;
        let mut trechos = Trechos::new();
        let mut continuar = glib::ControlFlow::Continue;
        for _ in 0..EVENTOS_POR_QUADRO {
            if inicio.elapsed() >= TEMPO_POR_QUADRO || bytes >= BYTES_POR_QUADRO {
                break;
            }
            let evento = self.sessao.borrow_mut().as_mut().unwrap().receber();
            match evento {
                Ok(evento) => {
                    bytes += match &evento.conteudo {
                        ConteudoRepl::Saida { bytes, .. } => bytes.len(),
                        ConteudoRepl::Resultado(Ok(Some(s)) | Err(s)) => s.len(),
                        _ => 0,
                    };
                    if self.mostrar(evento, &mut trechos) {
                        continuar = glib::ControlFlow::Break;
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.terminar_texto(&mut trechos);
                    acrescentar(
                        &mut trechos,
                        "a sessão do REPL foi desconectada; aperte Reiniciar\n".into(),
                        Some("erro"),
                    );
                    self.sessao.borrow_mut().as_mut().unwrap().encerrar();
                    self.entrada.set_sensitive(false);
                    continuar = glib::ControlFlow::Break;
                    break;
                }
            }
        }
        for (tag, texto) in trechos {
            self.escrever(&texto, tag);
        }
        self.atualizar_parada();
        continuar
    }

    fn terminar_texto(&self, trechos: &mut Trechos) {
        acrescentar(trechos, self.stdout.borrow_mut().terminar(), None);
        acrescentar(trechos, self.stderr.borrow_mut().terminar(), Some("erro"));
    }

    /// Returns true on termination. Reject events from an earlier session.
    fn mostrar(&self, evento: EventoRepl, trechos: &mut Trechos) -> bool {
        if !self
            .sessao
            .borrow()
            .as_ref()
            .is_some_and(|s| s.id() == evento.sessao)
        {
            return false;
        }
        match evento.conteudo {
            ConteudoRepl::Saida { fluxo, bytes } => {
                let (decoder, tag) = match fluxo {
                    FluxoRepl::Stdout => (&self.stdout, None),
                    FluxoRepl::Stderr => (&self.stderr, Some("erro")),
                };
                acrescentar(trechos, decoder.borrow_mut().decodificar(&bytes), tag);
            }
            ConteudoRepl::Resultado(resultado) => {
                self.terminar_texto(trechos);
                match resultado {
                    Ok(Some(valor)) => acrescentar(trechos, format!("{valor}\n"), None),
                    Ok(None) => {}
                    Err(e) => acrescentar(trechos, format!("{e}\n"), Some("erro")),
                }
            }
            ConteudoRepl::Terminou(fim) => {
                self.terminar_texto(trechos);
                let (texto, tag) = match fim {
                    FimRepl::Saida(code) => (
                        format!("sessão encerrada (código {code}); aperte Reiniciar\n"),
                        Some("eco"),
                    ),
                    FimRepl::Encerrada => {
                        ("sessão encerrada; aperte Reiniciar\n".into(), Some("eco"))
                    }
                    FimRepl::Falha(e) => (
                        format!("falha na sessão: {e}; aperte Reiniciar\n"),
                        Some("erro"),
                    ),
                };
                acrescentar(trechos, texto, tag);
                self.entrada.set_sensitive(false);
                return true;
            }
        }
        false
    }

    fn parar(&self) {
        if let Some(sessao) = self.sessao.borrow().as_ref() {
            sessao.parar();
        }
    }

    /// Called only on confirmed window close or panel disposal, not on tab hide.
    pub fn encerrar(&self) {
        if let Some(fonte) = self.consulta.borrow_mut().take() {
            fonte.remove();
        }
        self.sessao.borrow_mut().take();
        self.btn_parar.set_sensitive(false);
        self.entrada.set_sensitive(false);
    }

    fn reiniciar(self: &Rc<Self>) {
        self.encerrar();
        self.pendente.borrow_mut().clear();
        *self.stdout.borrow_mut() = TextoRepl::default();
        *self.stderr.borrow_mut() = TextoRepl::default();
        self.prompt.set_text(">");
        self.escrever(
            "— sessão nova: as variáveis foram esquecidas —\n",
            Some("eco"),
        );
        self.iniciar();
    }

    fn historico_com_setas(self: &Rc<Self>) {
        let teclas = gtk::ShortcutController::new();
        for (tecla, passo) in [("Up", -1i32), ("Down", 1)] {
            let p = Rc::downgrade(self);
            let Some(trigger) = gtk::ShortcutTrigger::parse_string(tecla) else {
                continue;
            };
            teclas.add_shortcut(gtk::Shortcut::new(
                Some(trigger),
                Some(gtk::CallbackAction::new(move |_, _| {
                    if let Some(p) = p.upgrade() {
                        p.andar_no_historico(passo);
                    }
                    glib::Propagation::Stop
                })),
            ));
        }
        self.entrada.add_controller(teclas);
    }

    fn andar_no_historico(&self, passo: i32) {
        let h = self.historico.borrow();
        let novo = (self.no_historico.get() as i32 + passo).clamp(0, h.len() as i32) as usize;
        self.no_historico.set(novo);
        let texto = h.get(novo).map_or("", String::as_str);
        self.entrada.set_text(texto);
        self.entrada.set_position(-1);
    }
}

impl Drop for PainelRepl {
    fn drop(&mut self) {
        if let Some(fonte) = self.consulta.get_mut().take() {
            fonte.remove();
        }
        // Dropping CanalRepl cancels both evaluation and backpressured writes.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the GTK loop until `pronto` or a few seconds pass.
    fn girar_ate(pronto: impl Fn() -> bool) {
        let ctx = glib::MainContext::default();
        let fim = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !pronto() && std::time::Instant::now() < fim {
            ctx.iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(pronto(), "GTK REPL interaction did not complete");
    }

    fn texto(p: &PainelRepl) -> String {
        let b = p.saida.buffer();
        b.text(&b.start_iter(), &b.end_iter(), false).to_string()
    }

    fn digitar(p: &Rc<PainelRepl>, linha: &str) {
        p.entrada.set_text(linha);
        p.entrada.emit_activate();
    }

    fn esperando(p: &PainelRepl) -> usize {
        p.sessao.borrow().as_ref().map_or(0, CanalRepl::pendentes)
    }

    #[test]
    fn printing_streams_live_and_stop_then_next_command_work_under_saturation() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            // Large, newline-free chunks also exercise the character cap.
            let codigo = format!(
                "enquanto verdadeiro {{ escreva(\"{}\") }}",
                "ação".repeat(1024)
            );
            digitar(&p, &codigo);
            let ticks = Rc::new(Cell::new(0));
            let t = ticks.clone();
            let heartbeat = glib::timeout_add_local(Duration::from_millis(1), move || {
                t.set(t.get() + 1);
                glib::ControlFlow::Continue
            });
            girar_ate(|| p.saida.buffer().char_count() == MAX_CARACTERES_SAIDA);
            assert!(ticks.get() > 2, "output starved the GTK main loop");
            assert_eq!(esperando(&p), 1, "printing loop unexpectedly finished");
            assert!(texto(&p).contains("açãoação"));
            assert!(!texto(&p).contains('�'));
            assert!(p.btn_parar.is_sensitive());
            p.btn_parar.emit_clicked();
            girar_ate(|| esperando(&p) == 0);
            assert!(texto(&p).contains("interrompido"));
            assert!(!p.btn_parar.is_sensitive());
            digitar(&p, "2 + 2");
            girar_ate(|| esperando(&p) == 0);
            assert!(texto(&p).ends_with("4\n"));
            assert!(p.saida.buffer().char_count() <= MAX_CARACTERES_SAIDA);
            heartbeat.remove();
        });
    }

    #[test]
    fn repeated_restart_rejects_old_callbacks_and_does_not_mix_new_input_or_counts() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            for _ in 0..4 {
                let antiga = p.sessao.borrow().as_ref().unwrap().id();
                digitar(&p, "enquanto verdadeiro { escreva(\"ANTIGA\") }");
                girar_ate(|| texto(&p).ends_with("ANTIGA\n"));
                p.reiniciar();
                assert_ne!(p.sessao.borrow().as_ref().unwrap().id(), antiga);
                assert_eq!(esperando(&p), 0);
                digitar(&p, "x = 40");
                digitar(&p, "x + 2");
                assert_eq!(p.drenar(antiga), glib::ControlFlow::Break);
                assert_eq!(
                    esperando(&p),
                    2,
                    "stale callback changed new session counts"
                );
                girar_ate(|| esperando(&p) == 0);
                let texto = texto(&p);
                let nova = texto.rsplit("— sessão nova:").next().unwrap();
                assert!(nova.ends_with("42\n"), "{nova}");
                assert!(
                    !nova.contains("ANTIGA"),
                    "old output entered the new session"
                );
                assert!(p.consulta.borrow().is_some());
            }
            // An incomplete command and UTF-8 prefix also belong to their session.
            digitar(&p, "se verdadeiro {");
            p.stdout.borrow_mut().decodificar(&[0xc3]);
            p.reiniciar();
            assert!(p.pendente.borrow().is_empty());
            assert_eq!(p.prompt.text(), ">");
            digitar(&p, "x");
            girar_ate(|| esperando(&p) == 0);
            assert!(texto(&p).contains("não definida"));
            assert!(!texto(&p).contains('�'));
        });
    }

    #[test]
    fn scrollback_limits_keep_the_tail_and_stderr_stays_distinct() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            p.escrever(&format!("{}fim\n", "antigo\n".repeat(6000)), None);
            assert!(p.saida.buffer().line_count() <= 5000);
            assert!(texto(&p).ends_with("fim\n"));
            p.escrever(
                &format!("{}fim", "á".repeat(MAX_CARACTERES_SAIDA as usize)),
                None,
            );
            assert_eq!(p.saida.buffer().char_count(), MAX_CARACTERES_SAIDA);
            assert!(texto(&p).ends_with("fim"));
            digitar(&p, "escreva(\"normal\")");
            digitar(&p, "escreva_erro(\"atenção\")");
            girar_ate(|| esperando(&p) == 0);
            let b = p.saida.buffer();
            let erro = b.tag_table().lookup("erro").unwrap();
            let (normal, _) = b
                .start_iter()
                .forward_search("normal\n", gtk::TextSearchFlags::TEXT_ONLY, None)
                .unwrap();
            let (aviso, _) = b
                .start_iter()
                .forward_search("atenção\n", gtk::TextSearchFlags::TEXT_ONLY, None)
                .unwrap();
            assert!(!normal.has_tag(&erro));
            assert!(aviso.has_tag(&erro));
        });
    }

    #[test]
    fn quit_clears_pending_work_stops_polling_and_restart_accepts_input_again() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            digitar(&p, "saia(7)");
            digitar(&p, "enquanto verdadeiro { x = 1 }");
            girar_ate(|| p.consulta.borrow().is_none());
            assert_eq!(esperando(&p), 0);
            assert!(!p.entrada.is_sensitive());
            assert!(!p.btn_parar.is_sensitive());
            assert!(texto(&p).contains("código 7"));
            p.reiniciar();
            digitar(&p, "6 * 7");
            girar_ate(|| esperando(&p) == 0);
            assert!(texto(&p).ends_with("42\n"));
            assert!(p.entrada.is_sensitive());
            p.encerrar();
            assert!(p.consulta.borrow().is_none());
            assert!(p.sessao.borrow().is_none());
            let fraco = Rc::downgrade(&p);
            drop(p);
            assert!(fraco.upgrade().is_none(), "polling retained the panel");
        });
    }

    #[test]
    fn destroying_the_ide_window_cancels_the_repl_even_when_ui_state_is_retained() {
        crate::gtk_test::run(|| {
            let app = gtk::Application::builder()
                .application_id("dev.expressa.aula.tests.repl")
                .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
                .build();
            app.register(None::<&gtk::gio::Cancellable>).unwrap();
            let ui = crate::build_ui_state(&app, false);
            let p = ui.borrow().repl.clone();
            ui.borrow().window.present();
            girar_ate(|| ui.borrow().window.is_mapped());
            digitar(&p, "enquanto verdadeiro { escreva(\"ativo\") }");
            girar_ate(|| texto(&p).ends_with("ativo\n"));
            ui.borrow().window.destroy();
            assert!(p.sessao.borrow().is_none());
            assert!(p.consulta.borrow().is_none());
            assert!(!p.entrada.is_sensitive());
        });
    }

    #[test]
    fn dropping_an_active_panel_removes_its_polling_source() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            digitar(&p, "enquanto verdadeiro { escreva(\"ativo\") }");
            girar_ate(|| texto(&p).ends_with("ativo\n"));
            let fonte = glib::MainContext::default()
                .find_source_by_id(p.consulta.borrow().as_ref().unwrap())
                .unwrap();
            let fraco = Rc::downgrade(&p);
            drop(p);
            assert!(fraco.upgrade().is_none());
            assert!(fonte.is_destroyed());
        });
    }

    #[test]
    fn worker_disconnection_is_visible_and_restart_recovers() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            digitar(&p, "enquanto verdadeiro { escreva(\"ativo\") }");
            girar_ate(|| texto(&p).ends_with("ativo\n"));
            // Simulate a worker disappearing without delivering a terminal event.
            p.sessao.borrow_mut().as_mut().unwrap().encerrar();
            girar_ate(|| p.consulta.borrow().is_none());
            assert!(texto(&p).contains("desconectada; aperte Reiniciar"));
            assert_eq!(esperando(&p), 0);
            assert!(!p.entrada.is_sensitive());
            p.reiniciar();
            digitar(&p, "42");
            girar_ate(|| esperando(&p) == 0);
            assert!(texto(&p).ends_with("42\n"));
        });
    }

    #[test]
    fn repl_input_stays_above_status_bar_when_bottom_pane_is_collapsed() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            let notebook = gtk::Notebook::new();
            notebook.append_page(&p.widget, Some(&Label::new(Some("REPL"))));
            let editor = ScrolledWindow::builder().child(&TextView::new()).build();
            let pane = crate::editor_panes(&editor, &notebook);
            pane.set_vexpand(true);
            let status = Label::new(Some("status"));
            let root = GtkBox::new(Orientation::Vertical, 6);
            root.append(&pane);
            root.append(&status);
            let window = gtk::Window::builder()
                .default_width(800)
                .default_height(360)
                .child(&root)
                .build();
            window.present();
            girar_ate(|| p.entrada.is_mapped() && pane.height() > 0);
            pane.set_position(i32::MAX);
            // Process a frame after forcing the bottom pane to its smallest size.
            let allocated = Rc::new(Cell::new(false));
            let ready = allocated.clone();
            pane.add_tick_callback(move |_, _| {
                ready.set(true);
                glib::ControlFlow::Break
            });
            girar_ate(|| allocated.get());
            let input = p.entrada.compute_bounds(&window).unwrap();
            let bar = status.compute_bounds(&window).unwrap();
            let (_, natural, _, _) = p.entrada.measure(Orientation::Vertical, -1);
            let visible = input.y() >= 0.0
                && input.y() + input.height() <= bar.y()
                && input.height() >= natural as f32;
            window.close();
            assert!(
                visible,
                "REPL input {input:?} (natural height {natural}) overlaps status {bar:?}"
            );
        });
    }

    #[test]
    fn repl_no_painel() {
        crate::gtk_test::run(|| {
            let p = PainelRepl::new();
            digitar(&p, "x = 20");
            digitar(&p, "se x > 10 {");
            assert_eq!(p.prompt.text(), "...");
            digitar(&p, "  escreva(\"grande\")");
            digitar(&p, "}");
            digitar(&p, "x + 1");
            girar_ate(|| texto(&p).contains("21"));
            let t = texto(&p);
            assert!(t.contains("grande\n"), "{t}");
            assert!(t.contains("21\n"), "{t}");

            // An endless line, then Parar: the panel gets the error back.
            digitar(&p, "enquanto verdadeiro { x = 1 }");
            p.parar();
            girar_ate(|| texto(&p).contains("interrompido"));
            assert!(texto(&p).contains("interrompido"), "{}", texto(&p));

            // Reiniciar forgets the names.
            p.reiniciar();
            digitar(&p, "x");
            girar_ate(|| texto(&p).contains("não definida"));
            assert!(texto(&p).contains("não definida"), "{}", texto(&p));
        });
    }
}
