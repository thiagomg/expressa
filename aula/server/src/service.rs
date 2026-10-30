use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use expressa_aula_proto::turma_server::Turma;
use expressa_aula_proto::{
    CreateProjectRequest, CreateProjectResponse, DeleteFileRequest, DeleteFileResponse,
    ListFilesRequest, ListFilesResponse, ListProjectsRequest, ListProjectsResponse, MkdirRequest,
    MkdirResponse, ReadFileRequest, ReadFileResponse, RenameRequest, RenameResponse, TreeEntry,
    WriteFileRequest, WriteFileResponse,
};
use tonic::{Request, Response, Status};

use crate::{paths, projetos};

/// The message the editor shows as is.
fn ja_existe(rel: &str) -> Status {
    Status::already_exists(format!("já existe `{rel}`"))
}

fn interno(e: std::io::Error) -> Status {
    Status::internal(e.to_string())
}

#[derive(Clone)]
pub struct Aula {
    root: PathBuf,
    /// Held while a student folder is laid out (`projetos::preparar`), so
    /// two requests never move the same files at once.
    preparando: Arc<Mutex<()>>,
}

impl Aula {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            preparando: Arc::new(Mutex::new(())),
        }
    }

    /// The student's folder, laid out as one folder per project.
    fn aluno(&self, student: &str) -> Result<PathBuf, Status> {
        let dir = paths::student_dir(&self.root, student).map_err(Status::invalid_argument)?;
        let _vez = self.preparando.lock().unwrap_or_else(|e| e.into_inner());
        projetos::preparar(&dir).map_err(interno)?;
        Ok(dir)
    }

    /// An existing project's folder. The default project is created on
    /// demand (older editors only know that one); any other must exist, so
    /// a mistyped name is an error instead of a new empty project.
    fn projeto(&self, student: &str, project: &str) -> Result<PathBuf, Status> {
        self.aluno(student)?;
        let dir =
            paths::project_dir(&self.root, student, project).map_err(Status::invalid_argument)?;
        if !dir.is_dir() {
            if paths::sanitize_project(project) == Ok(paths::PROJETO_PADRAO) {
                fs::create_dir_all(&dir).map_err(interno)?;
            } else {
                return Err(Status::not_found(format!(
                    "projeto `{}` não existe",
                    project.trim()
                )));
            }
        }
        Ok(dir)
    }

    fn file(&self, student: &str, project: &str, path: &str) -> Result<PathBuf, Status> {
        self.projeto(student, project)?;
        paths::student_file(&self.root, student, project, path).map_err(Status::invalid_argument)
    }
}

/// Modification time in nanoseconds since 1970, or 0 if the system has none.
fn modificado_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as i64)
}

fn walk_tree(dir: &Path, out: &mut Vec<TreeEntry>, prefix: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });
    for entry in entries {
        let path = entry.path();
        let rel = paths::relative_to(prefix, &path);
        if path.is_dir() {
            out.push(TreeEntry {
                path: rel,
                is_dir: true,
                ..Default::default()
            });
            walk_tree(&path, out, prefix)?;
        } else {
            let meta = entry.metadata()?;
            out.push(TreeEntry {
                path: rel,
                is_dir: false,
                size: meta.len(),
                modified_ns: modificado_ns(&meta),
            });
        }
    }
    Ok(())
}

#[tonic::async_trait]
impl Turma for Aula {
    async fn list_projects(
        &self,
        request: Request<ListProjectsRequest>,
    ) -> Result<Response<ListProjectsResponse>, Status> {
        let req = request.into_inner();
        let dir = self.aluno(&req.student)?;
        let projects = projetos::listar(&dir).map_err(interno)?;
        Ok(Response::new(ListProjectsResponse { projects }))
    }

