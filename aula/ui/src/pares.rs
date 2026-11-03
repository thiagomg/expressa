//! Closing pairs while typing: `(` gives `()`, `[` `[]`, `{` `{}`, `"` `""`,
//! with the cursor in the middle. Typing the closer that is already there
//! steps over it, Backspace on an empty pair removes both, and Enter between
//! `{` and `}` opens an indented line.
//!
//! Works on what the buffer received (`insert-text`), not on keys: on
//! Brazilian keyboards `"` is often a dead key, and a key handler would break
//! accents. The edits wait for the end of the user action, so the electric
//! indent (`indent.rs`) has already run and no iterator is in use.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use sourceview5::Buffer as SourceBuffer;

/// What to do after the user typed one character.
#[derive(Debug, PartialEq, Eq)]
pub enum Acao {
    Nada,
    /// Insert this closer after the cursor.
    Fechar(char),
    /// The next character is the one just typed: remove it (step over).
    Pular,
}

#[derive(Debug, PartialEq, Eq)]
enum Onde {
    Codigo,
    Texto,
    Comentario,
}

/// Where the end of `antes` (a line, up to the cursor) is: code, inside a
/// string, or inside a comment. One line only, as the editor sees it.
fn onde(antes: &str) -> Onde {
    let mut chars = antes.chars().peekable();
    let mut texto = false;
    let mut bloco = false;
    while let Some(c) = chars.next() {
        if bloco {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                bloco = false;
            }
        } else if texto {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => texto = false,
                _ => {}
            }
        } else {
            match c {
                '"' => texto = true,
                '/' if chars.peek() == Some(&'/') => return Onde::Comentario,
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    bloco = true;
                }
                _ => {}
            }
        }
    }
    if bloco {
        Onde::Comentario
    } else if texto {
        Onde::Texto
    } else {
        Onde::Codigo
    }
}

fn fechamento(abre: char) -> Option<char> {
    match abre {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        _ => None,
    }
}

/// The user typed `c`; `antes` is the line before it and `depois` the line
/// after the cursor.
pub fn ao_digitar(antes: &str, c: char, depois: &str) -> Acao {
    let proximo = depois.chars().next();
    // Pair only where nothing is glued after the cursor: `f(|x` stays `f(x`.
    let livre = proximo.is_none_or(|p| p.is_whitespace() || ")]},".contains(p));
    match (onde(antes), c) {
        (Onde::Comentario, _) => Acao::Nada,
        (Onde::Codigo, ')' | ']' | '}') if proximo == Some(c) => Acao::Pular,
        (Onde::Codigo, '(' | '[' | '{') if livre => Acao::Fechar(fechamento(c).unwrap()),
        // This `"` closes a string: step over the one paired when it opened.
        (Onde::Texto, '"') if proximo == Some('"') => Acao::Pular,
        (Onde::Codigo, '"') => {
            let colado = antes
                .chars()
                .last()
                .is_some_and(|a| a.is_alphanumeric() || a == '_');
            if livre && !colado {
                Acao::Fechar('"')
            } else {
                Acao::Nada
            }
        }
        _ => Acao::Nada,
    }
}

/// The user deleted `apagado`, which was right before the cursor: true
/// when the character after the cursor is its pair (`(|)`), to go too.
pub fn apagar_par(antes: &str, apagado: char, depois: &str) -> bool {
    onde(antes) == Onde::Codigo && fechamento(apagado).is_some_and(|f| depois.starts_with(f))
}

/// Enter was typed between an opener and its closer (`{|}`): the closer
/// should go one line further down.
pub fn abrir_bloco(linha_de_cima: &str, depois: &str) -> bool {
    let Some(abre) = linha_de_cima.trim_end().chars().last() else {
        return false;
    };
    matches!(abre, '(' | '[' | '{')
        && onde(linha_de_cima) == Onde::Codigo
        && depois.trim_start().chars().next() == fechamento(abre)
}

/// One change seen during a user action, handled when it ends.
enum Mudanca {
    Digitou(char),
    Apagou(char),
    Enter,
}

