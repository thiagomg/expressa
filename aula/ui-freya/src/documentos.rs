//! Open tabs, dirty flag, save, and go-to-line. No GTK.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::editor::{fonte_familia, fonte_px, new_editor};
use crate::prefs::Prefs;
use crate::shell::aplicar_tema_editor;
use freya::code_editor::CodeEditorData;
use freya::text_edit::{Rope, TextEditor};

const LIMITE_TEXTO: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aba {
    pub id: u64,
    pub nome: String,
    pub caminho: Option<PathBuf>,
    pub texto: String,
    pub dirty: bool,
    pub revisao: Option<String>,
    pub rel_remoto: Option<String>,
    pub cursor: usize,
    pub selecao: Option<(usize, usize)>,
    pub snapshot: bool,
}

impl Aba {
    pub fn vazia(id: u64, texto: &str) -> Self {
        Self {
            id,
            nome: "sem título".into(),
            caminho: None,
            texto: texto.to_string(),
            dirty: false,
            revisao: None,
            rel_remoto: None,
            cursor: 0,
            selecao: None,
            snapshot: false,
        }
    }
}

#[derive(Clone, Default)]
struct Editores(Rc<RefCell<HashMap<u64, CodeEditorData>>>);

impl PartialEq for Editores {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Editores {}
impl std::fmt::Debug for Editores {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Editores")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abas {
    pub itens: Vec<Aba>,
    pub atual: usize,
    proximo_id: u64,
    editores: Editores,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fechar {
    Fecha,
    Confirma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depois {
    SoFechar,
    Sair,
    Abrir,
    Novo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepoisNome {
    NovoArquivo(String),
    NovaPasta(String),
    Renomear(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Dialogo {
    #[default]
    Nenhum,
    Confirmar {
        id: u64,
        depois: Depois,
    },
    IrParaLinha,
    Nome {
        titulo: String,
        depois: DepoisNome,
    },
    Apagar {
        rel: String,
    },
    Servidor,
    Projetos,
    Recuperar {
        itens: Vec<Rascunho>,
    },
    Conflito {
        nome: String,
        aba: String,
        disco: String,
        revisao_disco: Option<String>,
        remoto: bool,
    },
    Comparar {
        nome: String,
        aba: String,
        disco: String,
    },
}

impl Abas {
    pub fn nova(texto: &str) -> Self {
        Self {
            itens: vec![Aba::vazia(1, texto)],
            atual: 0,
            proximo_id: 2,
            editores: Editores::default(),
        }
    }

    pub fn atual(&self) -> &Aba {
        &self.itens[self.atual]
    }

    pub fn atual_mut(&mut self) -> &mut Aba {
        &mut self.itens[self.atual]
    }

    pub fn rotulo(aba: &Aba) -> String {
        let marca = if aba.dirty { "● " } else { "" };
        let extra = if aba.snapshot { " 🔒" } else { "" };
        format!("{marca}{}{extra}", aba.nome)
    }

    pub fn nova_aba(&mut self, texto: &str) {
        let id = self.proximo_id;
        self.proximo_id += 1;
        self.itens.push(Aba::vazia(id, texto));
        self.atual = self.itens.len() - 1;
    }

    pub fn achar_caminho(&self, caminho: &Path) -> Option<usize> {
        self.itens
            .iter()
            .position(|a| a.caminho.as_deref() == Some(caminho))
    }

    pub fn ir_para(&mut self, idx: usize) {
        if idx < self.itens.len() {
            self.atual = idx;
        }
    }

    pub fn fechar_id(&mut self, id: u64) -> Fechar {
        let Some(idx) = self.itens.iter().position(|a| a.id == id) else {
            return Fechar::Fecha;
        };
        if self.itens[idx].dirty {
            return Fechar::Confirma;
        }
        self.remover(idx);
        Fechar::Fecha
    }

    pub fn remover(&mut self, idx: usize) {
        let id = self.itens.get(idx).map(|a| a.id);
        if let Some(id) = id {
            self.editores.0.borrow_mut().remove(&id);
        }
        if self.itens.len() == 1 {
            self.itens[0] = Aba::vazia(self.proximo_id, "");
            self.proximo_id += 1;
            self.atual = 0;
            return;
        }
        self.itens.remove(idx);
        if self.atual >= self.itens.len() {
            self.atual = self.itens.len() - 1;
        } else if idx < self.atual {
            self.atual -= 1;
        }
    }

    pub fn alguma_suja(&self) -> bool {
        self.itens.iter().any(|a| a.dirty)
    }

    pub fn proxima_suja(&self) -> Option<u64> {
        self.itens.iter().find(|a| a.dirty && !a.snapshot).map(|a| a.id)
    }

    pub fn idx_snapshot(&self) -> Option<usize> {
        self.itens.iter().position(|a| a.snapshot)
    }

    pub fn fechar_snapshots(&mut self) {
        let ids: Vec<u64> = self.itens.iter().filter(|a| a.snapshot).map(|a| a.id).collect();
        for id in ids {
            let _ = self.fechar_id(id);
        }
    }

    pub fn garantir_snapshot(&mut self, nome: &str, texto: &str, caminho: Option<PathBuf>) -> usize {
        if let Some(idx) = self.idx_snapshot() {
            self.itens[idx].nome = format!("{nome} (execução)");
            self.itens[idx].texto = texto.to_string();
            self.itens[idx].caminho = caminho;
            self.itens[idx].dirty = false;
            self.itens[idx].cursor = 0;
            self.atual = idx;
            return idx;
        }
        self.nova_aba(texto);
        let aba = self.atual_mut();
        aba.nome = format!("{nome} (execução)");
        aba.caminho = caminho;
        aba.dirty = false;
        aba.snapshot = true;
        self.atual
    }

    pub fn vizinha(&self, passo: i32) -> usize {
        let n = self.itens.len() as i32;
        if n == 0 {
            return self.atual;
        }
        (self.atual as i32 + passo).rem_euclid(n) as usize
    }

    pub fn proxima(&mut self, passo: i32) {
        self.atual = self.vizinha(passo);
    }
}

/// OS window title: `Expressa Aula — nome.lep *` when the current tab is dirty.
pub fn titulo_da_janela(aba: &Aba) -> String {
    if aba.dirty {
        format!("Expressa Aula — {} *", aba.nome)
    } else {
        format!("Expressa Aula — {}", aba.nome)
    }
}

/// 1-based line → UTF-16 cursor at the start of that line.
pub fn posicao_linha(texto: &str, linha: usize) -> Option<usize> {
    if linha == 0 {
        return None;
    }
    let alvo = linha - 1;
    let mut start = 0usize;
    for (i, trecho) in texto.split_inclusive('\n').enumerate() {
        if i == alvo {
            return Some(texto[..start].encode_utf16().count());
        }
        start += trecho.len();
    }
    None
}

pub fn garantir_lep(path: PathBuf) -> PathBuf {
    match path.extension() {
        Some(_) => path,
        None => path.with_extension("lep"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rascunho {
    pub nome: String,
    pub caminho: Option<PathBuf>,
    pub texto: String,
}

pub fn pasta_rascunho() -> PathBuf {
    dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("expressa-aula")
        .join("rascunho-freya")
}

fn ficheiro_rascunho(dir: &Path, id: u64) -> PathBuf {
    dir.join(format!("{id}.rascunho"))
}

pub fn apagar_rascunho(id: u64) {
    let dir = pasta_rascunho();
    let _ = std::fs::remove_file(ficheiro_rascunho(&dir, id));
    let _ = std::fs::remove_file(dir.join(format!("{id}.lep")));
}

pub fn gravar_rascunhos(abas: &Abas) {
    let dir = pasta_rascunho();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("rascunho: {e}");
        return;
    }
    for aba in &abas.itens {
        if !aba.dirty || aba.snapshot {
            apagar_rascunho(aba.id);
            continue;
        }
        let caminho = aba
            .caminho
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let corpo = format!(
            "#expressa-rascunho\nnome: {}\ncaminho: {}\n\n{}",
            aba.nome.replace('\n', " "),
            caminho,
            aba.texto
        );
        let dest = ficheiro_rascunho(&dir, aba.id);
        if let Err(e) = std::fs::write(&dest, corpo) {
            eprintln!("rascunho {}: {e}", aba.id);
        }
    }
}

pub fn ler_rascunho(path: &Path, nome_ficheiro: &str) -> Option<Rascunho> {
    let bruto = std::fs::read_to_string(path).ok()?;
    if bruto.trim().is_empty() {
        return None;
    }
    Some(parse_rascunho(&bruto, nome_ficheiro))
}

pub fn parse_rascunho(bruto: &str, nome_ficheiro: &str) -> Rascunho {
    if let Some(rest) = bruto.strip_prefix("#expressa-rascunho\n") {
        let mut nome = nome_ficheiro.to_string();
        let mut caminho = None;
        let mut corpo = rest;
        while let Some((linha, resto)) = corpo.split_once('\n') {
            if linha.is_empty() {
                corpo = resto;
                break;
            }
            if let Some(v) = linha.strip_prefix("nome: ") {
                nome = v.to_string();
            } else if let Some(v) = linha.strip_prefix("caminho: ")
                && !v.is_empty()
            {
                caminho = Some(PathBuf::from(v));
            }
            corpo = resto;
        }
        Rascunho {
            nome,
            caminho,
            texto: corpo.to_string(),
        }
    } else {
        Rascunho {
            nome: nome_ficheiro.to_string(),
            caminho: None,
            texto: bruto.to_string(),
        }
    }
}

pub fn ler_arquivo(path: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > LIMITE_TEXTO {
        return Err("arquivo maior que 8 MiB".into());
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn revisao_de_arquivo(path: &Path) -> Option<String> {
    expressa_aula_storage::read(path)
        .ok()
        .map(|s| s.revision)
}

pub fn expected_da_aba(aba: &Aba, path: &Path) -> expressa_aula_storage::Expected {
    match &aba.revisao {
        Some(r) => expressa_aula_storage::Expected::Revision(r.clone()),
        None if !path.exists() => expressa_aula_storage::Expected::Missing,
        None => match revisao_de_arquivo(path) {
            Some(r) => expressa_aula_storage::Expected::Revision(r),
            None => expressa_aula_storage::Expected::Missing,
        },
    }
}

pub async fn escolher_abrir() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("Expressa", &["lep"])
        .add_filter("Todos", &["*"])
        .pick_file()
        .await
        .map(|h| h.path().to_path_buf())
}

pub async fn escolher_salvar(sugerido: &str) -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("Expressa", &["lep"])
        .set_file_name(sugerido)
        .save_file()
        .await
        .map(|h| garantir_lep(h.path().to_path_buf()))
}

pub fn sincronizar(abas: &mut Abas, editor: &CodeEditorData) {
    let aba = abas.atual_mut();
    aba.texto = editor.rope.to_string();
    aba.dirty = editor.is_edited();
    aba.cursor = editor.cursor_pos();
    aba.selecao = editor.get_selection();
}

/// Load from disk / new file: new buffer, no undo from a previous tab.
pub fn carregar_editor(editor: &mut CodeEditorData, aba: &Aba, prefs: &Prefs) {
    *editor = new_editor(&aba.texto);
    aplicar_tema_editor(editor, prefs.dark);
    editor.measure(fonte_px(prefs.font_size), fonte_familia());
    let max = editor.len_utf16_cu();
    editor.move_cursor_to(aba.cursor.min(max));
    if let Some(sel) = aba.selecao {
        editor.set_selection(sel);
    }
    if !aba.dirty {
        editor.mark_as_saved();
    }
}

pub fn guardar_editor(abas: &Abas, id: u64, editor: CodeEditorData) {
    abas.editores.0.borrow_mut().insert(id, editor);
}

pub fn guardar_live(abas: &Abas, editor: &mut CodeEditorData) {
    let id = abas.atual().id;
    let live = std::mem::replace(editor, dummy_editor());
    guardar_editor(abas, id, live);
}

fn dummy_editor() -> CodeEditorData {
    CodeEditorData::new(Rope::from_str(""), None)
}

/// Switch tab without rebuilding the live buffer when we already have one.
pub fn trocar_aba(editor: &mut CodeEditorData, abas: &mut Abas, idx: usize, prefs: &Prefs) {
    if idx == abas.atual || idx >= abas.itens.len() {
        return;
    }
    let de = abas.atual().id;
    sincronizar(abas, editor);
    let live = std::mem::replace(editor, dummy_editor());
    guardar_editor(abas, de, live);
    abas.ir_para(idx);
    let aba = abas.atual().clone();
    instalar_editor(editor, abas, &aba, prefs);
}

pub fn instalar_editor(editor: &mut CodeEditorData, abas: &Abas, aba: &Aba, prefs: &Prefs) {
    if let Some(guardado) = abas.editores.0.borrow_mut().remove(&aba.id) {
        *editor = guardado;
        aplicar_tema_editor(editor, prefs.dark);
        editor.measure(fonte_px(prefs.font_size), fonte_familia());
    } else {
        carregar_editor(editor, aba, prefs);
    }
}

pub fn nome_de_caminho(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotulo_marca_alterado() {
        let mut a = Aba {
            id: 1,
            nome: "a.lep".into(),
            caminho: None,
            texto: String::new(),
            dirty: false,
            revisao: None,
            rel_remoto: None,
            cursor: 0,
            selecao: None,
            snapshot: false,
        };
        assert_eq!(Abas::rotulo(&a), "a.lep");
        a.dirty = true;
        assert_eq!(Abas::rotulo(&a), "● a.lep");
        a.snapshot = true;
        a.dirty = false;
        assert_eq!(Abas::rotulo(&a), "a.lep 🔒");
    }

    #[test]
    fn titulo_mostra_estrela_quando_sujo() {
        let mut a = Aba::vazia(1, "");
        a.nome = "poema.lep".into();
        assert_eq!(titulo_da_janela(&a), "Expressa Aula — poema.lep");
        a.dirty = true;
        assert_eq!(titulo_da_janela(&a), "Expressa Aula — poema.lep *");
    }

    #[test]
    fn fechar_ultima_reabre_vazia() {
        let mut abas = Abas::nova("x");
        assert_eq!(abas.fechar_id(1), Fechar::Fecha);
        assert_eq!(abas.itens.len(), 1);
        assert_eq!(abas.atual().texto, "");
        assert_eq!(abas.atual().nome, "sem título");
    }

    #[test]
    fn fechar_suja_pede_confirmacao() {
        let mut abas = Abas::nova("x");
        abas.atual_mut().dirty = true;
        assert_eq!(abas.fechar_id(1), Fechar::Confirma);
        assert_eq!(abas.itens.len(), 1);
    }

    #[test]
    fn proxima_cicla_as_abas() {
        let mut abas = Abas::nova("a");
        abas.nova_aba("b");
        abas.nova_aba("c");
        abas.atual = 0;
        assert_eq!(abas.vizinha(1), 1);
        abas.proxima(1);
        assert_eq!(abas.atual, 1);
        abas.proxima(1);
        abas.proxima(1);
        assert_eq!(abas.atual, 0);
        abas.proxima(-1);
        assert_eq!(abas.atual, 2);
    }

    #[test]
    fn proxima_suja_percorre_todas() {
        let mut abas = Abas::nova("a");
        abas.atual_mut().dirty = true;
        abas.nova_aba("b");
        abas.atual_mut().dirty = true;
        assert!(abas.alguma_suja());
        assert_eq!(abas.proxima_suja(), Some(1));
        abas.itens[0].dirty = false;
        assert_eq!(abas.proxima_suja(), Some(2));
        abas.itens[1].dirty = false;
        assert!(!abas.alguma_suja());
        assert_eq!(abas.proxima_suja(), None);
    }

    #[test]
    fn snapshot_nao_marca_sujo_e_reusa() {
        let mut abas = Abas::nova("x = 1\n");
        let idx = abas.garantir_snapshot("a.lep", "x = 1\n", None);
        assert!(abas.itens[idx].snapshot);
        assert!(!abas.itens[idx].dirty);
        assert!(Abas::rotulo(abas.atual()).contains("execução"));
        let idx2 = abas.garantir_snapshot("a.lep", "x = 2\n", None);
        assert_eq!(idx, idx2);
        assert_eq!(abas.atual().texto, "x = 2\n");
        abas.fechar_snapshots();
        assert!(abas.itens.iter().all(|a| !a.snapshot));
    }

    #[test]
    fn posicao_linha_primeira_e_segunda() {
        let t = "um\ndois\ntrês";
        assert_eq!(posicao_linha(t, 1), Some(0));
        assert_eq!(posicao_linha(t, 2), Some("um\n".encode_utf16().count()));
        assert_eq!(posicao_linha(t, 0), None);
        assert_eq!(posicao_linha(t, 9), None);
    }

    #[test]
    fn lep_quando_falta_extensao() {
        assert_eq!(garantir_lep(PathBuf::from("poema")), PathBuf::from("poema.lep"));
        assert_eq!(garantir_lep(PathBuf::from("a.lep")), PathBuf::from("a.lep"));
    }

    #[test]
    fn reusa_aba_pelo_caminho() {
        let mut abas = Abas::nova("");
        abas.atual_mut().caminho = Some(PathBuf::from("/tmp/a.lep"));
        abas.nova_aba("");
        assert_eq!(abas.achar_caminho(Path::new("/tmp/a.lep")), Some(0));
        assert_eq!(abas.achar_caminho(Path::new("/tmp/b.lep")), None);
    }

    #[test]
    fn expected_usa_revisao_da_aba() {
        let mut a = Aba::vazia(1, "x");
        a.revisao = Some("sha256:abc".into());
        match expected_da_aba(&a, Path::new("/nao/existe.lep")) {
            expressa_aula_storage::Expected::Revision(r) => assert_eq!(r, "sha256:abc"),
            other => panic!("{other:?}"),
        }
        let vazia = Aba::vazia(2, "");
        assert!(matches!(
            expected_da_aba(&vazia, Path::new("/nao/existe.lep")),
            expressa_aula_storage::Expected::Missing
        ));
    }

    #[test]
    fn rascunho_guarda_nome_e_caminho() {
        let r = parse_rascunho(
            "#expressa-rascunho\nnome: poema.lep\ncaminho: /tmp/poema.lep\n\nescreva(1)\n",
            "9.rascunho",
        );
        assert_eq!(r.nome, "poema.lep");
        assert_eq!(r.caminho.as_deref(), Some(Path::new("/tmp/poema.lep")));
        assert_eq!(r.texto, "escreva(1)\n");
    }

    #[test]
    fn gravar_rascunho_some_depois_de_salvar() {
        let dir = pasta_rascunho();
        let _ = std::fs::create_dir_all(&dir);
        let mut abas = Abas::nova("escreva(1)\n");
        abas.atual_mut().id = 4242;
        abas.atual_mut().nome = "poema.lep".into();
        abas.atual_mut().caminho = Some(PathBuf::from("/tmp/poema.lep"));
        abas.atual_mut().dirty = true;
        gravar_rascunhos(&abas);
        let lido = ler_rascunho(&dir.join("4242.rascunho"), "4242.rascunho").unwrap();
        assert_eq!(lido.nome, "poema.lep");
        assert_eq!(lido.texto, "escreva(1)\n");
        abas.atual_mut().dirty = false;
        gravar_rascunhos(&abas);
        assert!(ler_rascunho(&dir.join("4242.rascunho"), "4242.rascunho").is_none());
    }

    #[test]
    fn trocar_aba_restaura_cursor_e_desfazer() {
        let mut abas = Abas::nova("aaa");
        let mut ed = CodeEditorData::new(Rope::from_str("aaa"), None);
        ed.move_cursor_to(2);
        let n = ed.insert("X", 2);
        ed.move_cursor_to(2 + n);
        sincronizar(&mut abas, &ed);
        guardar_live(&abas, &mut ed);
        abas.nova_aba("bbb");
        ed = CodeEditorData::new(Rope::from_str("bbb"), None);
        ed.move_cursor_to(1);
        sincronizar(&mut abas, &ed);
        guardar_live(&abas, &mut ed);
        abas.ir_para(0);
        let aba = abas.atual().clone();
        let guardado = abas.editores.0.borrow_mut().remove(&aba.id).unwrap();
        ed = guardado;
        assert_eq!(ed.rope.to_string(), "aaXa");
        assert_eq!(ed.cursor_pos(), 3);
        assert!(ed.undo().is_some());
        assert_eq!(ed.rope.to_string(), "aaa");
    }
}
