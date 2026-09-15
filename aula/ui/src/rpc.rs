use expressa_aula_proto::runner_client::RunnerClient;
use expressa_aula_proto::turma_client::TurmaClient;
use expressa_aula_proto::{
    DeleteFileRequest, ExecFinished, ExecIn, ListFilesRequest, ReadFileRequest, RunRequest,
    TreeEntry, WriteFileRequest,
};
use tokio_stream::StreamExt;
use tonic::transport::{Channel, Endpoint};

pub struct Rpc {
    rt: tokio::runtime::Runtime,
    channel: Channel,
}

pub enum ExecEvent {
    Stdout(String),
    Leia(String),
    Finished(ExecFinished),
}

impl Rpc {
    pub fn connect(url: &str) -> Result<Self, String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let channel = rt
            .block_on(async {
                Endpoint::from_shared(url.to_string())
                    .map_err(|e| e.to_string())?
                    .connect()
                    .await
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| e)?;
        Ok(Self { rt, channel })
    }

    pub fn list_tree(&self, student: &str) -> Result<Vec<TreeEntry>, String> {
        let mut c = TurmaClient::new(self.channel.clone());
        let resp = self
            .rt
            .block_on(c.list_files(ListFilesRequest {
                student: student.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(resp.into_inner().entries)
    }

    pub fn read_file(&self, student: &str, path: &str) -> Result<String, String> {
        let mut c = TurmaClient::new(self.channel.clone());
        let resp = self
            .rt
            .block_on(c.read_file(ReadFileRequest {
                student: student.to_string(),
                path: path.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(resp.into_inner().source)
    }

    pub fn write_file(&self, student: &str, path: &str, source: &str) -> Result<(), String> {
        let mut c = TurmaClient::new(self.channel.clone());
        self.rt
            .block_on(c.write_file(WriteFileRequest {
                student: student.to_string(),
                path: path.to_string(),
                source: source.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_file(&self, student: &str, path: &str) -> Result<(), String> {
        let mut c = TurmaClient::new(self.channel.clone());
        self.rt
            .block_on(c.delete_file(DeleteFileRequest {
                student: student.to_string(),
                path: path.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn spawn_exec(
        &self,
        student: String,
        path: String,
        source: String,
        event_tx: std::sync::mpsc::Sender<ExecEvent>,
        line_rx: std::sync::mpsc::Receiver<String>,
    ) {
        let channel = self.channel.clone();
        self.rt.spawn(async move {
            let mut client = RunnerClient::new(channel);
            let (in_tx, in_rx) = tokio::sync::mpsc::channel::<ExecIn>(8);
            let start = ExecIn {
                payload: Some(expressa_aula_proto::exec_in::Payload::Start(RunRequest {
                    student,
                    path,
                    source,
                })),
            };
            if in_tx.send(start).await.is_err() {
                return;
            }

            let in_tx_lines = in_tx.clone();
            tokio::task::spawn_blocking(move || {
                while let Ok(line) = line_rx.recv() {
                    let msg = ExecIn {
                        payload: Some(expressa_aula_proto::exec_in::Payload::Line(line)),
                    };
                    if in_tx_lines.blocking_send(msg).is_err() {
                        break;
                    }
                }
            });

            let outbound = tokio_stream::wrappers::ReceiverStream::new(in_rx);
            let mut inbound = match client.exec(outbound).await {
                Ok(s) => s.into_inner(),
                Err(e) => {
                    let _ = event_tx.send(ExecEvent::Finished(ExecFinished {
                        ok: false,
                        error_message: e.to_string(),
                        error_file: String::new(),
                        error_line: 0,
                        error_col: 0,
                    }));
                    return;
                }
            };

            while let Some(msg) = inbound.next().await {
                match msg {
                    Ok(out) => match out.payload {
                        Some(expressa_aula_proto::exec_out::Payload::Stdout(s)) => {
                            let _ = event_tx.send(ExecEvent::Stdout(s));
                        }
                        Some(expressa_aula_proto::exec_out::Payload::LeiaPrompt(p)) => {
                            let _ = event_tx.send(ExecEvent::Leia(p));
                        }
                        Some(expressa_aula_proto::exec_out::Payload::Finished(f)) => {
                            let _ = event_tx.send(ExecEvent::Finished(f));
                            return;
                        }
                        None => {}
                    },
                    Err(e) => {
                        let _ = event_tx.send(ExecEvent::Finished(ExecFinished {
                            ok: false,
                            error_message: e.to_string(),
                            error_file: String::new(),
                            error_line: 0,
                            error_col: 0,
                        }));
                        return;
                    }
                }
            }
        });
    }
}

/// Start `expressa-aula-server` from next to this binary if nothing is listening.
pub fn ensure_server(url: &str) -> Result<Rpc, String> {
    if let Ok(rpc) = Rpc::connect(url) {
        return Ok(rpc);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("sem pasta do executável")?;
    let server = dir.join("expressa-aula-server");
    if !server.exists() {
        return Err(format!(
            "não conectou em {url} e não achei {}\nRode: cargo run -p expressa-aula-server",
            server.display()
        ));
    }
    std::process::Command::new(&server)
        .args(["--bind", "127.0.0.1:50051", "--root", "./aula-data"])
        .spawn()
        .map_err(|e| format!("não iniciou o servidor: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(400));
    Rpc::connect(url).map_err(|e| format!("servidor iniciado, mas ainda não responde ({e})"))
}
