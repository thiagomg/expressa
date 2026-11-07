//! Dispatch of menu and keyboard actions.

use std::path::PathBuf;

use freya::code_editor::CodeEditorData;
use freya::prelude::*;
use freya::text_edit::TextEditor;

use crate::atalhos::{Acao, mensagem_reservada};
use crate::documentos::{
    Abas, Depois, Dialogo, Fechar, carregar_editor, escolher_abrir, escolher_salvar, gravar_arquivo,
    gravar_rascunhos, ler_arquivo, nome_de_caminho, posicao_linha, sincronizar,
};
use crate::prefs::Prefs;
use crate::editor::fonte_px;
use crate::shell::{aplicar_tema_editor, tema_freya};

pub fn aplicar(
    acao: Acao,
    prefs: State<Prefs>,
    editor: State<CodeEditorData>,
    mut status: State<String>,
    theme: State<Theme>,
    mut atalhos_visiveis: State<bool>,
    abas: State<Abas>,
    mut dialogo: State<Dialogo>,
) {
    match acao {
        Acao::Sair => sair(prefs, editor, status, abas, dialogo),
        Acao::TemaClaro => definir_tema(false, prefs, editor, theme, status),
        Acao::TemaEscuro => definir_tema(true, prefs, editor, theme, status),
        Acao::ZoomMais => zoom(1, prefs, editor, status),
        Acao::ZoomMenos => zoom(-1, prefs, editor, status),
        Acao::ZoomNormal => zoom(0, prefs, editor, status),
        Acao::Atalhos => {
            atalhos_visiveis.set(true);
            status.set("Atalhos do teclado".into());
        }
        Acao::Novo => novo(editor, prefs, abas, status),
        Acao::Abrir => abrir(editor, prefs, abas, status, dialogo),
        Acao::Salvar => {
            let _ = salvar(false, editor, prefs, abas, status);
        }
        Acao::SalvarComo => {
            let _ = salvar(true, editor, prefs, abas, status);
        }
        Acao::FecharAba => fechar_aba(None, Depois::SoFechar, editor, prefs, abas, status, dialogo),
        Acao::IrParaLinha => dialogo.set(Dialogo::IrParaLinha),
        other => {
            let msg = mensagem_reservada(other);
            if !msg.is_empty() {
                status.set(msg.to_string());
            }
        }
    }
}

fn definir_tema(
    dark: bool,
    mut prefs: State<Prefs>,
    mut editor: State<CodeEditorData>,
    mut theme: State<Theme>,
    mut status: State<String>,
) {
    if prefs.peek().dark == dark {
        return;
    }
    prefs.write().dark = dark;
    prefs.read().save();
    theme.set(tema_freya(dark));
    aplicar_tema_editor(&mut editor.write(), dark);
    status.set(if dark {
        "Tema escuro".into()
    } else {
        "Tema claro".into()
    });
}

fn zoom(
    step: i32,
    mut prefs: State<Prefs>,
    mut editor: State<CodeEditorData>,
    mut status: State<String>,
) {
    prefs.write().zoom(step);
    prefs.read().save();
    let pt = prefs.read().font_size;
    editor.write().measure(fonte_px(pt), crate::editor::FONT_FAMILY);
    status.set(format!("Letra {pt} pt"));
}

fn novo(
    mut editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
) {
    sincronizar(&mut abas.write(), &editor.read());
    abas.write().nova_aba("");
    carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
    status.set("Nova aba".into());
}

fn abrir(
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    status: State<String>,
    _dialogo: State<Dialogo>,
) {
    let Some(path) = escolher_abrir() else {
        return;
    };
    abrir_caminho(path, editor, prefs, abas, status);
}

pub fn abrir_caminho(
    path: PathBuf,
    mut editor: State<CodeEditorData>,
    mut prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
) {
    sincronizar(&mut abas.write(), &editor.read());
    let ja = abas.read().achar_caminho(&path);
    if let Some(idx) = ja {
        abas.write().ir_para(idx);
        carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
        status.set(format!("Já aberto: {}", nome_de_caminho(&path)));
        return;
    }
    match ler_arquivo(&path) {
        Ok(texto) => {
            let so_vazia = {
                let a = abas.read();
                a.itens.len() == 1 && !a.atual().dirty && a.atual().caminho.is_none()
            };
            {
                let mut abas = abas.write();
                if so_vazia {
                    let aba = abas.atual_mut();
                    aba.texto = texto;
                    aba.caminho = Some(path.clone());
                    aba.nome = nome_de_caminho(&path);
                    aba.dirty = false;
                } else {
                    abas.nova_aba(&texto);
                    let aba = abas.atual_mut();
                    aba.caminho = Some(path.clone());
                    aba.nome = nome_de_caminho(&path);
                    aba.dirty = false;
                }
            }
            carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
            prefs.write().arquivo = path.display().to_string();
            prefs.read().save();
            status.set(format!("Aberto {}", nome_de_caminho(&path)));
        }
        Err(e) => status.set(format!("Não abriu: {e}")),
    }
}

