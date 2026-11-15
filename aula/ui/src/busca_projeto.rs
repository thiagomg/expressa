//! Busca tab of the bottom panel: find (and replace) in every file of the
//! project (Ctrl+Shift+F). Open tabs are searched as they are on screen,
//! saved or not; other files are read off the GTK thread.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, ListBox, Orientation, ScrolledWindow};
use regex::{Regex, RegexBuilder};

use crate::{UiRc, all_docs, em_segundo_plano, goto_file_line, set_status};

/// Stop listing after this many matches.
const ACHADOS_MAXIMO: usize = 2000;
/// Files bigger than this are not searched (data, not code).
const TAMANHO_MAXIMO: usize = 1 << 20;

/// What to look for, already compiled.
#[derive(Clone)]
pub struct Padrao {
    re: Regex,
    /// The replacement is literal unless the search is a regular expression
    /// (then `$1` means the first group).
    expandir: bool,
}

impl Padrao {
    pub fn new(
        consulta: &str,
        maiusculas: bool,
        palavra: bool,
        regex: bool,
    ) -> Result<Self, String> {
        if consulta.is_empty() {
            return Err("digite o que procurar".into());
        }
        let corpo = if regex {
            consulta.to_string()
        } else {
            regex::escape(consulta)
        };
        let corpo = if palavra {
            format!(r"\b(?:{corpo})\b")
        } else {
            corpo
        };
        let re = RegexBuilder::new(&corpo)
            .case_insensitive(!maiusculas)
            .build()
            .map_err(|e| format!("expressão regular inválida: {e}"))?;
        Ok(Self {
            re,
            expandir: regex,
        })
    }
}

/// One match: 1-based line, character columns on that line, and the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Achado {
    pub arquivo: String,
    pub linha: u32,
    pub col_ini: usize,
    pub col_fim: usize,
    pub texto: String,
}

/// Every match of `p` in `texto` (a match never spans lines).
pub fn procurar_em(arquivo: &str, texto: &str, p: &Padrao) -> Vec<Achado> {
    let mut out = Vec::new();
    for (i, linha) in texto.lines().enumerate() {
        for m in p.re.find_iter(linha) {
            if m.start() == m.end() {
                continue;
            }
            out.push(Achado {
                arquivo: arquivo.to_string(),
                linha: i as u32 + 1,
                col_ini: linha[..m.start()].chars().count(),
                col_fim: linha[..m.end()].chars().count(),
                texto: linha.to_string(),
            });
        }
    }
    out
}

/// `texto` with every match replaced, and how many there were.
/// A match stays inside one line, the same rule as [`procurar_em`].
pub fn trocar_em(texto: &str, p: &Padrao, por: &str) -> (String, usize) {
    let mut out = String::with_capacity(texto.len());
    let mut n = 0;
    let mut resto = texto;
    while !resto.is_empty() {
        let (linha, fim, proximo) = partir_linha(resto);
        let (nova, achadas) = trocar_na_linha(linha, p, por);
        n += achadas;
        out.push_str(&nova);
        out.push_str(fim);
        if proximo.is_empty() {
            break;
        }
        resto = proximo;
    }
    (out, n)
}

/// `(conteúdo, terminador, o que vem depois)`, com as mesmas quebras que
/// `str::lines`: só `\n`, e o `\r` de um `\r\n` sai do conteúdo. Um `\r`
/// sozinho continua no meio da linha.
fn partir_linha(texto: &str) -> (&str, &str, &str) {
    let Some(i) = texto.find('\n') else {
        return (texto, "", "");
    };
    if i > 0 && texto.as_bytes()[i - 1] == b'\r' {
        (&texto[..i - 1], &texto[i - 1..i + 1], &texto[i + 1..])
    } else {
        (&texto[..i], &texto[i..i + 1], &texto[i + 1..])
    }
}

