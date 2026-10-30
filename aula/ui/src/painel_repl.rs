//! REPL tab of the bottom panel: try an expression without making a file.
//! The interpreter runs on its own thread (`repl_por_canal`), so a slow line
//! never freezes the window, and Parar interrupts it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use expressa::runtime::{RespostaRepl, repl_por_canal};
use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, Orientation, ScrolledWindow, TextView};

/// One interpreter session on its thread.
struct Sessao {
    linhas: Sender<String>,
    respostas: Receiver<RespostaRepl>,
    parar: Arc<AtomicBool>,
}

impl Sessao {
    fn nova() -> Self {
        let (linhas, rx) = std::sync::mpsc::channel();
        let (tx, respostas) = std::sync::mpsc::channel();
        let parar = Arc::new(AtomicBool::new(false));
        let p = Arc::clone(&parar);
        std::thread::Builder::new()
            .name("repl".into())
            .spawn(move || repl_por_canal(rx, tx, p))
            .expect("thread do REPL");
        Self {
            linhas,
            respostas,
            parar,
        }
    }
}

pub struct PainelRepl {
    pub widget: GtkBox,
    saida: TextView,
    entrada: Entry,
    prompt: Label,
    sessao: RefCell<Sessao>,
    /// Lines typed so far of code that is not complete yet (`se x {`).
    pendente: RefCell<String>,
    /// Pieces sent and not answered yet.
    esperando: Cell<u32>,
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
        let parar = Button::with_label("Parar");
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
            sessao: RefCell::new(Sessao::nova()),
            pendente: RefCell::new(String::new()),
            esperando: Cell::new(0),
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
        let mut fim = buffer.end_iter();
        self.saida.scroll_to_iter(&mut fim, 0.0, false, 0.0, 0.0);
    }

    fn enviar(self: &Rc<Self>) {
        let linha = self.entrada.text().to_string();
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
        if self.sessao.borrow().linhas.send(codigo).is_err() {
            self.escrever("a sessão parou; aperte Reiniciar\n", Some("erro"));
            return;
        }
        self.esperando.set(self.esperando.get() + 1);
        if self.esperando.get() == 1 {
            self.aguardar();
        }
    }

    /// Polls for answers while some are pending (the session is on another
    /// thread with a plain channel).
    fn aguardar(self: &Rc<Self>) {
        let p = Rc::downgrade(self);
        glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
            let Some(p) = p.upgrade() else {
                return glib::ControlFlow::Break;
            };
            loop {
                let resposta = p.sessao.borrow().respostas.try_recv();
                match resposta {
                    Ok(r) => {
                        p.mostrar(r);
                        p.esperando.set(p.esperando.get().saturating_sub(1));
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        p.esperando.set(0);
                        break;
                    }
                }
            }
            if p.esperando.get() == 0 {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    fn mostrar(&self, r: RespostaRepl) {
        if !r.saida.is_empty() {
            self.escrever(&r.saida, None);
        }
        match r.resultado {
            Ok(Some(valor)) => self.escrever(&format!("{valor}\n"), None),
            Ok(None) => {}
            Err(e) => self.escrever(&format!("{e}\n"), Some("erro")),
        }
    }

    /// Interrupts the line running now. Nothing running: nothing to stop
    /// (and the next line must not be stopped by it).
    fn parar(&self) {
        if self.esperando.get() > 0 {
            self.sessao.borrow().parar.store(true, Ordering::SeqCst);
        }
    }

    fn reiniciar(&self) {
        // The old thread ends once its line stops and its channel closes.
        self.sessao.borrow().parar.store(true, Ordering::SeqCst);
        *self.sessao.borrow_mut() = Sessao::nova();
        self.pendente.borrow_mut().clear();
        self.esperando.set(0);
        self.prompt.set_text(">");
        self.escrever(
            "— sessão nova: as variáveis foram esquecidas —\n",
            Some("eco"),
        );
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
    }

    fn texto(p: &PainelRepl) -> String {
        let b = p.saida.buffer();
        b.text(&b.start_iter(), &b.end_iter(), false).to_string()
    }

    fn digitar(p: &Rc<PainelRepl>, linha: &str) {
        p.entrada.set_text(linha);
        p.enviar();
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
