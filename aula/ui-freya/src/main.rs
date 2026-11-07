//! Expressa Aula on Freya. Ships beside the GTK `expressa-aula` binary.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod editor;

use freya::{
    code_editor::{CodeEditor, CodeEditorThemeExt},
    prelude::*,
};

use crate::editor::{FONT_FAMILY, FONT_SIZE, new_editor};

const TITULO: &str = "Expressa Aula";

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(app)
                .with_title(TITULO)
                .with_size(1200.0, 800.0),
        ),
    );
}

fn app() -> impl IntoElement {
    use_init_theme(|| dark_theme().with_dark_code_editor());
    let a11y_id = use_a11y();
    let editor = use_state(|| new_editor(editor::TEXTO_INICIAL));

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background((24, 24, 24))
        .child(
            CodeEditor::new(editor, a11y_id)
                .font_size(FONT_SIZE)
                .font_family(FONT_FAMILY),
        )
}