/// Turns the pairs on for `buffer`.
pub fn attach(buffer: &SourceBuffer) {
    let profundidade = Rc::new(Cell::new(0u32));
    let mudancas = Rc::new(Cell::new(0u32));
    let mudanca: Rc<RefCell<Option<Mudanca>>> = Rc::new(RefCell::new(None));
    // Our own edits are user actions too: do not react to them.
    let aplicando = Rc::new(Cell::new(false));
    {
        let (p, n, m) = (
            Rc::clone(&profundidade),
            Rc::clone(&mudancas),
            Rc::clone(&mudanca),
        );
        buffer.connect_begin_user_action(move |_| {
            p.set(p.get() + 1);
            if p.get() == 1 {
                n.set(0);
                m.replace(None);
            }
        });
    }
    {
        let (p, n, m, a) = (
            Rc::clone(&profundidade),
            Rc::clone(&mudancas),
            Rc::clone(&mudanca),
            Rc::clone(&aplicando),
        );
        buffer.connect_local("insert-text", true, move |args| {
            if p.get() == 0 || a.get() {
                return None;
            }
            n.set(n.get() + 1);
            let text = args[2].get::<String>().ok()?;
            let mut chars = text.chars();
            let nova = match (chars.next(), chars.next()) {
                (Some('\n'), None) => Some(Mudanca::Enter),
                (Some(c), None) => Some(Mudanca::Digitou(c)),
                _ => None,
            };
            m.replace(nova);
            None
        });
    }
    {
        let (p, n, m, a) = (
            Rc::clone(&profundidade),
            Rc::clone(&mudancas),
            Rc::clone(&mudanca),
            Rc::clone(&aplicando),
        );
        // Before the default handler: the deleted text is still there.
        buffer.connect_local("delete-range", false, move |args| {
            if p.get() == 0 || a.get() {
                return None;
            }
            n.set(n.get() + 1);
            let b = args[0].get::<SourceBuffer>().ok()?;
            let start = args[1].get::<gtk::TextIter>().ok()?;
            let end = args[2].get::<gtk::TextIter>().ok()?;
            let apagado = b.text(&start, &end, false);
            let mut chars = apagado.chars();
            let nova = match (chars.next(), chars.next()) {
                (Some(c), None) => Some(Mudanca::Apagou(c)),
                _ => None,
            };
            m.replace(nova);
            None
        });
    }
    buffer.connect_end_user_action(move |b| {
        profundidade.set(profundidade.get().saturating_sub(1));
        if profundidade.get() > 0 || aplicando.get() {
            return;
        }
        // One keystroke is one change; a paste or a replace is more.
        if mudancas.get() != 1 {
            return;
        }
        let Some(m) = mudanca.take() else { return };
        aplicando.set(true);
        aplicar(b, m);
        aplicando.set(false);
    });
}

/// Text of the cursor's line before and after the cursor.
fn em_volta(b: &SourceBuffer) -> (String, String) {
    let cursor = b.iter_at_mark(&b.get_insert());
    let mut inicio = cursor;
    inicio.set_line_offset(0);
    let mut fim = cursor;
    if !fim.ends_line() {
        fim.forward_to_line_end();
    }
    (
        b.text(&inicio, &cursor, false).to_string(),
        b.text(&cursor, &fim, false).to_string(),
    )
}

fn aplicar(b: &SourceBuffer, m: Mudanca) {
    let (antes, depois) = em_volta(b);
    match m {
        Mudanca::Digitou(c) => {
            // `antes` ends with the typed character.
            let Some(sem_c) = antes.strip_suffix(c) else {
                return;
            };
            match ao_digitar(sem_c, c, &depois) {
                Acao::Nada => {}
                Acao::Fechar(f) => {
                    b.begin_user_action();
                    let mut cursor = b.iter_at_mark(&b.get_insert());
                    let offset = cursor.offset();
                    b.insert(&mut cursor, &f.to_string());
                    b.place_cursor(&b.iter_at_offset(offset));
                    b.end_user_action();
                }
                Acao::Pular => apagar_depois_do_cursor(b),
            }
        }
        Mudanca::Apagou(c) => {
            if apagar_par(&antes, c, &depois) {
                apagar_depois_do_cursor(b);
            }
        }
        Mudanca::Enter => {
            let cursor = b.iter_at_mark(&b.get_insert());
            let linha = cursor.line();
            if linha == 0 {
                return;
            }
            let mut ini = b.iter_at_line(linha - 1).unwrap_or(cursor);
            ini.set_line_offset(0);
            let mut fim = ini;
            if !fim.ends_line() {
                fim.forward_to_line_end();
            }
            let de_cima = b.text(&ini, &fim, false).to_string();
            if !abrir_bloco(&de_cima, &depois) {
                return;
            }
            // `{` / `|}` becomes `{` / `|` / `}`: the closer goes down, the
            // cursor stays on an indented empty line.
            b.begin_user_action();
            let mut c = b.iter_at_mark(&b.get_insert());
            b.insert(&mut c, "\n");
            crate::indent::indent_line(b, linha + 1);
            crate::indent::indent_line(b, linha);
            let mut fim_meio = b.iter_at_line(linha).unwrap_or(c);
            if !fim_meio.ends_line() {
                fim_meio.forward_to_line_end();
            }
            b.place_cursor(&fim_meio);
            b.end_user_action();
        }
    }
}

