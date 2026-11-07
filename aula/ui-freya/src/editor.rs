//! Code-editor buffer helpers.

use freya::{
    code_editor::{CodeEditorData, EditorLanguage, EditorSyntaxTheme},
    text_edit::{Rope, TextEditor},
};

/// Monospace family that covers Portuguese on typical Linux desktops.
pub const FONT_FAMILY: &str = "DejaVu Sans Mono";
/// Default pixel size matching GTK Aula's 11 pt.
pub const FONT_SIZE: f32 = 16.0;

/// Map the prefs `fonte` (points, same as GTK) to pixels for Freya.
pub fn fonte_px(pt: u32) -> f32 {
    (pt as f32) * FONT_SIZE / 11.0
}

pub const TEXTO_INICIAL: &str = "\
// Expressa Aula (Freya)
// Digite função, não, início para conferir o teclado.

escreva(\"olá\")
";

pub fn linguagem() -> EditorLanguage {
    EditorLanguage::new(
        tree_sitter_expressa::LANGUAGE,
        tree_sitter_expressa::HIGHLIGHTS_QUERY,
    )
}

pub fn new_editor(texto: &str) -> CodeEditorData {
    let rope = Rope::from_str(texto);
    let mut editor = CodeEditorData::new(rope, Some(linguagem()));
    editor.set_theme(EditorSyntaxTheme::dark());
    editor.parse();
    editor.measure(FONT_SIZE, FONT_FAMILY);
    editor
}

/// Replace the buffer and put the caret at `cursor` (UTF-16 units).
pub fn aplicar_texto(editor: &mut CodeEditorData, texto: &str, cursor: usize) {
    editor.set(texto);
    editor.parse();
    let max = editor.len_utf16_cu();
    editor.move_cursor_to(cursor.min(max));
}

pub fn posicao_utf16(texto: &str, byte: usize) -> usize {
    texto.get(..byte.min(texto.len())).unwrap_or("").encode_utf16().count()
}

pub fn byte_do_cursor(editor: &CodeEditorData) -> usize {
    let texto = editor.rope.to_string();
    let chars = editor.utf16_cu_to_char(editor.cursor_pos());
    texto.chars().take(chars).map(char::len_utf8).sum()
}

/// Inclusive line range of the selection, or the caret line.
pub fn linhas_selecionadas(editor: &CodeEditorData) -> (usize, usize) {
    if let Some((from, to)) = editor.get_selection() {
        let a = from.min(to);
        let b = from.max(to);
        let r1 = editor.char_to_line(editor.utf16_cu_to_char(a));
        let r2 = editor.char_to_line(editor.utf16_cu_to_char(b.saturating_sub(1).max(a)));
        (r1.min(r2), r1.max(r2))
    } else {
        let r = editor.cursor_row();
        (r, r)
    }
}

pub fn mensagem_sintaxe(texto: &str) -> String {
    match expressa::parser::parse(texto) {
        Ok(_) => String::new(),
        Err(e) => format!("{} em {}:{}", e.message, e.span.line, e.span.col),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_text_is_portuguese() {
        assert!(TEXTO_INICIAL.contains("função"));
        assert!(TEXTO_INICIAL.contains("não"));
        assert!(TEXTO_INICIAL.contains("início"));
    }

    #[test]
    fn rope_roundtrip() {
        let rope = Rope::from_str("função não\n");
        assert_eq!(rope.to_string(), "função não\n");
        assert_eq!(rope.len_lines(), 2);
    }

    #[test]
    fn fonte_padrao_casa_com_gtk() {
        assert!((fonte_px(11) - FONT_SIZE).abs() < f32::EPSILON);
    }

    #[test]
    fn sintaxe_aponta_a_linha() {
        let msg = mensagem_sintaxe("se");
        assert!(msg.contains("em "), "{msg}");
        assert_eq!(mensagem_sintaxe("escreva(1)\n"), "");
    }
}