/// Returns whether the file was saved.
pub fn salvar(
    forcar_nome: bool,
    mut editor: State<CodeEditorData>,
    mut prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
) -> bool {
    sincronizar(&mut abas.write(), &editor.read());
    let caminho = if forcar_nome || abas.read().atual().caminho.is_none() {
        let sugerido = abas.read().atual().nome.clone();
        let Some(p) = escolher_salvar(&sugerido) else {
            return false;
        };
        p
    } else {
        abas.read().atual().caminho.clone().unwrap()
    };
    let texto = editor.read().rope.to_string();
    match gravar_arquivo(&caminho, &texto) {
        Ok(()) => {
            {
                let mut abas = abas.write();
                let aba = abas.atual_mut();
                aba.caminho = Some(caminho.clone());
                aba.nome = nome_de_caminho(&caminho);
                aba.texto = texto;
                aba.dirty = false;
            }
            editor.write().mark_as_saved();
            prefs.write().arquivo = caminho.display().to_string();
            prefs.read().save();
            status.set(format!("Salvo {}", nome_de_caminho(&caminho)));
            true
        }
        Err(e) => {
            status.set(format!("Não salvou: {e}"));
            false
        }
    }
}

pub fn fechar_aba(
    id: Option<u64>,
    depois: Depois,
    mut editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
) {
    sincronizar(&mut abas.write(), &editor.read());
    let id = id.unwrap_or_else(|| abas.read().atual().id);
    let resultado = abas.write().fechar_id(id);
    match resultado {
        Fechar::Confirma => dialogo.set(Dialogo::Confirmar { id, depois }),
        Fechar::Fecha => {
            carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
            status.set("Aba fechada".into());
            if depois == Depois::Sair {
                Platform::get().close_window(Platform::window_id());
            } else if depois == Depois::Abrir {
                abrir(editor, prefs, abas, status, dialogo);
            } else if depois == Depois::Novo {
                novo(editor, prefs, abas, status);
            }
        }
    }
}

pub fn confirmar_descarte(
    salvar_antes: bool,
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    mut abas: State<Abas>,
    status: State<String>,
    mut dialogo: State<Dialogo>,
) {
    let Dialogo::Confirmar { id, depois } = dialogo.read().clone() else {
        return;
    };
    if salvar_antes && !salvar(false, editor, prefs, abas, status) {
        return;
    }
    if !salvar_antes {
        let idx = abas.read().itens.iter().position(|a| a.id == id);
        if let Some(idx) = idx {
            abas.write().itens[idx].dirty = false;
        }
    }
    dialogo.set(Dialogo::Nenhum);
    fechar_aba(Some(id), depois, editor, prefs, abas, status, dialogo);
}

fn sair(
    _prefs: State<Prefs>,
    editor: State<CodeEditorData>,
    _status: State<String>,
    mut abas: State<Abas>,
    mut dialogo: State<Dialogo>,
) {
    sincronizar(&mut abas.write(), &editor.read());
    gravar_rascunhos(&abas.read());
    if abas.read().alguma_suja() {
        let id = abas.read().atual().id;
        dialogo.set(Dialogo::Confirmar {
            id,
            depois: Depois::Sair,
        });
        return;
    }
    Platform::get().close_window(Platform::window_id());
}

pub fn ir_para_linha(
    texto_linha: &str,
    mut editor: State<CodeEditorData>,
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
) {
    dialogo.set(Dialogo::Nenhum);
    let Ok(n) = texto_linha.trim().parse::<usize>() else {
        status.set("Linha inválida".into());
        return;
    };
    let texto = editor.read().rope.to_string();
    match posicao_linha(&texto, n) {
        Some(pos) => {
            editor.write().move_cursor_to(pos);
            status.set(format!("Linha {n}"));
        }
        None => status.set(format!("Não há linha {n}")),
    }
}

pub fn selecionar_aba(
    idx: usize,
    mut editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    mut abas: State<Abas>,
) {
    if idx == abas.read().atual {
        return;
    }
    sincronizar(&mut abas.write(), &editor.read());
    abas.write().ir_para(idx);
    carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
}
