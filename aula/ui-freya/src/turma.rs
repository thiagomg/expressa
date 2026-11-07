//! Classroom Turma RPC for the Freya editor. GTK `aula/ui` stays untouched.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use expressa_aula_proto::turma_client::TurmaClient;
use expressa_aula_proto::{
    CreateProjectRequest, DeleteFileRequest, ListFilesRequest, ListProjectsRequest, MkdirRequest,
    ReadFileRequest, RenameRequest, WriteFileRequest,
};
use crate::prefs::{Prefs, Recente};
use crate::projeto::{Projeto, nos_de_entradas};
use expressa_aula_storage::{Expected, SaveError, Saved};
use tonic::transport::{Channel, Endpoint};

/// Process this UI started. Drop kills it; a server we only connected to is left alone.
struct ServidorFilho {
    child: Mutex<Option<Child>>,
}

impl Default for ServidorFilho {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }
}

impl Drop for ServidorFilho {
    fn drop(&mut self) {
        if let Ok(mut g) = self.child.lock() {
            parar_servidor(&mut g);
        }
    }
}

fn parar_servidor(slot: &mut Option<Child>) {
    let Some(mut child) = slot.take() else {
        return;
    };
    let pid = child.id();
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
        std::thread::sleep(Duration::from_millis(80));
        let ainda = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ainda {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[derive(Clone)]
pub struct Turma {
    canal: Arc<Mutex<Option<Channel>>>,
    spawned: Arc<ServidorFilho>,
    pub url: String,
    pub aluno: String,
    pub projeto: String,
    pub projetos: Vec<String>,
    pub conectado: bool,
    pub recado: String,
}

impl Default for Turma {
    fn default() -> Self {
        Self {
            canal: Arc::new(Mutex::new(None)),
            spawned: Arc::new(ServidorFilho::default()),
            url: crate::prefs::DEFAULT_URL.to_string(),
            aluno: "local".into(),
            projeto: String::new(),
            projetos: Vec::new(),
            conectado: false,
            recado: String::new(),
        }
    }
}

impl PartialEq for Turma {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
            && self.aluno == other.aluno
            && self.projeto == other.projeto
            && self.projetos == other.projetos
            && self.conectado == other.conectado
            && self.recado == other.recado
    }
}

impl Eq for Turma {}

fn esperar<T: Send + 'static>(
    fut: impl std::future::Future<Output = T> + Send + 'static,
) -> Result<T, String> {
    let handle = tokio::runtime::Handle::try_current().map_err(|e| e.to_string())?;
    std::thread::spawn(move || handle.block_on(fut))
        .join()
        .map_err(|_| "falha na thread RPC".to_string())
}

fn binario_servidor() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("expressa-aula-freya"));
    let nome = if cfg!(windows) {
        "expressa-aula-server.exe"
    } else {
        "expressa-aula-server"
    };
    if let Some(dir) = exe.parent() {
        let p = dir.join(nome);
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from(nome)
}

pub fn pasta_aula_data() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("expressa-aula")
        .join("aula-data")
}

pub fn pasta_cache_remoto(aluno: &str, projeto: &str) -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("expressa-aula")
        .join("remoto")
        .join(aluno)
        .join(projeto)
}