fn trocar_na_linha(linha: &str, p: &Padrao, por: &str) -> (String, usize) {
    let mut out = String::with_capacity(linha.len());
    let mut ultimo = 0;
    let mut n = 0;
    for caps in p.re.captures_iter(linha) {
        let Some(m) = caps.get(0) else {
            continue;
        };
        // A busca também ignora casamento vazio. Trocar esses pontos faria
        // `a*` inserir o texto novo entre os caracteres da linha.
        if m.start() == m.end() {
            continue;
        }
        out.push_str(&linha[ultimo..m.start()]);
        if p.expandir {
            caps.expand(por, &mut out);
        } else {
            out.push_str(por);
        }
        ultimo = m.end();
        n += 1;
    }
    if n == 0 {
        return (linha.to_string(), 0);
    }
    out.push_str(&linha[ultimo..]);
    (out, n)
}

/// The search that Trocar is allowed to apply. A later edit of the query
/// does not change this copy.
#[derive(Clone)]
struct BuscaFeita {
    padrao: Padrao,
    consulta: String,
    maiusculas: bool,
    palavra: bool,
    regex: bool,
    arquivos: std::collections::BTreeSet<String>,
    cortado: bool,
}

pub struct PainelBusca {
    pub widget: GtkBox,
    consulta: Entry,
    troca: Entry,
    maiusculas: gtk::ToggleButton,
    palavra: gtk::ToggleButton,
    regex: gtk::ToggleButton,
    lista: ListBox,
    resumo: Label,
    trocar: Button,
    achados: RefCell<Vec<Achado>>,
    busca: RefCell<Option<BuscaFeita>>,
}

impl PainelBusca {
    pub fn new(ui: &UiRc) -> Rc<Self> {
        let consulta = Entry::builder()
            .placeholder_text("procurar em todos os arquivos")
            .hexpand(true)
            .build();
        let troca = Entry::builder()
            .placeholder_text("trocar por")
            .hexpand(true)
            .build();
        let opcao = |r: &str, dica: &str| {
            let b = gtk::ToggleButton::with_label(r);
            b.set_tooltip_text(Some(dica));
            b
        };
        let maiusculas = opcao("Aa", "Diferenciar maiúsculas de minúsculas");
        let palavra = opcao("Palavra", "Só a palavra inteira");
        let regex = opcao(".*", "Expressão regular (na troca, $1 é o primeiro grupo)");
        let buscar = Button::with_label("Buscar");
        let trocar = Button::with_label("Trocar em todos");
        trocar.set_sensitive(false);
        let linha1 = GtkBox::new(Orientation::Horizontal, 6);
        linha1.append(&consulta);
        linha1.append(&maiusculas);
        linha1.append(&palavra);
        linha1.append(&regex);
        linha1.append(&buscar);
        let linha2 = GtkBox::new(Orientation::Horizontal, 6);
        linha2.append(&troca);
        linha2.append(&trocar);
        let resumo = Label::new(None);
        resumo.set_xalign(0.0);
        let lista = ListBox::new();
        lista.set_selection_mode(gtk::SelectionMode::Single);
        let rolagem = ScrolledWindow::builder()
            .child(&lista)
            .vexpand(true)
            .hexpand(true)
            .build();
        let widget = GtkBox::new(Orientation::Vertical, 4);
        for w in [&linha1, &linha2] {
            w.set_margin_start(6);
            w.set_margin_end(6);
        }
        resumo.set_margin_start(6);
        widget.append(&linha1);
        widget.append(&linha2);
        widget.append(&resumo);
        widget.append(&rolagem);

        let painel = Rc::new(Self {
            widget,
            consulta,
            troca,
            maiusculas,
            palavra,
            regex,
            lista,
            resumo,
            trocar: trocar.clone(),
            achados: RefCell::new(Vec::new()),
            busca: RefCell::new(None),
        });
        let ligar = |b: &Button, f: fn(&UiRc, &Rc<PainelBusca>)| {
            let ui = Rc::downgrade(ui);
            let p = Rc::downgrade(&painel);
            b.connect_clicked(move |_| {
                if let (Some(ui), Some(p)) = (ui.upgrade(), p.upgrade()) {
                    f(&ui, &p);
                }
            });
        };
        ligar(&buscar, buscar_no_projeto);
        ligar(&trocar, trocar_no_projeto);
        {
            let ui = Rc::downgrade(ui);
            let p = Rc::downgrade(&painel);
            painel.consulta.connect_activate(move |_| {
                if let (Some(ui), Some(p)) = (ui.upgrade(), p.upgrade()) {
                    buscar_no_projeto(&ui, &p);
                }
            });
        }
        {
            let ui = Rc::downgrade(ui);
            let p = Rc::downgrade(&painel);
            painel.lista.connect_row_activated(move |_, row| {
                if let (Some(ui), Some(p)) = (ui.upgrade(), p.upgrade()) {
                    p.abrir(&ui, row.index() as usize);
                }
            });
        }
        {
            let atualizar = Rc::new({
                let p = Rc::downgrade(&painel);
                move || {
                    if let Some(p) = p.upgrade() {
                        p.atualizar_troca();
                    }
                }
            });
            {
                let atualizar = atualizar.clone();
                painel.consulta.connect_changed(move |_| atualizar());
            }
            for botao in [&painel.maiusculas, &painel.palavra, &painel.regex] {
                let atualizar = atualizar.clone();
                botao.connect_toggled(move |_| atualizar());
            }
        }
        painel
    }

