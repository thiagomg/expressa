//! Electric indent, comment, duplicate and search applied to the Freya buffer.

use freya::code_editor::CodeEditorData;
use freya::prelude::*;
use freya::text_edit::TextEditor;

use crate::busca::{anterior, proxima};
use crate::editor::{
    aplicar_texto, byte_do_cursor, linhas_selecionadas, posicao_utf16,
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
    aplicar_texto(editor, &novo, 0);
    editor.move_cursor_to_row(row, col.saturating_add_signed(delta as isize));
}

pub fn comentar(editor: &mut CodeEditorData) {
    let (a, b) = linhas_selecionadas(editor);
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = comentar_linhas(&editor.rope.to_string(), a, b);
    aplicar_texto(editor, &novo, 0);
    editor.move_cursor_to_row(row, col);
}

pub fn duplicar(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = duplicar_linha(&editor.rope.to_string(), row);
    aplicar_texto(editor, &novo, 0);
    editor.move_cursor_to_row(row + 1, col);
}

pub fn mover(editor: &mut CodeEditorData, delta: i32) {
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let (novo, dest) = mover_linha(&editor.rope.to_string(), row, delta);
    aplicar_texto(editor, &novo, 0);
    editor.move_cursor_to_row(dest, col);
}

pub fn recuar_arquivo(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let col = editor.cursor_col();
    let novo = indentar_todo(&editor.rope.to_string());
    aplicar_texto(editor, &novo, 0);
    editor.move_cursor_to_row(row, col);
}

pub fn indentar_atual(editor: &mut CodeEditorData) {
    let row = editor.cursor_row();
    let Some(novo) = indentar_linha(&editor.rope.to_string(), row) else {
        return;
    };
    aplicar_texto(editor, &novo, 0);
    let linha = novo.split('\n').nth(row).unwrap_or("");
    let col = linha.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    editor.move_cursor_to_row(row, col);
}

/// Handle Tab / Enter / closer keys. `true` lets the editor keep the event.
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
        Key::Named(NamedKey::Enter) if !e.modifiers.contains(Modifiers::CONTROL) => {
            e.stop_propagation();
            e.prevent_default();
            let pos = editor.cursor_pos();
            let n = editor.insert("\n", pos);
            editor.move_cursor_to(pos + n);
            editor.parse();
            indentar_atual(editor);
            false
        }
        Key::Character(c) if c == "}" || c == "m" => {
            let pos = editor.cursor_pos();
            let n = editor.insert(c, pos);
            editor.move_cursor_to(pos + n);
            editor.parse();
            let row = editor.cursor_row();
            let linha = editor.rope.to_string();
            let atual = linha.split('\n').nth(row).unwrap_or("");
            if starts_with_closer(atual) {
                indentar_atual(editor);
            }
            e.stop_propagation();
            e.prevent_default();
            false
        }
        _ => true,
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
