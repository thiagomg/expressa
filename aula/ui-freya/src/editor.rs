//! Code-editor buffer helpers.

use freya::{
    code_editor::{CodeEditorData, EditorSyntaxTheme},
    text_edit::Rope,
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

pub fn new_editor(texto: &str) -> CodeEditorData {
    let rope = Rope::from_str(texto);
    let mut editor = CodeEditorData::new(rope, None);
    editor.set_theme(EditorSyntaxTheme::dark());
    editor.parse();
    editor.measure(FONT_SIZE, FONT_FAMILY);
    editor
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
}