fn apagar_depois_do_cursor(b: &SourceBuffer) {
    b.begin_user_action();
    let mut ini = b.iter_at_mark(&b.get_insert());
    let mut fim = ini;
    fim.forward_char();
    b.delete(&mut ini, &mut fim);
    b.end_user_action();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fecha_parenteses_colchete_chave_e_aspas_no_codigo() {
        assert_eq!(ao_digitar("escreva", '(', ""), Acao::Fechar(')'));
        assert_eq!(ao_digitar("x = ", '[', ""), Acao::Fechar(']'));
        assert_eq!(ao_digitar("se x ", '{', ""), Acao::Fechar('}'));
        assert_eq!(ao_digitar("escreva(", '"', ")"), Acao::Fechar('"'));
        // Before a closer or a space it is fine too.
        assert_eq!(ao_digitar("f(", '(', ")"), Acao::Fechar(')'));
    }

    #[test]
    fn nao_fecha_colado_em_palavra_nem_em_texto_ou_comentario() {
        assert_eq!(ao_digitar("f", '(', "x)"), Acao::Nada);
        assert_eq!(ao_digitar("x = \"abc ", '(', ""), Acao::Nada);
        assert_eq!(ao_digitar("// veja ", '(', ""), Acao::Nada);
        assert_eq!(ao_digitar("/* nota ", '"', ""), Acao::Nada);
        // `"` glued to a word is not a new string: `diz"`.
        assert_eq!(ao_digitar("diz", '"', ""), Acao::Nada);
        // After a block comment closes, it is code again.
        assert_eq!(ao_digitar("/* a */ f", '(', ""), Acao::Fechar(')'));
    }

    #[test]
    fn pula_o_fechamento_que_ja_esta_la() {
        assert_eq!(ao_digitar("f(x", ')', ")"), Acao::Pular);
        assert_eq!(ao_digitar("[1, 2", ']', "]"), Acao::Pular);
        // Closing a string with its pair right after.
        assert_eq!(ao_digitar("escreva(\"oi", '"', "\")"), Acao::Pular);
        // A different closer is typed normally.
        assert_eq!(ao_digitar("f(x", ']', ")"), Acao::Nada);
        // An escaped quote does not close the string.
        assert_eq!(onde("\"a\\\""), Onde::Texto);
    }

    #[test]
    fn backspace_apaga_o_par_vazio() {
        assert!(apagar_par("f", '(', ")"));
        assert!(apagar_par("x = ", '"', "\""));
        assert!(!apagar_par("f", '(', "x)"));
        assert!(!apagar_par("// f", '(', ")"));
    }

    #[test]
    fn enter_entre_chaves_abre_bloco() {
        assert!(abrir_bloco("se x > 1 {", "}"));
        assert!(abrir_bloco("xs = [", "    ]"));
        assert!(!abrir_bloco("se x > 1 {", ""));
        assert!(!abrir_bloco("// {", "}"));
        assert!(!abrir_bloco("x = 1", "}"));
    }

    fn buffer() -> Option<SourceBuffer> {
        let b = SourceBuffer::new(None::<&gtk::TextTagTable>);
        crate::indent::attach_electric(&b);
        attach(&b);
        Some(b)
    }

    fn digitar(b: &SourceBuffer, texto: &str) {
        for c in texto.chars() {
            b.begin_user_action();
            b.insert_at_cursor(&c.to_string());
            b.end_user_action();
        }
    }

    fn backspace(b: &SourceBuffer) {
        b.begin_user_action();
        let mut cursor = b.iter_at_mark(&b.get_insert());
        b.backspace(&mut cursor, true, true);
        b.end_user_action();
    }

    fn com_cursor(b: &SourceBuffer) -> String {
        let (s, e) = b.bounds();
        let mut t = b.text(&s, &e, false).to_string();
        let at = b.iter_at_mark(&b.get_insert()).offset() as usize;
        let i = t.char_indices().nth(at).map_or(t.len(), |(i, _)| i);
        t.insert(i, '|');
        t
    }

    #[test]
    fn no_editor() {
        crate::gtk_test::run(|| {
            let Some(b) = buffer() else { return };
            digitar(&b, "escreva(\"oi");
            assert_eq!(com_cursor(&b), "escreva(\"oi|\")");
            digitar(&b, "\")");
            assert_eq!(com_cursor(&b), "escreva(\"oi\")|");

            b.set_text("");
            digitar(&b, "f(");
            backspace(&b);
            assert_eq!(com_cursor(&b), "f|");

            b.set_text("");
            digitar(&b, "se x {\n");
            assert_eq!(com_cursor(&b), "se x {\n    |\n}");

            // A paste is left alone.
            b.set_text("");
            b.begin_user_action();
            b.insert_at_cursor("f(");
            b.end_user_action();
            assert_eq!(com_cursor(&b), "f(|");
        });
    }
}
