//! Code-editor buffer helpers.

use std::sync::OnceLock;

use freya::{
    code_editor::{CodeEditorData, EditorLanguage, EditorSyntaxTheme},
    engine::prelude::{FontMgr, FontStyle, Typeface},
    text_edit::{Rope, TextEditor},
};

/// First candidate; the runtime family is [`fonte_familia`].
pub const FONT_FAMILY: &str = "DejaVu Sans Mono";
/// Families tried in order until one has `ãéçõ`.
pub const FONTES_CANDIDATAS: &[&str] = &[
    "DejaVu Sans Mono",
    "Cascadia Mono",
    "Consolas",
    "Liberation Mono",
    "Noto Sans Mono",
    "monospace",
];
const AMOSTRA_FONTE: &str = "ãéçõ";
static FAMILIA: OnceLock<String> = OnceLock::new();

/// Default pixel size matching GTK Aula's 11 pt.
pub const FONT_SIZE: f32 = 16.0;

/// Map the prefs `fonte` (points, same as GTK) to pixels for Freya.
pub fn fonte_px(pt: u32) -> f32 {
    (pt as f32) * FONT_SIZE / 11.0
}

/// Family used by the editor, console, REPL and debug pane.
pub fn fonte_familia() -> &'static str {
    FAMILIA.get_or_init(escolher_fonte).as_str()
}

/// Pin the family from prefs when it still covers Portuguese, else pick one.
pub fn instalar_fonte(preferida: Option<&str>) -> &'static str {
    FAMILIA
        .get_or_init(|| {
            if let Some(nome) = preferida
                && !nome.is_empty()
                && familia_serve(nome)
            {
                return nome.to_string();
            }
            escolher_fonte()
        })
        .as_str()
}

pub fn escolher_fonte() -> String {
    FONTES_CANDIDATAS
        .iter()
        .copied()
        .find(|n| familia_serve(n))
        .unwrap_or("monospace")
        .to_string()
}

fn familia_serve(nome: &str) -> bool {
    let mgr = FontMgr::new();
    let mut set = mgr.match_family(nome);
    let face = if set.count() > 0 {
        set.match_style(FontStyle::normal())
            .or_else(|| set.new_typeface(0))
    } else {
        mgr.match_family_style(nome, FontStyle::normal())
    };
    face.as_ref().is_some_and(amostra_cabe)
}

fn amostra_cabe(face: &Typeface) -> bool {
    AMOSTRA_FONTE.chars().all(|c| face.unichar_to_glyph(c as i32) != 0)
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
    editor.measure(FONT_SIZE, fonte_familia());
    editor
}

/// Replace the buffer without pushing undo history (open file, recover draft).
#[allow(dead_code)]
pub fn aplicar_texto(editor: &mut CodeEditorData, texto: &str, cursor: usize) {
    editor.set(texto);
    editor.parse();
    let max = editor.len_utf16_cu();
    editor.move_cursor_to(cursor.min(max));
}

/// Apply a student edit with insert/remove so undo and `is_edited` stay correct.
pub fn substituir_texto(editor: &mut CodeEditorData, novo: &str) {
    let velho = editor.rope.to_string();
    if velho == novo {
        return;
    }
    let (start, old_end, inserted) = faixa_diferente(&velho, novo);
    if start < old_end {
        editor.remove(start..old_end);
    }
    if !inserted.is_empty() {
        editor.insert(&inserted, start);
    }
    editor.parse();
}

/// UTF-16 range in `velho` that must change, plus the replacement slice of `novo`.
fn faixa_diferente(velho: &str, novo: &str) -> (usize, usize, String) {
    let a: Vec<char> = velho.chars().collect();
    let b: Vec<char> = novo.chars().collect();
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    let mut ja = a.len();
    let mut jb = b.len();
    while ja > i && jb > i && a[ja - 1] == b[jb - 1] {
        ja -= 1;
        jb -= 1;
    }
    let start: usize = a[..i].iter().map(|c| c.len_utf16()).sum();
    let old_end = start + a[i..ja].iter().map(|c| c.len_utf16()).sum::<usize>();
    let inserted: String = b[i..jb].iter().collect();
    (start, old_end, inserted)
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
    fn escolhe_familia_com_acentos() {
        assert_eq!(FONTES_CANDIDATAS[0], FONT_FAMILY);
        assert_eq!(*FONTES_CANDIDATAS.last().unwrap(), "monospace");
        assert!(familia_serve(FONT_FAMILY) || familia_serve("monospace"));
        let escolhida = escolher_fonte();
        assert!(familia_serve(&escolhida), "{escolhida}");
        assert!(!familia_serve("FonteQueNaoExiste123"));
    }

    #[test]
    fn sintaxe_aponta_a_linha() {
        let msg = mensagem_sintaxe("se");
        assert!(msg.contains("em "), "{msg}");
        assert_eq!(mensagem_sintaxe("escreva(1)\n"), "");
    }

    #[test]
    fn faixa_so_o_trecho_que_mudou() {
        let (s, e, ins) = faixa_diferente("abc", "aXc");
        assert_eq!((s, e, ins.as_str()), (1, 2, "X"));
        let (s, e, ins) = faixa_diferente("x = 1\n", "// x = 1\n");
        assert_eq!(ins, "// ");
        assert_eq!(s, 0);
        assert_eq!(e, 0);
    }

    fn editor_de_teste(texto: &str) -> CodeEditorData {
        CodeEditorData::new(Rope::from_str(texto), None)
    }

    #[test]
    fn substituir_marca_editado_e_desfaz() {
        let mut ed = editor_de_teste("x = 1\n");
        ed.mark_as_saved();
        assert!(!ed.is_edited());
        substituir_texto(&mut ed, "// x = 1\n");
        assert_eq!(ed.rope.to_string(), "// x = 1\n");
        assert!(ed.is_edited());
        assert!(ed.undo().is_some());
        assert_eq!(ed.rope.to_string(), "x = 1\n");
    }
}