    fn consulta_atual(&self) -> (String, bool, bool, bool) {
        (
            self.consulta.text().to_string(),
            self.maiusculas.is_active(),
            self.palavra.is_active(),
            self.regex.is_active(),
        )
    }

    fn registrar(
        &self,
        padrao: Padrao,
        consulta: String,
        maiusculas: bool,
        palavra: bool,
        regex: bool,
        achados: Vec<Achado>,
        arquivos: usize,
        cortado: bool,
    ) {
        let nomes = achados.iter().map(|a| a.arquivo.clone()).collect();
        *self.busca.borrow_mut() = Some(BuscaFeita {
            padrao,
            consulta,
            maiusculas,
            palavra,
            regex,
            arquivos: nomes,
            cortado,
        });
        self.mostrar(achados, arquivos, cortado);
        self.atualizar_troca();
    }

    fn atualizar_troca(&self) {
        self.trocar.set_sensitive(self.pode_trocar());
    }

    fn pode_trocar(&self) -> bool {
        let guard = self.busca.borrow();
        let Some(busca) = guard.as_ref() else {
            return false;
        };
        if busca.cortado || busca.arquivos.is_empty() {
            return false;
        }
        let (consulta, maiusculas, palavra, regex) = self.consulta_atual();
        busca.consulta == consulta
            && busca.maiusculas == maiusculas
            && busca.palavra == palavra
            && busca.regex == regex
    }

    /// The pattern and files from the last search, when Trocar is still valid.
    fn busca_para_troca(&self) -> Result<BuscaFeita, &'static str> {
        let Some(busca) = self.busca.borrow().clone() else {
            return Err("busque antes de trocar");
        };
        if busca.cortado {
            return Err("resultado cortado; refine a busca antes de trocar");
        }
        let (consulta, maiusculas, palavra, regex) = self.consulta_atual();
        let mesma = busca.consulta == consulta
            && busca.maiusculas == maiusculas
            && busca.palavra == palavra
            && busca.regex == regex;
        if !mesma {
            return Err("a busca mudou; busque de novo antes de trocar");
        }
        if busca.arquivos.is_empty() {
            return Err("busque antes de trocar");
        }
        Ok(busca)
    }

    /// Ctrl+Shift+F: focus the search, starting from the selected text.
    pub fn focar(&self, selecionado: Option<String>) {
        if let Some(s) = selecionado.filter(|s| !s.contains('\n') && !s.is_empty()) {
            self.consulta.set_text(&s);
        }
        self.consulta.grab_focus();
    }

    fn mostrar(&self, achados: Vec<Achado>, arquivos: usize, cortado: bool) {
        while let Some(row) = self.lista.first_child() {
            self.lista.remove(&row);
        }
        for a in &achados {
            let linha = a.texto.trim();
            let curta: String = linha.chars().take(160).collect();
            let rotulo = Label::new(Some(&format!("{}:{}  {curta}", a.arquivo, a.linha)));
            rotulo.set_xalign(0.0);
            rotulo.add_css_class("monospace");
            rotulo.set_ellipsize(gtk::pango::EllipsizeMode::End);
            self.lista.append(&rotulo);
        }
        let n = achados.len();
        self.resumo.set_text(&match n {
            0 => "nada encontrado".to_string(),
            _ if cortado => format!("primeiros {n} resultados (há mais; refine a busca)"),
            _ => format!("{n} resultado(s) em {arquivos} arquivo(s)"),
        });
        *self.achados.borrow_mut() = achados;
    }

    fn abrir(&self, ui: &UiRc, i: usize) {
        let Some(a) = self.achados.borrow().get(i).cloned() else {
            return;
        };
        goto_file_line(ui, &a.arquivo, a.linha);
        let Some(doc) = crate::doc_by_name(ui, &a.arquivo) else {
            return;
        };
        let linha = a.linha as i32 - 1;
        if let (Some(ini), Some(fim)) = (
            doc.buffer.iter_at_line_offset(linha, a.col_ini as i32),
            doc.buffer.iter_at_line_offset(linha, a.col_fim as i32),
        ) {
            doc.buffer.select_range(&ini, &fim);
        }
    }
}

