//! Dispatch of menu and keyboard actions.

use std::path::{Path, PathBuf};

use freya::code_editor::CodeEditorData;
use freya::prelude::*;
use freya::text_edit::TextEditor;

use crate::ajuda::{
    Sugestao, completar, ficha, garantir_importe, palavra_em, prefixo_em, spawn_tutorial,
};
use crate::atalhos::Acao;
use crate::debug::Depuracao;
use crate::grafico::Graficos;
use crate::repl::Repl;
use crate::documentos::{
    Abas, Depois, DepoisNome, Dialogo, Fechar, carregar_editor, escolher_abrir, escolher_salvar,
    expected_da_aba, gravar_rascunhos, guardar_live, instalar_editor, ler_arquivo, nome_de_caminho,
    posicao_linha, revisao_de_arquivo, sincronizar, trocar_aba,
};
use crate::turma::Turma;
use expressa_aula_storage::{Expected, SaveError};
use crate::edicao::{buscar, comentar, duplicar, mover, recuar_arquivo};
use crate::editor::fonte_px;
use crate::prefs::Prefs;
use crate::console::{Console, parse_error_location};
use crate::execucao::{
    EventoExec, Execucao, argumentos, caminho_relativo, preparar_copia, pasta_exec,
};
use crate::projeto::{
    Projeto, apagar, abrir_raiz, criar_arquivo, criar_pasta, escolher_pasta, pasta_do_item, renomear,
};
use crate::shell::{aplicar_tema_editor, tema_freya};