    async fn create_project(
        &self,
        request: Request<CreateProjectRequest>,
    ) -> Result<Response<CreateProjectResponse>, Status> {
        let req = request.into_inner();
        if req.project.trim().is_empty() {
            return Err(Status::invalid_argument("dê um nome ao projeto"));
        }
        self.aluno(&req.student)?;
        let dir = paths::project_dir(&self.root, &req.student, &req.project)
            .map_err(Status::invalid_argument)?;
        fs::create_dir(&dir).map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => {
                Status::already_exists(format!("já existe o projeto `{}`", req.project.trim()))
            }
            _ => interno(e),
        })?;
        Ok(Response::new(CreateProjectResponse {}))
    }

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        let dir = self.projeto(&req.student, &req.project)?;
        let mut entries = Vec::new();
        walk_tree(&dir, &mut entries, &dir).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListFilesResponse { entries }))
    }

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<ReadFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.project, &req.path)?;
        let source = fs::read_to_string(&path).map_err(|e| Status::not_found(e.to_string()))?;
        Ok(Response::new(ReadFileResponse { source }))
    }

    async fn write_file(
        &self,
        request: Request<WriteFileRequest>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.project, &req.path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Status::internal(e.to_string()))?;
        }
        if req.create_new {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| match e.kind() {
                    std::io::ErrorKind::AlreadyExists => ja_existe(&req.path),
                    _ => Status::internal(e.to_string()),
                })?;
            file.write_all(req.source.as_bytes())
                .map_err(|e| Status::internal(e.to_string()))?;
        } else {
            fs::write(&path, req.source.as_bytes()).map_err(|e| Status::internal(e.to_string()))?;
        }
        Ok(Response::new(WriteFileResponse {}))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.project, &req.path)?;
        if path.is_dir() {
            fs::remove_dir_all(&path).map_err(|e| Status::internal(e.to_string()))?;
        } else {
            fs::remove_file(&path).map_err(|e| Status::not_found(e.to_string()))?;
        }
        Ok(Response::new(DeleteFileResponse {}))
    }

    async fn mkdir(
        &self,
        request: Request<MkdirRequest>,
    ) -> Result<Response<MkdirResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.project, &req.path)?;
        fs::create_dir_all(&path).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(MkdirResponse {}))
    }

    async fn rename(
        &self,
        request: Request<RenameRequest>,
    ) -> Result<Response<RenameResponse>, Status> {
        let req = request.into_inner();
        let from = self.file(&req.student, &req.project, &req.from)?;
        let to = self.file(&req.student, &req.project, &req.to)?;
        if from == to {
            return Ok(Response::new(RenameResponse {}));
        }
        // `fs::rename` replaces an existing file on Unix.
        if to.symlink_metadata().is_ok() {
            return Err(ja_existe(&req.to));
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| Status::internal(e.to_string()))?;
        }
        fs::rename(&from, &to).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(RenameResponse {}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Code;

    /// A fresh root where `ana` has one file from before projects existed,
    /// `ana/a.lep` = "original" (moved to `principal` on first use).
    fn servidor(nome: &str) -> Aula {
        let root = std::env::temp_dir().join(format!(
            "expressa-aula-server-{nome}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("ana")).unwrap();
        fs::write(root.join("ana/a.lep"), "original").unwrap();
        Aula::new(root)
    }

    /// `rel` in ana's default project.
    fn ler(aula: &Aula, rel: &str) -> String {
        ler_em(aula, "principal", rel)
    }

    fn ler_em(aula: &Aula, projeto: &str, rel: &str) -> String {
        fs::read_to_string(aula.root.join("ana").join(projeto).join(rel)).unwrap()
    }

    fn escrever(rel: &str, source: &str, create_new: bool) -> Request<WriteFileRequest> {
        escrever_em("", rel, source, create_new)
    }

    fn escrever_em(
        projeto: &str,
        rel: &str,
        source: &str,
        create_new: bool,
    ) -> Request<WriteFileRequest> {
        Request::new(WriteFileRequest {
            student: "ana".into(),
            project: projeto.into(),
            path: rel.into(),
            source: source.into(),
            create_new,
        })
    }

    fn renomear(from: &str, to: &str) -> Request<RenameRequest> {
        Request::new(RenameRequest {
            student: "ana".into(),
            project: String::new(),
            from: from.into(),
            to: to.into(),
        })
    }

    async fn projetos(aula: &Aula, aluno: &str) -> Vec<String> {
        aula.list_projects(Request::new(ListProjectsRequest {
            student: aluno.into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .projects
    }

    async fn criar_projeto(aula: &Aula, nome: &str) -> Result<(), Status> {
        aula.create_project(Request::new(CreateProjectRequest {
            student: "ana".into(),
            project: nome.into(),
        }))
        .await
        .map(|_| ())
    }

    async fn arquivos(aula: &Aula, projeto: &str) -> Vec<String> {
        aula.list_files(Request::new(ListFilesRequest {
            student: "ana".into(),
            project: projeto.into(),
        }))
        .await
        .unwrap()
        .into_inner()
        .entries
        .into_iter()
        .map(|e| e.path)
        .collect()
    }

    #[tokio::test]
    async fn arquivos_antigos_aparecem_no_projeto_principal() {
        let aula = servidor("antigos");
        assert_eq!(projetos(&aula, "ana").await, ["principal"]);
        assert_eq!(arquivos(&aula, "").await, ["a.lep"]);
        assert_eq!(arquivos(&aula, "principal").await, ["a.lep"]);
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn listagem_traz_tamanho_e_data_dos_arquivos() {
        let aula = servidor("versao");
        fs::create_dir_all(aula.root.join("ana/lib")).unwrap();
        let entradas = aula
            .list_files(Request::new(ListFilesRequest {
                student: "ana".into(),
                project: String::new(),
            }))
            .await
            .unwrap()
            .into_inner()
            .entries;
        let a = entradas.iter().find(|e| e.path == "a.lep").unwrap();
        assert_eq!(a.size, "original".len() as u64);
        assert!(a.modified_ns > 0);
        let lib = entradas.iter().find(|e| e.path == "lib").unwrap();
        assert!(lib.is_dir);
        assert_eq!((lib.size, lib.modified_ns), (0, 0));
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn projetos_sao_separados() {
        let aula = servidor("separados");
        criar_projeto(&aula, "aula-2").await.unwrap();
        aula.write_file(escrever_em("aula-2", "a.lep", "da aula 2", true))
            .await
            .unwrap();
        assert_eq!(ler(&aula, "a.lep"), "original");
        assert_eq!(ler_em(&aula, "aula-2", "a.lep"), "da aula 2");
        // The new project does not show up inside the old one.
        assert_eq!(arquivos(&aula, "").await, ["a.lep"]);
        assert_eq!(projetos(&aula, "ana").await, ["aula-2", "principal"]);
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn criar_projeto_que_existe_falha() {
        let aula = servidor("existe");
        criar_projeto(&aula, "aula-2").await.unwrap();
        let err = criar_projeto(&aula, "aula-2").await.unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
        for ruim in ["", "..", "a/b", ".x"] {
            let err = criar_projeto(&aula, ruim).await.unwrap_err();
            assert_eq!(err.code(), Code::InvalidArgument, "{ruim:?}");
        }
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn projeto_inexistente_nao_e_criado_por_engano() {
        let aula = servidor("inexistente");
        let err = aula
            .write_file(escrever_em("aual-2", "a.lep", "x", false))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
        assert_eq!(projetos(&aula, "ana").await, ["principal"]);
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn aluno_novo_comeca_sem_projetos() {
        let aula = servidor("novo");
        assert_eq!(projetos(&aula, "bruno").await, Vec::<String>::new());
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn caminho_nao_sai_do_projeto() {
        let aula = servidor("fuga");
        criar_projeto(&aula, "aula-2").await.unwrap();
        for rel in ["../principal/a.lep", "../../bruno/principal/a.lep"] {
            let err = aula
                .write_file(escrever_em("aula-2", rel, "x", false))
                .await
                .unwrap_err();
            assert_eq!(err.code(), Code::InvalidArgument, "{rel}");
        }
        assert_eq!(ler(&aula, "a.lep"), "original");
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn create_new_nao_substitui_arquivo() {
        let aula = servidor("create-new");
        let err = aula
            .write_file(escrever("a.lep", "novo", true))
            .await
            .unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
        assert_eq!(ler(&aula, "a.lep"), "original");

        aula.write_file(escrever("b.lep", "novo", true))
            .await
            .unwrap();
        assert_eq!(ler(&aula, "b.lep"), "novo");
        // A plain save still replaces.
        aula.write_file(escrever("a.lep", "salvo", false))
            .await
            .unwrap();
        assert_eq!(ler(&aula, "a.lep"), "salvo");
        let _ = fs::remove_dir_all(&aula.root);
    }

    #[tokio::test]
    async fn renomear_nao_substitui_destino() {
        let aula = servidor("rename");
        aula.write_file(escrever("b.lep", "outro", false))
            .await
            .unwrap();
        let err = aula.rename(renomear("b.lep", "a.lep")).await.unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
        assert_eq!(ler(&aula, "a.lep"), "original");
        assert_eq!(ler(&aula, "b.lep"), "outro");

        aula.rename(renomear("b.lep", "c.lep")).await.unwrap();
        assert_eq!(ler(&aula, "c.lep"), "outro");
        // Same name: nothing to do, and nothing lost.
        aula.rename(renomear("a.lep", "./a.lep")).await.unwrap();
        assert_eq!(ler(&aula, "a.lep"), "original");
        let _ = fs::remove_dir_all(&aula.root);
    }

    /// Over the wire, as the editor talks to it: an old editor (no project)
    /// and a new one share `principal`, and a new project stays apart.
    #[tokio::test]
    async fn pela_rede_editor_antigo_e_novo() {
        use expressa_aula_proto::turma_client::TurmaClient;
        use expressa_aula_proto::turma_server::TurmaServer;

        let aula = servidor("rede");
        let root = aula.root.clone();
        let porta = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let servidor = tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(TurmaServer::new(aula))
                .serve(porta),
        );
        let url = format!("http://{porta}");
        let mut c = loop {
            match TurmaClient::connect(url.clone()).await {
                Ok(c) => break c,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
            }
        };

        // Old editor: no `project` field at all.
        let antigo = c
            .read_file(ReadFileRequest {
                student: "ana".into(),
                path: "a.lep".into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .into_inner()
            .source;
        assert_eq!(antigo, "original");

        c.create_project(CreateProjectRequest {
            student: "ana".into(),
            project: "aula-2".into(),
        })
        .await
        .unwrap();
        c.write_file(WriteFileRequest {
            student: "ana".into(),
            project: "aula-2".into(),
            path: "a.lep".into(),
            source: "novo".into(),
            create_new: true,
        })
        .await
        .unwrap();
        let projetos = c
            .list_projects(ListProjectsRequest {
                student: "ana".into(),
            })
            .await
            .unwrap()
            .into_inner()
            .projects;
        assert_eq!(projetos, ["aula-2", "principal"]);
        assert_eq!(
            fs::read_to_string(root.join("ana/principal/a.lep")).unwrap(),
            "original"
        );
        assert_eq!(
            fs::read_to_string(root.join("ana/aula-2/a.lep")).unwrap(),
            "novo"
        );

        servidor.abort();
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn apagar_ponto_nao_apaga_o_projeto() {
        let aula = servidor("delete-dot");
        for rel in [".", "lib/.."] {
            let err = aula
                .delete_file(Request::new(DeleteFileRequest {
                    student: "ana".into(),
                    project: String::new(),
                    path: rel.into(),
                }))
                .await
                .unwrap_err();
            assert_eq!(err.code(), Code::InvalidArgument, "{rel}");
        }
        assert_eq!(ler(&aula, "a.lep"), "original");
        let _ = fs::remove_dir_all(&aula.root);
    }
}