/// What the worker needs: the texts of open tabs, and the files to read.
fn juntar(
    ui: &UiRc,
) -> (
    Vec<(String, String)>,
    Vec<String>,
    crate::store::Files,
    String,
) {
    let abertos: Vec<(String, String)> = all_docs(ui)
        .iter()
        .filter(|d| !d.is_untitled())
        .map(|d| (d.name(), d.text()))
        .collect();
    let u = ui.borrow();
    let outros = u
        .project_files
        .iter()
        .filter(|f| !abertos.iter().any(|(n, _)| n == *f))
        .cloned()
        .collect();
    (abertos, outros, u.files.clone(), u.project.clone())
}

fn buscar_no_projeto(ui: &UiRc, painel: &Rc<PainelBusca>) {
    let (consulta, maiusculas, palavra, regex) = painel.consulta_atual();
    let padrao = match Padrao::new(&consulta, maiusculas, palavra, regex) {
        Ok(p) => p,
        Err(e) => {
            painel.resumo.set_text(&e);
            return;
        }
    };
    let (abertos, outros, files, projeto) = juntar(ui);
    painel.busca.replace(None);
    painel.trocar.set_sensitive(false);
    painel.resumo.set_text("procurando…");
    let painel = Rc::clone(painel);
    let padrao_ui = padrao.clone();
    em_segundo_plano(
        ui,
        move || {
            let mut achados = Vec::new();
            let mut arquivos = 0;
            let textos = abertos.into_iter().map(Ok).chain(
                outros
                    .into_iter()
                    .map(|f| files.read_file(&projeto, &f).map(|t| (f, t))),
            );
            // Binary or unreadable files are skipped.
            for (nome, texto) in textos.flatten() {
                if texto.len() > TAMANHO_MAXIMO {
                    continue;
                }
                let novos = procurar_em(&nome, &texto, &padrao);
                if !novos.is_empty() {
                    arquivos += 1;
                }
                achados.extend(novos);
                if achados.len() >= ACHADOS_MAXIMO {
                    achados.truncate(ACHADOS_MAXIMO);
                    return (achados, arquivos, true);
                }
            }
            (achados, arquivos, false)
        },
        move |_, (achados, arquivos_n, cortado)| {
            painel.registrar(
                padrao_ui, consulta, maiusculas, palavra, regex, achados, arquivos_n, cortado,
            );
        },
    );
}

