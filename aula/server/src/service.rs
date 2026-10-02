use std::fs;
use std::path::{Path, PathBuf};

use expressa_aula_proto::turma_server::Turma;
use expressa_aula_proto::{
    DeleteFileRequest, DeleteFileResponse, ListFilesRequest, ListFilesResponse, MkdirRequest,
    MkdirResponse, ReadFileRequest, ReadFileResponse, RenameRequest, RenameResponse, TreeEntry,
    WriteFileRequest, WriteFileResponse,
};
use tonic::{Request, Response, Status};

use crate::paths;

#[derive(Clone)]
pub struct Aula {
    pub root: PathBuf,
}

impl Aula {
    fn file(&self, student: &str, path: &str) -> Result<PathBuf, Status> {
        paths::student_file(&self.root, student, path).map_err(Status::invalid_argument)
    }

    fn dir(&self, student: &str) -> Result<PathBuf, Status> {
        paths::student_dir(&self.root, student).map_err(Status::invalid_argument)
    }
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
            });
            walk_tree(&path, out, prefix)?;
        } else {
            out.push(TreeEntry {
                path: rel,
                is_dir: false,
            });
        }
    }
    Ok(())
}

#[tonic::async_trait]
impl Turma for Aula {
    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        let dir = self.dir(&req.student)?;
        let mut entries = Vec::new();
        walk_tree(&dir, &mut entries, &dir).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(ListFilesResponse { entries }))
    }

    async fn read_file(
        &self,
        request: Request<ReadFileRequest>,
    ) -> Result<Response<ReadFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.path)?;
        let source = fs::read_to_string(&path).map_err(|e| Status::not_found(e.to_string()))?;
        Ok(Response::new(ReadFileResponse { source }))
    }

    async fn write_file(
        &self,
        request: Request<WriteFileRequest>,
    ) -> Result<Response<WriteFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| Status::internal(e.to_string()))?;
        }
        fs::write(&path, req.source.as_bytes()).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(WriteFileResponse {}))
    }

    async fn delete_file(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let req = request.into_inner();
        let path = self.file(&req.student, &req.path)?;
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
        let path = self.file(&req.student, &req.path)?;
        fs::create_dir_all(&path).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(MkdirResponse {}))
    }

    async fn rename(
        &self,
        request: Request<RenameRequest>,
    ) -> Result<Response<RenameResponse>, Status> {
        let req = request.into_inner();
        let from = self.file(&req.student, &req.from)?;
        let to = self.file(&req.student, &req.to)?;
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| Status::internal(e.to_string()))?;
        }
        fs::rename(&from, &to).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(RenameResponse {}))
    }
}
