//! Electric indent, comment, duplicate and search applied to the Freya buffer.

use freya::code_editor::CodeEditorData;
use freya::prelude::*;
use freya::text_edit::TextEditor;

use crate::busca::{anterior, proxima};
use crate::editor::{
    byte_do_cursor, linhas_selecionadas, posicao_utf16, substituir_texto,
};
use crate::indent::{
    comentar_linhas, duplicar_linha, indentar_linha, indentar_todo, mover_linha, recuar_bloco,
    starts_with_closer,
};

pub fn recuar(editor: &mut CodeEditorData, delta: i32) {
    let (a, b) = linhas_selecionadas(editor);
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = recuar_bloco(&editor.rope.to_string(), a, b, delta);
    substituir_texto(editor, &novo);
    editor.move_cursor_to_row(row, col.saturating_add_signed(delta as isize));
}

pub fn comentar(editor: &mut CodeEditorData) {
    let (a, b) = linhas_selecionadas(editor);
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = comentar_linhas(&editor.rope.to_string(), a, b);
    substituir_texto(editor, &novo);
    editor.move_cursor_to_row(row, col);
}

pub fn duplicar(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = duplicar_linha(&editor.rope.to_string(), row);
    substituir_texto(editor, &novo);
    editor.move_cursor_to_row(row + 1, col);
}

pub fn mover(editor: &mut CodeEditorData, delta: i32) {
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let (novo, dest) = mover_linha(&editor.rope.to_string(), row, delta);
    substituir_texto(editor, &novo);
    editor.move_cursor_to_row(dest, col);
}

pub fn recuar_arquivo(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = indentar_todo(&editor.rope.to_string());
    substituir_texto(editor, &novo);
    editor.move_cursor_to_row(row, col);
}

pub fn indentar_atual(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let Some(novo) = indentar_linha(&editor.rope.to_string(), row) else {
        return;
    };
    substituir_texto(editor, &novo);
    let linha = novo.split('\n').nth(row).unwrap_or("");
    let col = linha.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    editor.move_cursor_to_row(row, col);
}

/// Handle Tab. Other keys stay with the editor so IME/compose can finish.
/// `true` lets the editor keep the event.
pub fn tratar_tecla(e: &Event<KeyboardEventData>, editor: &mut CodeEditorData) -> bool {
    match &e.key {
        Key::Named(NamedKey::Tab) if e.modifiers.contains(Modifiers::SHIFT) => {
            e.stop_propagation();
            e.prevent_default();
            recuar(editor, -4);
            false
        }
        Key::Named(NamedKey::Tab) => {
            e.stop_propagation();
            e.prevent_default();
            recuar(editor, 4);
            false
        }
        _ => true,
    }
}

/// Enter, `}` and `m` indent *after* the editor has inserted the character.
pub fn tecla_pede_indent(key: &Key, modifiers: Modifiers) -> bool {
    match key {
        Key::Named(NamedKey::Enter) if !modifiers.contains(Modifiers::CONTROL) => true,
        Key::Character(c) if c == "}" || c == "m" => true,
        _ => false,
    }
}

/// Recede a closer or indent a blank line opened by Enter. No-op otherwise.
pub fn reindentar_depois(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let texto = editor.rope.to_string();
    let atual = texto.split('\n').nth(row).unwrap_or("");
    if starts_with_closer(atual) || atual.chars().all(|c| c == ' ' || c == '\t') {
        indentar_atual(editor);
    }
}

pub fn buscar(editor: &mut CodeEditorData, termo: &str, para_frente: bool) -> Option<String> {
    if termo.is_empty() {
        return Some("Digite o que buscar".into());
    }
    let texto = editor.rope.to_string();
    let de = byte_do_cursor(editor);
    let achado = if para_frente {
        proxima(&texto, termo, de.saturating_add(1)).or_else(|| proxima(&texto, termo, 0))
    } else {
        anterior(&texto, termo, de).or_else(|| anterior(&texto, termo, texto.len()))
    };
    match achado {
        Some(idx) => {
            let from = posicao_utf16(&texto, idx);
            let to = posicao_utf16(&texto, idx + termo.len());
            editor.set_selection((from, to));
            None
        }
        None => Some(format!("Não achou “{termo}”")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use freya::text_edit::Rope;

    fn editor_de_teste(texto: &str) -> CodeEditorData {
        CodeEditorData::new(Rope::from_str(texto), None)
    }

    #[test]
    fn comentar_marca_sujo_e_desfaz() {
        let mut ed = editor_de_teste("x = 1\n");
        ed.mark_as_saved();
        comentar(&mut ed);
        assert_eq!(ed.rope.to_string(), "// x = 1\n");
        assert!(ed.is_edited());
        assert!(ed.undo().is_some());
        assert_eq!(ed.rope.to_string(), "x = 1\n");
    }

    #[test]
    fn indentar_linha_empilha_historico() {
        let mut ed = editor_de_teste("se verdadeiro {\nx = 1\n}\n");
        ed.mark_as_saved();
        ed.move_cursor_to_row(1, 0);
        indentar_atual(&mut ed);
        assert_eq!(ed.rope.to_string(), "se verdadeiro {\n    x = 1\n}\n");
        assert!(ed.is_edited());
        assert!(ed.undo().is_some());
        assert_eq!(ed.rope.to_string(), "se verdadeiro {\nx = 1\n}\n");
    }

    #[test]
    fn tecla_m_nao_e_sempre_indent() {
        assert!(tecla_pede_indent(
            &Key::Character("m".into()),
            Modifiers::default()
        ));
        assert!(tecla_pede_indent(
            &Key::Named(NamedKey::Enter),
            Modifiers::default()
        ));
        assert!(!tecla_pede_indent(
            &Key::Character("a".into()),
            Modifiers::default()
        ));
    }

    #[test]
    fn m_no_meio_da_palavra_nao_recua() {
        let mut ed = editor_de_teste("    tempo\n");
        ed.move_cursor_to_row(0, 9);
        reindentar_depois(&mut ed);
        assert_eq!(ed.rope.to_string(), "    tempo\n");
    }

    #[test]
    fn fim_sozinho_recua_depois_da_tecla() {
        let mut ed = editor_de_teste("se verdadeiro {\n    fim\n");
        ed.move_cursor_to_row(1, 7);
        reindentar_depois(&mut ed);
        assert_eq!(ed.rope.to_string(), "se verdadeiro {\nfim\n");
    }

    #[test]
    fn enter_indenta_linha_em_branco() {
        let mut ed = editor_de_teste("se verdadeiro {\n\n");
        ed.move_cursor_to_row(1, 0);
        reindentar_depois(&mut ed);
        assert_eq!(ed.rope.to_string(), "se verdadeiro {\n    \n");
    }
}