pub fn aplicar(
    acao: Acao,
    prefs: State<Prefs>,
    mut editor: State<CodeEditorData>,
    mut status: State<String>,
    theme: State<Theme>,
    mut atalhos_visiveis: State<bool>,
    abas: State<Abas>,
    mut dialogo: State<Dialogo>,
    mut busca_aberta: State<bool>,
    busca_termo: State<String>,
    mut projeto: State<Projeto>,
    mut exec: State<Execucao>,
    console: State<Console>,
    args: State<String>,
    mut ajuda: State<Option<String>>,
    mut catalogo: State<bool>,
    mut sugestoes: State<Vec<Sugestao>>,
    mut depuracao: State<Depuracao>,
    mut repl: State<Repl>,
    graficos: State<Graficos>,
    turma: State<Turma>,
) {
    match acao {
        Acao::Sair => sair(prefs, editor, status, abas, dialogo),
        Acao::Servidor => {
            dialogo.set(Dialogo::Servidor);
            status.set("Servidor".into());
        }
        Acao::TemaClaro => definir_tema(false, prefs, editor, theme, status),
        Acao::TemaEscuro => definir_tema(true, prefs, editor, theme, status),
        Acao::ZoomMais => zoom(1, prefs, editor, status),
        Acao::ZoomMenos => zoom(-1, prefs, editor, status),
        Acao::ZoomNormal => zoom(0, prefs, editor, status),
        Acao::Atalhos => {
            atalhos_visiveis.set(true);
            status.set("Atalhos do teclado".into());
        }
        Acao::Executar => {
            if exec.read().rodando && depuracao.read().pausado {
                exec.write().comando("continuar");
                depuracao.write().pausado = false;
                status.set("Continuando…".into());
            } else {
                executar(
                    editor, prefs, abas, status, projeto, exec, console, args, false, depuracao,
                    graficos,
                );
            }
        }
        Acao::Parar => {
            exec.write().parar();
            status.set("Parando…".into());
        }
        Acao::Depurar => executar(
            editor, prefs, abas, status, projeto, exec, console, args, true, depuracao, graficos,
        ),
        Acao::Ponto => alternar_ponto(editor, prefs, abas, status, projeto, exec, depuracao),
        Acao::Proximo => comando_debug("proximo", exec, status, depuracao),
        Acao::Entrar => comando_debug("entrar", exec, status, depuracao),
        Acao::SairFuncao => comando_debug("sair", exec, status, depuracao),
        Acao::MostrarPainel => {
            let visivel = !depuracao.read().visivel;
            depuracao.write().visivel = visivel;
            status.set(if visivel {
                "Painel visível".into()
            } else {
                "Painel oculto".into()
            });
        }
        Acao::Repl => {
            let visivel = !repl.read().visivel;
            repl.write().visivel = visivel;
            if visivel {
                repl.write().iniciar();
            }
            status.set(if visivel {
                "REPL".into()
            } else {
                "REPL oculto".into()
            });
        }
        Acao::Novo => novo(editor, prefs, abas, status),
        Acao::Abrir => abrir(editor, prefs, abas, status, dialogo),
        Acao::AbrirProjeto => escolher_projeto(editor, prefs, abas, status, projeto, turma),
        Acao::MostrarArquivos => {
            let visivel = !projeto.read().visivel;
            projeto.write().visivel = visivel;
            status.set(if visivel {
                "Arquivos visíveis".into()
            } else {
                "Arquivos ocultos".into()
            });
        }
        Acao::Salvar => {
            let _ = salvar(false, editor, prefs, abas, status, turma, dialogo, None);
        }
        Acao::SalvarComo => {
            let _ = salvar(true, editor, prefs, abas, status, turma, dialogo, None);
        }
        Acao::FecharAba => fechar_aba(None, Depois::SoFechar, editor, prefs, abas, status, dialogo),
        Acao::IrParaLinha => dialogo.set(Dialogo::IrParaLinha),
        Acao::Buscar => {
            if busca_aberta() {
                achar(true, editor, status, busca_termo);
            } else {
                busca_aberta.set(true);
                status.set("Buscar".into());
            }
        }
        Acao::BuscarProxima => achar(true, editor, status, busca_termo),
        Acao::Escapar => {
            busca_aberta.set(false);
            dialogo.set(Dialogo::Nenhum);
            atalhos_visiveis.set(false);
            ajuda.set(None);
            catalogo.set(false);
            sugestoes.set(Vec::new());
        }
        Acao::Ajuda => mostrar_ajuda(editor, status, ajuda, catalogo),
        Acao::Catalogo => {
            catalogo.set(true);
            status.set("Funções nativas".into());
        }
        Acao::Tutoriais => match spawn_tutorial() {
            Ok(()) => status.set("Tutoriais".into()),
            Err(e) => status.set(e),
        },
        Acao::Completar => {
            let texto = editor.read().rope.to_string();
            let byte = crate::editor::byte_do_cursor(&editor.read());
            sugestoes.set(completar(&texto, byte));
        }
        Acao::IrDefinicao => ir_definicao(editor, status),
        Acao::Comentar => {
            comentar(&mut editor.write());
            status.set("Comentário".into());
        }
        Acao::Duplicar => {
            duplicar(&mut editor.write());
            status.set("Linha duplicada".into());
        }
        Acao::Recuar => {
            recuar_arquivo(&mut editor.write());
            status.set("Recuo ajustado".into());
        }
        Acao::MoverCima => {
            mover(&mut editor.write(), -1);
            status.set("Linha movida".into());
        }
        Acao::MoverBaixo => {
            mover(&mut editor.write(), 1);
            status.set("Linha movida".into());
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
    guardar_live(&abas.read(), &mut editor.write());
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
        trocar_aba(&mut editor.write(), &mut abas.write(), idx, &prefs.read());
        status.set(format!("Já aberto: {}", nome_de_caminho(&path)));
        return;
    }
    match ler_arquivo(&path) {
        Ok(texto) => {
            let so_vazia = {
                let a = abas.read();
                a.itens.len() == 1 && !a.atual().dirty && a.atual().caminho.is_none()
            };
            if !so_vazia {
                guardar_live(&abas.read(), &mut editor.write());
            }
            {
                let mut abas = abas.write();
                if so_vazia {
                    let aba = abas.atual_mut();
                    aba.texto = texto;
                    aba.caminho = Some(path.clone());
                    aba.nome = nome_de_caminho(&path);
                    aba.dirty = false;
                    aba.revisao = revisao_de_arquivo(&path);
                    aba.rel_remoto = None;
                } else {
                    abas.nova_aba(&texto);
                    let aba = abas.atual_mut();
                    aba.caminho = Some(path.clone());
                    aba.nome = nome_de_caminho(&path);
                    aba.dirty = false;
                    aba.revisao = revisao_de_arquivo(&path);
                    aba.rel_remoto = None;
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
    turma: State<Turma>,
    mut dialogo: State<Dialogo>,
    expected_override: Option<Expected>,
) -> bool {
    sincronizar(&mut abas.write(), &editor.read());
    let rel_remoto = abas.read().atual().rel_remoto.clone();
    let caminho = if forcar_nome || (abas.read().atual().caminho.is_none() && rel_remoto.is_none()) {
        let sugerido = abas.read().atual().nome.clone();
        let Some(p) = escolher_salvar(&sugerido) else {
            return false;
        };
        p
    } else if let Some(p) = abas.read().atual().caminho.clone() {
        p
    } else if let Some(rel) = &rel_remoto {
        turma.read().caminho_cache(rel)
    } else {
        return false;
    };
    let texto = editor.read().rope.to_string();
    let expected = expected_override.unwrap_or_else(|| expected_da_aba(abas.read().atual(), &caminho));
    let remoto = rel_remoto.is_some() && turma.read().conectado && !forcar_nome;
    let resultado = if remoto {
        let rel = rel_remoto.clone().unwrap();
        match turma.read().gravar(&rel, &texto, &expected) {
            Ok(saved) => {
                let _ = turma.read().gravar_cache(&rel, &texto);
                Ok(saved.revision)
            }
            Err(e) => Err(e),
        }
    } else {
        match crate::turma::gravar_local_protegido(&caminho, &texto, &expected) {
            Ok(saved) => Ok(saved.revision),
            Err(e) => Err(e),
        }
    };
    match resultado {
        Ok(revisao) => {
            {
                let mut abas = abas.write();
                let aba = abas.atual_mut();
                aba.caminho = Some(caminho.clone());
                aba.nome = nome_de_caminho(&caminho);
                aba.texto = texto;
                aba.dirty = false;
                aba.revisao = Some(revisao);
                if forcar_nome {
                    aba.rel_remoto = None;
                }
            }
            editor.write().mark_as_saved();
            crate::documentos::apagar_rascunho(abas.read().atual().id);
            prefs.write().arquivo = caminho.display().to_string();
            prefs.read().save();
            status.set(format!("Salvo {}", nome_de_caminho(&caminho)));
            true
        }
        Err(SaveError::Conflict) => {
            let disco = if remoto {
                rel_remoto
                    .as_deref()
                    .and_then(|rel| turma.read().ler(rel).ok().map(|(t, _)| t))
                    .unwrap_or_default()
            } else {
                ler_arquivo(&caminho).unwrap_or_default()
            };
            let revisao_disco = if remoto {
                rel_remoto
                    .as_deref()
                    .and_then(|rel| turma.read().ler(rel).ok().and_then(|(_, r)| r))
            } else {
                revisao_de_arquivo(&caminho)
            };
            dialogo.set(Dialogo::Conflito {
                nome: nome_de_caminho(&caminho),
                aba: texto,
                disco,
                revisao_disco,
                remoto,
            });
            status.set("Conflito ao salvar".into());
            false
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
    let fechando_atual = abas.read().atual().id == id;
    let resultado = abas.write().fechar_id(id);
    match resultado {
        Fechar::Confirma => dialogo.set(Dialogo::Confirmar { id, depois }),
        Fechar::Fecha => {
            if fechando_atual {
                instalar_editor(
                    &mut editor.write(),
                    &abas.read(),
                    abas.read().atual(),
                    &prefs.read(),
                );
            }
            status.set("Aba fechada".into());
            if depois == Depois::Sair {
                sair(prefs, editor, status, abas, dialogo);
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
    turma: State<Turma>,
) {
    let Dialogo::Confirmar { id, depois } = dialogo.read().clone() else {
        return;
    };
    if salvar_antes && !salvar(false, editor, prefs, abas, status, turma, dialogo, None) {
        return;
    }
    if !salvar_antes {
        let idx = abas.read().itens.iter().position(|a| a.id == id);
        if let Some(idx) = idx {
            abas.write().itens[idx].dirty = false;
        }
        crate::documentos::apagar_rascunho(id);
    }
    dialogo.set(Dialogo::Nenhum);
    fechar_aba(Some(id), depois, editor, prefs, abas, status, dialogo);
}

fn sair(
    prefs: State<Prefs>,
    mut editor: State<CodeEditorData>,
    mut status: State<String>,
    mut abas: State<Abas>,
    mut dialogo: State<Dialogo>,
) {
    sincronizar(&mut abas.write(), &editor.read());
    gravar_rascunhos(&abas.read());
    let Some(id) = abas.read().proxima_suja() else {
        Platform::get().close_window(Platform::window_id());
        return;
    };
    let idx = abas.read().itens.iter().position(|a| a.id == id);
    if let Some(idx) = idx {
        trocar_aba(&mut editor.write(), &mut abas.write(), idx, &prefs.read());
        status.set(format!("Salvar {}?", abas.read().atual().nome));
    }
    dialogo.set(Dialogo::Confirmar {
        id,
        depois: Depois::Sair,
    });
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

pub fn escolher_projeto(
    editor: State<CodeEditorData>,
    mut prefs: State<Prefs>,
    abas: State<Abas>,
    mut status: State<String>,
    mut projeto: State<Projeto>,
    mut turma: State<Turma>,
) {
    let Some(pasta) = escolher_pasta() else {
        return;
    };
    let aberto = abrir_raiz(&mut projeto.write(), &mut prefs.write(), pasta);
    match aberto {
        Ok(()) => {
            turma.write().conectado = false;
            turma.write().projeto.clear();
            status.set("Projeto aberto".into());
            restaurar_arquivo(editor, prefs, abas, status);
        }
        Err(e) => status.set(e),
    }
}

pub fn restaurar_arquivo(
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    status: State<String>,
) {
    let arquivo = prefs.read().arquivo.clone();
    if arquivo.is_empty() {
        return;
    }
    let path = crate::prefs::expand_tilde(&arquivo);
    if path.is_file() {
        abrir_caminho(path, editor, prefs, abas, status);
    }
}

pub fn clicar_arvore(
    rel: String,
    editor: State<CodeEditorData>,
    mut prefs: State<Prefs>,
    abas: State<Abas>,
    mut status: State<String>,
    mut projeto: State<Projeto>,
    turma: State<Turma>,
) {
    let is_dir = projeto.read().todos.iter().any(|n| n.rel == rel && n.is_dir);
    if is_dir {
        let mut p = projeto.write();
        if !p.abertos.remove(&rel) {
            p.abertos.insert(rel.clone());
        }
        p.selecionado = Some(rel);
        return;
    }
    projeto.write().selecionado = Some(rel.clone());
    if projeto.read().remoto && turma.read().conectado {
        abrir_remoto(rel, editor, prefs, abas, status, turma);
        return;
    }
    let Some(raiz) = projeto.read().raiz.clone() else {
        return;
    };
    let path = raiz.join(&rel);
    if path.is_file() {
        prefs.write().arquivo = path.display().to_string();
        prefs.read().save();
        abrir_caminho(path, editor, prefs, abas, status);
    } else {
        status.set(format!("Não achou {rel}"));
    }
}

pub fn abrir_remoto(
    rel: String,
    mut editor: State<CodeEditorData>,
    mut prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
    turma: State<Turma>,
) {
    sincronizar(&mut abas.write(), &editor.read());
    match turma.read().ler(&rel) {
        Ok((texto, revisao)) => {
            let path = turma.read().caminho_cache(&rel);
            let _ = turma.read().gravar_cache(&rel, &texto);
            let ja = abas.read().achar_caminho(&path);
            if let Some(idx) = ja {
                trocar_aba(&mut editor.write(), &mut abas.write(), idx, &prefs.read());
                status.set(format!("Já aberto: {rel}"));
                return;
            }
            let so_vazia = {
                let a = abas.read();
                a.itens.len() == 1 && !a.atual().dirty && a.atual().caminho.is_none()
            };
            if !so_vazia {
                guardar_live(&abas.read(), &mut editor.write());
            }
            {
                let mut abas = abas.write();
                if so_vazia {
                    let aba = abas.atual_mut();
                    aba.texto = texto;
                    aba.caminho = Some(path.clone());
                    aba.nome = nome_de_caminho(&path);
                    aba.dirty = false;
                    aba.revisao = revisao;
                    aba.rel_remoto = Some(rel.clone());
                } else {
                    abas.nova_aba(&texto);
                    let aba = abas.atual_mut();
                    aba.caminho = Some(path.clone());
                    aba.nome = nome_de_caminho(&path);
                    aba.dirty = false;
                    aba.revisao = revisao;
                    aba.rel_remoto = Some(rel.clone());
                }
            }
            carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
            prefs.write().arquivo = path.display().to_string();
            prefs.read().save();
            status.set(format!("Aberto {rel}"));
        }
        Err(e) => status.set(format!("Não abriu: {e}")),
    }
}

pub fn pedir_nome(
    depois: DepoisNome,
    mut dialogo: State<Dialogo>,
    mut nome_campo: State<String>,
) {
    let titulo = match &depois {
        DepoisNome::NovoArquivo(_) => "Novo arquivo".to_string(),
        DepoisNome::NovaPasta(_) => "Nova pasta".to_string(),
        DepoisNome::Renomear(rel) => {
            nome_campo.set(
                std::path::Path::new(rel)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            "Renomear".to_string()
        }
    };
    if matches!(depois, DepoisNome::NovoArquivo(_) | DepoisNome::NovaPasta(_)) {
        nome_campo.set(String::new());
    }
    dialogo.set(Dialogo::Nome { titulo, depois });
}

pub fn confirmar_nome(
    nome: &str,
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
    mut projeto: State<Projeto>,
    turma: State<Turma>,
) {
    let Dialogo::Nome { depois, .. } = dialogo.read().clone() else {
        return;
    };
    dialogo.set(Dialogo::Nenhum);
    if projeto.read().remoto && turma.read().conectado {
        let r = match depois {
            DepoisNome::NovoArquivo(pasta) => {
                let nome = crate::projeto::garantir_lep_nome(nome.trim());
                let rel = if pasta.is_empty() {
                    nome
                } else {
                    format!("{pasta}/{nome}")
                };
                match turma.read().gravar(&rel, "escreva(\"olá\")\n", &Expected::Missing) {
                    Ok(_) => {
                        let _ = turma.read().gravar_cache(&rel, "escreva(\"olá\")\n");
                        let _ = turma.read().recarregar_arvore(&mut projeto.write());
                        abrir_remoto(rel, editor, prefs, abas, status, turma);
                        Ok(())
                    }
                    Err(e) => Err(e.to_string()),
                }
            }
            DepoisNome::NovaPasta(pasta) => {
                let rel = if pasta.is_empty() {
                    nome.trim().to_string()
                } else {
                    format!("{pasta}/{}", nome.trim())
                };
                turma.read().mkdir(&rel).map(|_| {
                    let _ = turma.read().recarregar_arvore(&mut projeto.write());
                    status.set("Pasta criada".into());
                })
            }
            DepoisNome::Renomear(rel) => {
                let dest = match std::path::Path::new(&rel).parent() {
                    Some(p) if p.as_os_str().is_empty() => nome.trim().to_string(),
                    Some(p) => format!("{}/{nome}", p.to_string_lossy().replace('\\', "/")),
                    None => nome.trim().to_string(),
                };
                turma.read().renomear(&rel, &dest).map(|_| {
                    let _ = turma.read().recarregar_arvore(&mut projeto.write());
                    status.set("Renomeado".into());
                })
            }
        };
        if let Err(e) = r {
            status.set(e);
        }
        return;
    }
    let Some(raiz) = projeto.read().raiz.clone() else {
        status.set("Abra um projeto".into());
        return;
    };
    let r = match depois {
        DepoisNome::NovoArquivo(pasta) => criar_arquivo(&raiz, &pasta, nome).map(|p| {
            let _ = projeto.write().recarregar();
            abrir_caminho(p, editor, prefs, abas, status);
        }),
        DepoisNome::NovaPasta(pasta) => criar_pasta(&raiz, &pasta, nome).map(|_| {
            let _ = projeto.write().recarregar();
            status.set("Pasta criada".into());
        }),
        DepoisNome::Renomear(rel) => renomear(&raiz, &rel, nome).map(|p| {
            let _ = projeto.write().recarregar();
            if p.is_file() {
                abrir_caminho(p, editor, prefs, abas, status);
            } else {
                status.set("Renomeado".into());
            }
        }),
    };
    if let Err(e) = r {
        status.set(e);
    }
}

pub fn pedir_apagar(mut dialogo: State<Dialogo>, projeto: State<Projeto>) {
    if let Some(rel) = projeto.read().selecionado.clone() {
        dialogo.set(Dialogo::Apagar { rel });
    }
}

pub fn confirmar_apagar(
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
    mut projeto: State<Projeto>,
    turma: State<Turma>,
) {
    let Dialogo::Apagar { rel } = dialogo.read().clone() else {
        return;
    };
    dialogo.set(Dialogo::Nenhum);
    if projeto.read().remoto && turma.read().conectado {
        match turma.read().apagar(&rel) {
            Ok(()) => {
                projeto.write().selecionado = None;
                let _ = turma.read().recarregar_arvore(&mut projeto.write());
                status.set(format!("Apagou {rel}"));
            }
            Err(e) => status.set(e),
        }
        return;
    }
    let Some(raiz) = projeto.read().raiz.clone() else {
        return;
    };
    match apagar(&raiz, &rel) {
        Ok(()) => {
            projeto.write().selecionado = None;
            let _ = projeto.write().recarregar();
            status.set(format!("Apagou {rel}"));
        }
        Err(e) => status.set(e),
    }
}

pub fn pasta_selecionada(projeto: State<Projeto>) -> String {
    pasta_do_item(&projeto.read())
}

pub fn conectar_servidor(
    url: &str,
    aluno: &str,
    mut turma: State<Turma>,
    mut prefs: State<Prefs>,
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
) {
    let url = crate::prefs::normalize_url(url);
    let aluno = if aluno.trim().is_empty() {
        "local".to_string()
    } else {
        aluno.trim().to_string()
    };
    turma.write().url = url.clone();
    turma.write().aluno = aluno;
    prefs.write().servidor = url;
    prefs.read().save();
    status.set("Conectando…".into());
    let resultado = turma.write().conectar();
    match resultado {
        Ok(()) => {
            let recado = turma.read().recado.clone();
            dialogo.set(Dialogo::Projetos);
            status.set(recado);
        }
        Err(e) => {
            status.set(format!("Não conectou: {e}"));
        }
    }
}

pub fn abrir_projeto_servidor(
    nome: String,
    editor: State<CodeEditorData>,
    mut prefs: State<Prefs>,
    abas: State<Abas>,
    mut status: State<String>,
    mut projeto: State<Projeto>,
    mut turma: State<Turma>,
    mut dialogo: State<Dialogo>,
) {
    dialogo.set(Dialogo::Nenhum);
    let resultado = {
        let mut t = turma.write();
        let mut p = projeto.write();
        let mut pr = prefs.write();
        t.abrir_projeto(&nome, &mut p, &mut pr)
    };
    match resultado {
        Ok(()) => {
            status.set(format!("Projeto {nome}"));
            restaurar_arquivo(editor, prefs, abas, status);
        }
        Err(e) => status.set(e),
    }
}

pub fn criar_projeto_servidor(
    nome: &str,
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    mut status: State<String>,
    projeto: State<Projeto>,
    mut turma: State<Turma>,
    dialogo: State<Dialogo>,
) {
    let nome = nome.trim();
    if nome.is_empty() {
        status.set("Nome vazio".into());
        return;
    }
    let criado = turma.read().criar_projeto(nome);
    match criado {
        Ok(()) => {
            let lista = turma.read().listar_projetos();
            match lista {
                Ok(lista) => {
                    turma.write().projetos = lista;
                    abrir_projeto_servidor(
                        nome.to_string(),
                        editor,
                        prefs,
                        abas,
                        status,
                        projeto,
                        turma,
                        dialogo,
                    );
                }
                Err(e) => status.set(e),
            }
        }
        Err(e) => status.set(e),
    }
}

pub fn recuperar_rascunhos(
    mut editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
) {
    let Dialogo::Recuperar { itens } = dialogo.read().clone() else {
        return;
    };
    dialogo.set(Dialogo::Nenhum);
    guardar_live(&abas.read(), &mut editor.write());
    for r in itens {
        abas.write().nova_aba(&r.texto);
        {
            let mut abas = abas.write();
            let aba = abas.atual_mut();
            aba.nome = r.nome;
            aba.caminho = r.caminho;
            aba.dirty = true;
        }
    }
    carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
    gravar_rascunhos(&abas.read());
    status.set("Rascunhos recuperados".into());
}

pub fn fechar_dialogo_rascunhos(mut dialogo: State<Dialogo>) {
    dialogo.set(Dialogo::Nenhum);
}

pub fn descartar_rascunhos(mut dialogo: State<Dialogo>, mut status: State<String>) {
    crate::turma::apagar_rascunhos();
    dialogo.set(Dialogo::Nenhum);
    status.set("Rascunhos descartados".into());
}

pub fn resolver_conflito(
    escolha: &str,
    mut editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
    mut dialogo: State<Dialogo>,
    turma: State<Turma>,
) {
    let Dialogo::Conflito {
        nome,
        aba: texto_aba,
        disco,
        revisao_disco,
        remoto: _,
    } = dialogo.read().clone()
    else {
        return;
    };
    match escolha {
        "comparar" => {
            dialogo.set(Dialogo::Comparar {
                nome,
                aba: texto_aba,
                disco,
            });
        }
        "recarregar" => {
            dialogo.set(Dialogo::Nenhum);
            {
                let mut abas = abas.write();
                let atual = abas.atual_mut();
                atual.texto = disco.clone();
                atual.dirty = false;
                atual.revisao = revisao_disco;
            }
            carregar_editor(&mut editor.write(), abas.read().atual(), &prefs.read());
            status.set("Recarregado do disco".into());
        }
        "salvar_como" => {
            dialogo.set(Dialogo::Nenhum);
            let _ = salvar(true, editor, prefs, abas, status, turma, dialogo, None);
        }
        "substituir" => {
            let expected = match revisao_disco {
                Some(r) => Expected::Revision(r),
                None => Expected::Missing,
            };
            dialogo.set(Dialogo::Nenhum);
            let _ = salvar(
                false,
                editor,
                prefs,
                abas,
                status,
                turma,
                dialogo,
                Some(expected),
            );
        }
        _ => {
            dialogo.set(Dialogo::Nenhum);
        }
    }
}

pub fn executar(
    editor: State<CodeEditorData>,
    _prefs: State<Prefs>,
    mut abas: State<Abas>,
    mut status: State<String>,
    projeto: State<Projeto>,
    mut exec: State<Execucao>,
    mut console: State<Console>,
    args: State<String>,
    debug: bool,
    mut depuracao: State<Depuracao>,
    mut graficos: State<Graficos>,
) {
    if exec.read().rodando {
        status.set("Já está rodando — use Parar".into());
        return;
    }
    sincronizar(&mut abas.write(), &editor.read());
    console.write().limpar();
    depuracao.write().fim();
    graficos.write().limpar();
    let fonte = editor.read().rope.to_string();
    let raiz = projeto.read().raiz.clone();
    let rel = caminho_relativo(
        raiz.as_deref().unwrap_or(Path::new(".")),
        abas.read().atual().caminho.as_deref(),
    );
    let mut textos = Vec::new();
    for aba in &abas.read().itens {
        if let Some(p) = &aba.caminho {
            textos.push((
                caminho_relativo(raiz.as_deref().unwrap_or(Path::new(".")), Some(p)),
                aba.texto.clone(),
            ));
        }
    }
    textos.push((rel.clone(), fonte.clone()));
    let ws = match &raiz {
        Some(raiz) => preparar_copia(raiz, &textos),
        None => {
            let dest = pasta_exec();
            let _ = std::fs::remove_dir_all(&dest);
            if let Err(e) = std::fs::create_dir_all(&dest) {
                Err(e.to_string())
            } else if let Err(e) = std::fs::write(dest.join(&rel), &fonte) {
                Err(e.to_string())
            } else {
                Ok(dest)
            }
        }
    };
    let ws = match ws {
        Ok(p) => p,
        Err(e) => {
            status.set(e);
            return;
        }
    };
    let arquivo = ws.join(&rel).display().to_string();
    let argv = argumentos(&args.read());
    let pontos = depuracao.read().pontos.clone();
    if debug {
        depuracao.write().visivel = true;
    }
    exec.write()
        .iniciar(fonte, arquivo, ws, argv, debug, pontos);
    status.set(if debug {
        "Depurando…".into()
    } else {
        "Rodando…".into()
    });
}

fn comando_debug(
    cmd: &str,
    mut exec: State<Execucao>,
    mut status: State<String>,
    mut depuracao: State<Depuracao>,
) {
    if !exec.read().rodando {
        status.set("Nada em execução".into());
        return;
    }
    exec.write().comando(cmd);
    depuracao.write().pausado = false;
    status.set(match cmd {
        "proximo" => "Próxima linha…".into(),
        "entrar" => "Entrando…".into(),
        "sair" => "Saindo da função…".into(),
        _ => "Continuando…".into(),
    });
}

fn alternar_ponto(
    editor: State<CodeEditorData>,
    _prefs: State<Prefs>,
    abas: State<Abas>,
    mut status: State<String>,
    projeto: State<Projeto>,
    mut exec: State<Execucao>,
    mut depuracao: State<Depuracao>,
) {
    let linha = (editor.read().cursor_row() + 1) as u32;
    let rel = caminho_relativo(
        projeto.read().raiz.as_deref().unwrap_or(Path::new(".")),
        abas.read().atual().caminho.as_deref(),
    );
    let ligado = depuracao.write().alternar_ponto(rel.clone(), linha);
    if exec.read().rodando {
        let cmd = if ligado {
            format!("ponto {rel}:{linha}")
        } else {
            format!("remover {rel}:{linha}")
        };
        exec.write().comando(&cmd);
    }
    status.set(if ligado {
        format!("Ponto em {rel}:{linha}")
    } else {
        format!("Removeu ponto {rel}:{linha}")
    });
}

pub fn adicionar_observado(nome: &str, mut depuracao: State<Depuracao>, mut campo: State<String>) {
    depuracao.write().adicionar_observado(nome);
    campo.set(String::new());
}

pub fn remover_observado(nome: String, mut depuracao: State<Depuracao>) {
    depuracao.write().remover_observado(&nome);
}

pub fn enviar_repl(
    mut repl: State<Repl>,
    mut entrada: State<String>,
    mut status: State<String>,
) {
    let linha = entrada.read().clone();
    match repl.write().enviar(&linha) {
        Ok(()) => {
            entrada.set(String::new());
            status.set("REPL".into());
        }
        Err(e) => status.set(e),
    }
}

pub fn parar_repl(repl: State<Repl>, mut status: State<String>) {
    repl.read().parar();
    status.set("Parando REPL…".into());
}

pub fn limpar_repl(mut repl: State<Repl>) {
    repl.write().limpar();
}

pub fn reiniciar_repl(mut repl: State<Repl>, mut status: State<String>) {
    repl.write().reiniciar();
    status.set("REPL reiniciado".into());
}

pub fn fechar_repl(mut repl: State<Repl>) {
    repl.write().visivel = false;
}

pub fn destacar_graficos(mut graficos: State<Graficos>) {
    let v = !graficos.read().destacado;
    graficos.write().destacado = v;
}

pub fn fechar_tela_graf(
    id: u64,
    mut graficos: State<Graficos>,
    exec: State<Execucao>,
) {
    let ev = graficos.write().fechar_tela(id);
    exec.read().evento_graf(ev);
}

pub fn clique_tela_graf(
    id: u64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    graficos: State<Graficos>,
    exec: State<Execucao>,
) {
    let ev = graficos
        .read()
        .telas
        .iter()
        .find(|t| t.id == id)
        .map(|t| crate::grafico::clique_na_tela(t, x, y, w, h));
    if let Some(ev) = ev {
        exec.read().evento_graf(ev);
    }
}

pub fn aplicar_eventos(
    eventos: Vec<EventoExec>,
    mut console: State<Console>,
    mut status: State<String>,
    mut exec: State<Execucao>,
    mut depuracao: State<Depuracao>,
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    projeto: State<Projeto>,
    dialogo: State<Dialogo>,
    mut graficos: State<Graficos>,
) {
    for ev in eventos {
        match ev {
            EventoExec::Saida { texto, stderr } => console.write().escrever(&texto, stderr),
            EventoExec::Leia(prompt) => {
                console.write().leia_prompt = Some(prompt);
                console.write().entrada.clear();
            }
            EventoExec::Pausado {
                arquivo,
                linha,
                trecho,
                vars,
                pilha,
                erro,
            } => {
                depuracao.write().aplicar_pausa(
                    arquivo.clone(),
                    linha,
                    trecho.clone(),
                    vars,
                    pilha,
                    erro.clone(),
                );
                status.set(match &erro {
                    Some(e) => format!("erro em {arquivo}:{linha}: {e}"),
                    None => format!("pausado {arquivo}:{linha}  {}", trecho.trim()),
                });
                if linha > 0 {
                    ir_para_pausa(
                        &arquivo,
                        linha,
                        editor,
                        prefs,
                        abas,
                        status,
                        projeto,
                        dialogo,
                    );
                }
            }
            EventoExec::Folha(folha) => {
                graficos.write().aplicar_folha(folha);
            }
            EventoExec::TelaGraf(tela) => {
                graficos.write().aplicar_tela(tela);
            }
            EventoExec::Fim {
                ok,
                erro,
                arquivo,
                linha,
                pilha,
            } => {
                exec.write().rodando = false;
                depuracao.write().fim();
                if ok {
                    status.set("Terminou".into());
                } else {
                    if !erro.is_empty() {
                        console.write().escrever(&format!("{erro}\n"), true);
                    }
                    for p in pilha {
                        console.write().escrever(&format!("{p}\n"), true);
                    }
                    if linha > 0 {
                        console
                            .write()
                            .escrever(&format!(" em {arquivo}:{linha}\n"), true);
                    }
                    status.set("Erro na execução".into());
                }
            }
        }
    }
}

pub fn responder_leia(
    mut exec: State<Execucao>,
    mut console: State<Console>,
    entrada: State<String>,
) {
    let linha = entrada.read().clone();
    exec.write().responder_leia(linha);
    console.write().leia_prompt = None;
    let mut entrada = entrada;
    entrada.set(String::new());
}

pub fn ir_para_pausa(
    arquivo: &str,
    linha: u32,
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    status: State<String>,
    projeto: State<Projeto>,
    dialogo: State<Dialogo>,
) {
    let path = {
        let raiz = projeto.read().raiz.clone();
        let p = PathBuf::from(arquivo);
        if p.is_file() {
            p
        } else if let Some(raiz) = raiz {
            raiz.join(arquivo)
        } else {
            p
        }
    };
    if path.is_file() {
        abrir_caminho(path, editor, prefs, abas, status);
    }
    ir_para_linha(&linha.to_string(), editor, status, dialogo);
}

pub fn ir_para_saida(
    linha: &str,
    editor: State<CodeEditorData>,
    prefs: State<Prefs>,
    abas: State<Abas>,
    status: State<String>,
    projeto: State<Projeto>,
    dialogo: State<Dialogo>,
) {
    let Some((arquivo, n)) = parse_error_location(linha) else {
        return;
    };
    let path = {
        let raiz = projeto.read().raiz.clone();
        let p = PathBuf::from(&arquivo);
        if p.is_file() {
            p
        } else if let Some(raiz) = raiz {
            raiz.join(&arquivo)
        } else {
            p
        }
    };
    if path.is_file() {
        abrir_caminho(path, editor, prefs, abas, status);
        ir_para_linha(&n.to_string(), editor, status, dialogo);
    }
}

pub fn mostrar_ajuda(
    editor: State<CodeEditorData>,
    mut status: State<String>,
    mut ajuda: State<Option<String>>,
    mut catalogo: State<bool>,
) {
    let texto = editor.read().rope.to_string();
    let byte = crate::editor::byte_do_cursor(&editor.read());
    let nome = palavra_em(&texto, byte);
    match ficha(&nome) {
        Some(f) => {
            ajuda.set(Some(f));
            status.set(format!("Ajuda: {nome}"));
        }
        None => {
            catalogo.set(true);
            status.set("Coloque o cursor num nome de função (F1)".into());
        }
    }
}

pub fn aplicar_sugestao(
    s: Sugestao,
    mut editor: State<CodeEditorData>,
    mut sugestoes: State<Vec<Sugestao>>,
) {
    let texto = editor.read().rope.to_string();
    let byte = crate::editor::byte_do_cursor(&editor.read());
    let prefix = prefixo_em(&texto, byte);
    let start = byte.saturating_sub(prefix.len());
    let mut novo = String::new();
    novo.push_str(&texto[..start]);
    novo.push_str(&s.inserir);
    novo.push_str(&texto[byte..]);
    if let Some(m) = &s.importe
        && let Some(com) = garantir_importe(&novo, m)
    {
        novo = com;
    }
    {
        let mut ed = editor.write();
        crate::editor::substituir_texto(&mut ed, &novo);
        let cursor = start + s.inserir.encode_utf16().count();
        let max = ed.len_utf16_cu();
        ed.move_cursor_to(cursor.min(max));
    }
    sugestoes.set(Vec::new());
}

fn ir_definicao(mut editor: State<CodeEditorData>, mut status: State<String>) {
    let texto = editor.read().rope.to_string();
    let byte = crate::editor::byte_do_cursor(&editor.read());
    let nome = palavra_em(&texto, byte);
    if nome.is_empty() {
        status.set("Nenhuma definição".into());
        return;
    }
    for (i, linha) in texto.lines().enumerate() {
        let t = linha.trim_start();
        if (t.starts_with(&format!("{nome} =")) || t.starts_with(&format!("{nome}=")))
            && let Some(pos) = crate::documentos::posicao_linha(&texto, i + 1)
        {
            editor.write().move_cursor_to(pos);
            status.set(format!("Definição de {nome}"));
            return;
        }
    }
    status.set(format!("Não achou definição de {nome}"));
}

pub fn achar(
    para_frente: bool,
    mut editor: State<CodeEditorData>,
    mut status: State<String>,
    busca_termo: State<String>,
) {
    if let Some(msg) = buscar(&mut editor.write(), &busca_termo.read(), para_frente) {
        status.set(msg);
    } else {
        status.set(String::new());
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
    trocar_aba(&mut editor.write(), &mut abas.write(), idx, &prefs.read());
}