impl Turma {
    pub fn garantir_servidor_local(&mut self) -> Result<(), String> {
        if !self.url.contains("127.0.0.1") && !self.url.contains("localhost") {
            return Ok(());
        }
        if self.tentar_conectar().is_ok() {
            return Ok(());
        }
        let bin = binario_servidor();
        if !bin.is_file() {
            return Err(format!(
                "não achou {bin}. Compile expressa-aula-server ou suba o servidor.",
                bin = bin.display()
            ));
        }
        let root = pasta_aula_data();
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let child = Command::new(&bin)
            .args([
                "--bind",
                "127.0.0.1:50051",
                "--root",
                &root.display().to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("não subiu o servidor: {e}"))?;
        *self.spawned.child.lock().unwrap() = Some(child);
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(50));
            if self.tentar_conectar().is_ok() {
                return Ok(());
            }
        }
        Err("servidor local não respondeu".into())
    }

    fn tentar_conectar(&mut self) -> Result<(), String> {
        let url = self.url.clone();
        let channel = esperar(async move {
            Endpoint::from_shared(url)
                .map_err(|e| e.to_string())?
                .connect_timeout(Duration::from_secs(2))
                .timeout(Duration::from_secs(8))
                .connect()
                .await
                .map_err(|e| e.to_string())
        })??;
        *self.canal.lock().unwrap() = Some(channel);
        self.conectado = true;
        Ok(())
    }

    pub fn conectar(&mut self) -> Result<(), String> {
        self.garantir_servidor_local()?;
        if !self.conectado {
            self.tentar_conectar()?;
        }
        self.projetos = self.listar_projetos()?;
        self.recado = format!("conectado {} ({})", self.url, self.aluno);
        Ok(())
    }

    pub fn aplicar_de(&mut self, outra: Turma) {
        self.canal = outra.canal;
        self.spawned = outra.spawned;
        self.url = outra.url;
        self.aluno = outra.aluno;
        self.projeto = outra.projeto;
        self.projetos = outra.projetos;
        self.conectado = outra.conectado;
        self.recado = outra.recado;
    }

    fn canal(&self) -> Result<Channel, String> {
        self.canal
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| "não conectado".to_string())
    }

    pub fn listar_projetos(&self) -> Result<Vec<String>, String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            c.list_projects(ListProjectsRequest { student: aluno })
                .await
                .map(|r| r.into_inner().projects)
                .map_err(|e| e.message().to_string())
        })?
    }

    pub fn criar_projeto(&self, nome: &str) -> Result<(), String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        let nome = nome.to_string();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            c.create_project(CreateProjectRequest {
                student: aluno,
                project: nome,
            })
            .await
            .map(|_| ())
            .map_err(|e| e.message().to_string())
        })?
    }

    pub fn listar_arquivos(&self) -> Result<Vec<(String, bool)>, String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        let projeto = self.projeto.clone();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            c.list_files(ListFilesRequest {
                student: aluno,
                project: projeto,
            })
            .await
            .map(|r| {
                r.into_inner()
                    .entries
                    .into_iter()
                    .map(|e| (e.path, e.is_dir))
                    .collect()
            })
            .map_err(|e| e.message().to_string())
        })?
    }

    pub fn ler(&self, path: &str) -> Result<(String, Option<String>), String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        let projeto = self.projeto.clone();
        let path = path.to_string();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            match c
                .read_file(ReadFileRequest {
                    student: aluno,
                    project: projeto,
                    path,
                })
                .await
            {
                Ok(r) => {
                    let r = r.into_inner();
                    let rev = (!r.revision.is_empty()).then_some(r.revision);
                    Ok((r.source, rev))
                }
                Err(e) if e.code() == tonic::Code::NotFound => Err("arquivo não existe".into()),
                Err(e) => Err(e.message().to_string()),
            }
        })?
    }

    pub fn gravar(
        &self,
        path: &str,
        source: &str,
        expected: &Expected,
    ) -> Result<Saved, SaveError> {
        let (create_new, expected_revision) = match expected {
            Expected::Missing => (true, String::new()),
            Expected::Revision(r) => (false, r.clone()),
            Expected::Unversioned => return Err(SaveError::Unsupported),
        };
        let ch = self.canal().map_err(SaveError::Other)?;
        let aluno = self.aluno.clone();
        let projeto = self.projeto.clone();
        let path = path.to_string();
        let source = source.to_string();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            match c
                .write_file_checked(WriteFileRequest {
                    student: aluno,
                    project: projeto,
                    path,
                    source,
                    create_new,
                    expected_revision,
                })
                .await
            {
                Ok(r) => {
                    let r = r.into_inner();
                    if r.revision.is_empty() {
                        Err(SaveError::Unsupported)
                    } else {
                        Ok(Saved {
                            revision: r.revision,
                            warning: (!r.warning.is_empty()).then_some(r.warning),
                        })
                    }
                }
                Err(e) => Err(match e.code() {
                    tonic::Code::Aborted => SaveError::Conflict,
                    tonic::Code::AlreadyExists => SaveError::AlreadyExists,
                    tonic::Code::Unimplemented | tonic::Code::FailedPrecondition => {
                        SaveError::Unsupported
                    }
                    _ => SaveError::Other(e.message().to_string()),
                }),
            }
        })
        .map_err(SaveError::Other)?
    }

    pub fn apagar(&self, path: &str) -> Result<(), String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        let projeto = self.projeto.clone();
        let path = path.to_string();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            c.delete_file(DeleteFileRequest {
                student: aluno,
                project: projeto,
                path,
            })
            .await
            .map(|_| ())
            .map_err(|e| e.message().to_string())
        })?
    }

    pub fn mkdir(&self, path: &str) -> Result<(), String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        let projeto = self.projeto.clone();
        let path = path.to_string();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            c.mkdir(MkdirRequest {
                student: aluno,
                project: projeto,
                path,
            })
            .await
            .map(|_| ())
            .map_err(|e| e.message().to_string())
        })?
    }

    pub fn abrir_projeto(
        &mut self,
        nome: &str,
        projeto: &mut Projeto,
        prefs: &mut Prefs,
    ) -> Result<(), String> {
        self.projeto = nome.trim().to_string();
        if self.projeto.is_empty() {
            return Err("nome de projeto vazio".into());
        }
        let cache = pasta_cache_remoto(&self.aluno, &self.projeto);
        std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
        let files = self.listar_arquivos()?;
        for (rel, is_dir) in &files {
            let dest = cache.join(rel);
            if *is_dir {
                std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
                continue;
            }
            if let Ok((texto, _)) = self.ler(rel) {
                if let Some(dir) = dest.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                }
                let _ = std::fs::write(&dest, texto);
            }
        }
        projeto.raiz = Some(cache.clone());
        projeto.remoto = true;
        projeto.abertos.insert(String::new());
        projeto.todos = nos_de_entradas(files);
        prefs.servidor = self.url.clone();
        prefs.projeto = format!("{}/{}", self.aluno, self.projeto);
        prefs.pasta = cache.display().to_string();
        prefs.lembrar_recente(Recente::Servidor {
            url: self.url.clone(),
            aluno: self.aluno.clone(),
            projeto: self.projeto.clone(),
        });
        prefs.save();
        Ok(())
    }

    pub fn recarregar_arvore(&self, projeto: &mut Projeto) -> Result<(), String> {
        projeto.todos = nos_de_entradas(self.listar_arquivos()?);
        Ok(())
    }

    pub fn caminho_cache(&self, rel: &str) -> PathBuf {
        pasta_cache_remoto(&self.aluno, &self.projeto).join(rel)
    }

    pub fn gravar_cache(&self, rel: &str, texto: &str) -> Result<(), String> {
        let dest = self.caminho_cache(rel);
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(dest, texto).map_err(|e| e.to_string())
    }

    pub fn renomear(&self, from: &str, to: &str) -> Result<(), String> {
        let ch = self.canal()?;
        let aluno = self.aluno.clone();
        let projeto = self.projeto.clone();
        let from = from.to_string();
        let to = to.to_string();
        esperar(async move {
            let mut c = TurmaClient::new(ch);
            c.rename(RenameRequest {
                student: aluno,
                project: projeto,
                from,
                to,
            })
            .await
            .map(|_| ())
            .map_err(|e| e.message().to_string())
        })?
    }
}

