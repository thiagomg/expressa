//! Open tabs, dirty flag, save, and go-to-line. No GTK.

use std::path::{Path, PathBuf};

use crate::editor::{FONT_FAMILY, fonte_px, new_editor};
use crate::prefs::Prefs;
use crate::shell::aplicar_tema_editor;
use freya::code_editor::CodeEditorData;

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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abas {
    pub itens: Vec<Aba>,
    pub atual: usize,
    proximo_id: u64,
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
        itens: Vec<(String, String)>,
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
        format!("{marca}{}", aba.nome)
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

    #[allow(dead_code)]
    pub fn proxima(&mut self, passo: i32) {
        let n = self.itens.len() as i32;
        if n == 0 {
            return;
        }
        self.atual = (self.atual as i32 + passo).rem_euclid(n) as usize;
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

pub fn pasta_rascunho() -> PathBuf {
    dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("expressa-aula")
        .join("rascunho-freya")
}

pub fn gravar_rascunhos(abas: &Abas) {
    let dir = pasta_rascunho();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("rascunho: {e}");
        return;
    }
    for aba in &abas.itens {
        if !aba.dirty {
            continue;
        }
        let dest = dir.join(format!("{}.lep", aba.id));
        if let Err(e) = std::fs::write(&dest, &aba.texto) {
            eprintln!("rascunho {}: {e}", aba.id);
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

pub fn escolher_abrir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Expressa", &["lep"])
        .add_filter("Todos", &["*"])
        .pick_file()
}

pub fn escolher_salvar(sugerido: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Expressa", &["lep"])
        .set_file_name(sugerido)
        .save_file()
        .map(garantir_lep)
}

pub fn sincronizar(abas: &mut Abas, editor: &CodeEditorData) {
    let aba = abas.atual_mut();
    aba.texto = editor.rope.to_string();
    aba.dirty = editor.is_edited();
}

pub fn carregar_editor(editor: &mut CodeEditorData, aba: &Aba, prefs: &Prefs) {
    *editor = new_editor(&aba.texto);
    aplicar_tema_editor(editor, prefs.dark);
    editor.measure(fonte_px(prefs.font_size), FONT_FAMILY);
    if !aba.dirty {
        editor.mark_as_saved();
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
        };
        assert_eq!(Abas::rotulo(&a), "a.lep");
        a.dirty = true;
        assert_eq!(Abas::rotulo(&a), "● a.lep");
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
}
