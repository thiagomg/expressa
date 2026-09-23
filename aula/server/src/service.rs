use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use expressa::runtime::{LeiaHost, run_with_leia_host};
use expressa_aula_proto::runner_server::Runner;
use expressa_aula_proto::turma_server::Turma;
use expressa_aula_proto::{
    DeleteFileRequest, DeleteFileResponse, ExecFinished, ExecIn, ExecOut, ListFilesRequest,
    ListFilesResponse, ReadFileRequest, ReadFileResponse, TreeEntry, WriteFileRequest,
    WriteFileResponse,
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use crate::paths;

#[derive(Clone)]
pub struct Aula {
    pub root: PathBuf,
    pub timeout: Duration,
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
        fs::remove_file(&path).map_err(|e| Status::not_found(e.to_string()))?;
        Ok(Response::new(DeleteFileResponse {}))
    }
}

struct ChannelLeia {
    prompts: std::sync::mpsc::Sender<String>,
    lines: std::sync::mpsc::Receiver<String>,
}

impl LeiaHost for ChannelLeia {
    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        self.prompts
            .send(prompt.to_string())
            .map_err(|_| "IDE desconectada".to_string())?;
        self.lines
            .recv()
            .map_err(|_| "IDE desconectada".to_string())
    }
}

struct ChannelOut {
    tx: std::sync::mpsc::Sender<String>,
}

impl Write for ChannelOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = self.tx.send(String::from_utf8_lossy(buf).into_owned());
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl Runner for Aula {
    type ExecStream = ReceiverStream<Result<ExecOut, Status>>;

    async fn exec(
        &self,
        request: Request<Streaming<ExecIn>>,
    ) -> Result<Response<Self::ExecStream>, Status> {
        let mut inbound = request.into_inner();
        let start = loop {
            match inbound.next().await {
                Some(Ok(msg)) => match msg.payload {
                    Some(expressa_aula_proto::exec_in::Payload::Start(s)) => break s,
                    _ => continue,
                },
                Some(Err(e)) => return Err(Status::internal(e.to_string())),
                None => return Err(Status::invalid_argument("falta start")),
            }
        };

        let path = self.file(&start.student, &start.path)?;
        let workspace = self.dir(&start.student)?;
        fs::create_dir_all(&workspace).map_err(|e| Status::internal(e.to_string()))?;
        let source = if start.source.is_empty() {
            fs::read_to_string(&path).map_err(|e| Status::not_found(e.to_string()))?
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| Status::internal(e.to_string()))?;
            }
            fs::write(&path, start.source.as_bytes())
                .map_err(|e| Status::internal(e.to_string()))?;
            start.source
        };

        let file = path.to_string_lossy().into_owned();
        let timeout = self.timeout;
        let workspace_run = workspace.clone();

        let (prompt_tx, prompt_rx) = std::sync::mpsc::channel::<String>();
        let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
        let (out_tx, out_rx) = std::sync::mpsc::channel::<String>();
        let (done_tx, done_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut host = ChannelLeia {
                prompts: prompt_tx,
                lines: line_rx,
            };
            let mut out = ChannelOut { tx: out_tx.clone() };
            let mut err = ChannelOut { tx: out_tx };
            let result = run_with_leia_host(
                &source,
                &file,
                Some(workspace_run),
                Some(timeout),
                &mut out,
                &mut err,
                &mut host,
            );
            let _ = done_tx.send(result);
        });

        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = inbound.next() => {
                        match msg {
                            Some(Ok(m)) => {
                                if let Some(expressa_aula_proto::exec_in::Payload::Line(line)) =
                                    m.payload
                                {
                                    let _ = line_tx.send(line);
                                }
                            }
                            Some(Err(_)) | None => break,
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(8)) => {
                        while let Ok(chunk) = out_rx.try_recv() {
                            let _ = tx.send(Ok(ExecOut {
                                payload: Some(expressa_aula_proto::exec_out::Payload::Stdout(chunk)),
                            })).await;
                        }
                        while let Ok(prompt) = prompt_rx.try_recv() {
                            let _ = tx.send(Ok(ExecOut {
                                payload: Some(
                                    expressa_aula_proto::exec_out::Payload::LeiaPrompt(prompt),
                                ),
                            })).await;
                        }
                        if let Ok(result) = done_rx.try_recv() {
                            while let Ok(chunk) = out_rx.try_recv() {
                                let _ = tx.send(Ok(ExecOut {
                                    payload: Some(
                                        expressa_aula_proto::exec_out::Payload::Stdout(chunk),
                                    ),
                                })).await;
                            }
                            let finished = match result {
                                Ok(0) => ExecFinished {
                                    ok: true,
                                    error_message: String::new(),
                                    error_file: String::new(),
                                    error_line: 0,
                                    error_col: 0,
                                },
                                Ok(code) => ExecFinished {
                                    ok: false,
                                    error_message: format!("encerrou com código {code}"),
                                    error_file: String::new(),
                                    error_line: 0,
                                    error_col: 0,
                                },
                                Err(e) => ExecFinished {
                                    ok: false,
                                    error_message: e.message,
                                    error_file: e.file,
                                    error_line: e.span.line,
                                    error_col: e.span.col,
                                },
                            };
                            let _ = tx.send(Ok(ExecOut {
                                payload: Some(
                                    expressa_aula_proto::exec_out::Payload::Finished(finished),
                                ),
                            })).await;
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