pub fn listar_rascunhos() -> Vec<crate::documentos::Rascunho> {
    let dir = crate::documentos::pasta_rascunho();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let path = e.path();
        let ext = path.extension().and_then(|s| s.to_str());
        if ext != Some("rascunho") && ext != Some("lep") {
            continue;
        }
        let nome_ficheiro = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "rascunho.lep".into());
        if let Some(r) = crate::documentos::ler_rascunho(&path, &nome_ficheiro) {
            out.push(r);
        }
    }
    out.sort_by(|a, b| a.nome.cmp(&b.nome));
    out
}

pub fn apagar_rascunhos() {
    let dir = crate::documentos::pasta_rascunho();
    let _ = std::fs::remove_dir_all(&dir);
}

pub fn gravar_local_protegido(
    path: &Path,
    texto: &str,
    expected: &Expected,
) -> Result<Saved, SaveError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(SaveError::from)?;
    }
    expressa_aula_storage::save(path, texto.as_bytes(), expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rascunho_lista_e_apaga() {
        let dir = crate::documentos::pasta_rascunho();
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1.lep"), "escreva(1)").unwrap();
        let itens = listar_rascunhos();
        assert!(itens.iter().any(|r| r.nome == "1.lep" && r.texto.contains("escreva")));
        apagar_rascunhos();
        assert!(listar_rascunhos().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn parar_servidor_mata_o_filho() {
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        let mut slot = Some(child);
        parar_servidor(&mut slot);
        assert!(slot.is_none());
        let vivo = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(true);
        assert!(!vivo);
    }

    #[test]
    fn save_detecta_conflito() {
        let dir = std::env::temp_dir().join(format!(
            "aula-cas-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.lep");
        let first = gravar_local_protegido(&path, "um", &Expected::Missing).unwrap();
        std::fs::write(&path, "dois").unwrap();
        let err = gravar_local_protegido(&path, "tres", &Expected::Revision(first.revision)).unwrap_err();
        assert!(matches!(err, SaveError::Conflict));
        let _ = std::fs::remove_dir_all(dir);
    }
}