fn trocar_no_projeto(ui: &UiRc, painel: &Rc<PainelBusca>) {
    let busca = match painel.busca_para_troca() {
        Ok(b) => b,
        Err(e) => {
            painel.resumo.set_text(e);
            return;
        }
    };
    let por = painel.troca.text().to_string();
    let quantos = painel.achados.borrow().len();
    let n_arquivos = busca.arquivos.len();
    let dialogo = gtk::AlertDialog::builder()
        .modal(true)
        .message(format!(
            "Trocar {quantos} ocorrência(s) em {n_arquivos} arquivo(s)?",
        ))
        .detail(
            "Os arquivos fechados são gravados agora. Os abertos mudam no editor \
             e ficam para você salvar (Ctrl+Z desfaz).",
        )
        .buttons(["Cancelar", "Trocar"])
        .cancel_button(0)
        .default_button(1)
        .build();
    let window = ui.borrow().window.clone();
    let ui = Rc::clone(ui);
    let painel = Rc::clone(painel);
    dialogo.choose(Some(&window), None::<&gtk::gio::Cancellable>, move |r| {
        if r == Ok(1) {
            aplicar_troca(&ui, &painel, busca, por);
        }
    });
}

fn aplicar_troca(ui: &UiRc, painel: &Rc<PainelBusca>, busca: BuscaFeita, por: String) {
    // Open tabs that the search listed: one undo step each, not saved yet.
    // The path stays out of the disk write even when this buffer no longer
    // matches, or grew past the search limit, so the tab is not overwritten.
    let mut nos_abertos = 0;
    let mut abertos = std::collections::BTreeSet::new();
    for d in all_docs(ui) {
        let Some(caminho) = d.path.borrow().clone() else {
            continue;
        };
        if !busca.arquivos.contains(&caminho) {
            continue;
        }
        abertos.insert(caminho);
        let texto = d.text();
        if texto.len() > TAMANHO_MAXIMO {
            continue;
        }
        let (novo, n) = trocar_em(&texto, &busca.padrao, &por);
        if n == 0 {
            continue;
        }
        nos_abertos += n;
        let linha = d.cursor().line();
        d.buffer.begin_user_action();
        d.buffer.set_text(&novo);
        d.buffer.end_user_action();
        if let Some(it) = d.buffer.iter_at_line(linha) {
            d.buffer.place_cursor(&it);
        }
    }
    let fechados: Vec<String> = busca
        .arquivos
        .iter()
        .filter(|f| !abertos.contains(*f))
        .cloned()
        .collect();
    let (_, _, files, projeto) = juntar(ui);
    let p = busca.padrao;
    let painel = Rc::clone(painel);
    em_segundo_plano(
        ui,
        move || {
            let mut trocas = 0;
            let mut erros = Vec::new();
            let mut avisos = Vec::new();
            for f in fechados {
                let snapshot = match files.current_text(&projeto, &f) {
                    Ok(Some(snapshot)) => snapshot,
                    Ok(None) => continue,
                    Err(e) if erro_de_utf8(&e) => continue,
                    Err(e) => {
                        erros.push(format!("{f}: {e}"));
                        continue;
                    }
                };
                if snapshot.text.len() > TAMANHO_MAXIMO {
                    continue;
                }
                let (novo, n) = trocar_em(&snapshot.text, &p, &por);
                if n == 0 {
                    continue;
                }
                let expected = snapshot
                    .revision
                    .map(crate::store::Expected::Revision)
                    .unwrap_or(crate::store::Expected::Unversioned);
                match files.write_file(&projeto, &f, &novo, &expected) {
                    Ok(saved) => {
                        trocas += n;
                        if let Some(warning) = saved.warning {
                            avisos.push(format!("{f}: {warning}"));
                        }
                    }
                    Err(e) => erros.push(format!("{f}: {e}")),
                }
            }
            (trocas, erros, avisos)
        },
        move |ui, (trocas, erros, avisos)| {
            let mut msg = format!("trocou {} ocorrência(s)", trocas + nos_abertos);
            if nos_abertos > 0 {
                msg.push_str(" — salve as abas alteradas");
            }
            if !erros.is_empty() {
                msg.push_str(&format!("; não gravou: {}", erros.join(", ")));
            }
            if !avisos.is_empty() {
                msg.push_str(&format!("; avisos: {}", avisos.join(", ")));
            }
            set_status(ui, &msg);
            ui.borrow().file_cache.borrow_mut().clear();
            buscar_no_projeto(ui, &painel);
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(q: &str, aa: bool, palavra: bool, re: bool) -> Padrao {
        Padrao::new(q, aa, palavra, re).unwrap()
    }

    #[test]
    fn acha_com_linha_e_colunas_em_caracteres() {
        let t = "nota = 1\nescreva(\"média\", nota)\n";
        let a = procurar_em("a.lep", t, &p("nota", false, false, false));
        assert_eq!(a.len(), 2);
        assert_eq!((a[1].linha, a[1].col_ini, a[1].col_fim), (2, 17, 21));
    }

    #[test]
    fn opcoes_maiusculas_palavra_e_regex() {
        let t = "Soma = soma + somatorio\n";
        assert_eq!(
            procurar_em("a", t, &p("soma", false, false, false)).len(),
            3
        );
        assert_eq!(procurar_em("a", t, &p("soma", true, false, false)).len(), 2);
        assert_eq!(procurar_em("a", t, &p("soma", false, true, false)).len(), 2);
        assert_eq!(
            procurar_em("a", t, &p("som[a-z]+", false, false, true)).len(),
            3
        );
        // Without regex, special characters are literal.
        assert_eq!(
            procurar_em("a", "x.y xzy", &p(".", false, false, false)).len(),
            1
        );
        assert!(Padrao::new("(", false, false, true).is_err());
        assert!(Padrao::new("", false, false, false).is_err());
    }

    #[test]
    fn troca_literal_ou_com_grupos() {
        let (t, n) = trocar_em("a = $1 + a", &p("a", true, true, false), "b$1");
        assert_eq!((t.as_str(), n), ("b$1 = $1 + b$1", 2));
        let (t, n) = trocar_em("nota1 nota2", &p(r"nota(\d)", true, false, true), "n$1");
        assert_eq!((t.as_str(), n), ("n1 n2", 2));
        let (t, n) = trocar_em("nada", &p("x", true, false, false), "y");
        assert_eq!((t.as_str(), n), ("nada", 0));
    }

    #[test]
    fn troca_respeita_as_linhas_como_a_busca() {
        let texto = "x\nx\n";
        let ancora = p("^x", true, false, true);
        assert_eq!(procurar_em("a.lep", texto, &ancora).len(), 2);
        let (t, n) = trocar_em(texto, &ancora, "y");
        assert_eq!((t.as_str(), n), ("y\ny\n", 2));

        let texto = "a \n b";
        let espacos = p(r"\s+", true, false, true);
        assert_eq!(procurar_em("a.lep", texto, &espacos).len(), 2);
        let (t, n) = trocar_em(texto, &espacos, "_");
        assert_eq!((t.as_str(), n), ("a_\n_b", 2));

        let (t, n) = trocar_em("a\r\nb\r\n", &p("^.", true, false, true), "z");
        assert_eq!((t.as_str(), n), ("z\r\nz\r\n", 2));

        let texto = "b\rb";
        let ancora = p("^b", true, false, true);
        assert_eq!(procurar_em("a.lep", texto, &ancora).len(), 1);
        let (t, n) = trocar_em(texto, &ancora, "Q");
        assert_eq!((t.as_str(), n), ("Q\rb", 1));

        let texto = "a\rb";
        let cr = p("\r", true, false, true);
        assert_eq!(procurar_em("a.lep", texto, &cr).len(), 1);
        let (t, n) = trocar_em(texto, &cr, "_");
        assert_eq!((t.as_str(), n), ("a_b", 1));
    }

    #[test]
    fn troca_ignora_casamento_vazio() {
        let estrela = p("a*", true, false, true);
        assert_eq!(procurar_em("a.lep", "ab", &estrela).len(), 1);
        let (t, n) = trocar_em("ab", &estrela, "Q");
        assert_eq!((t.as_str(), n), ("Qb", 1));
        let vazio = p("x*", true, false, true);
        assert!(procurar_em("a.lep", "ab", &vazio).is_empty());
        let (t, n) = trocar_em("ab", &vazio, "Q");
        assert_eq!((t.as_str(), n), ("ab", 0));
    }
}

fn erro_de_utf8(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("utf-8") || m.contains("utf8")
}
