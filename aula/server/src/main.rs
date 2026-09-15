mod paths;
mod service;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use expressa_aula_proto::runner_server::RunnerServer;
use expressa_aula_proto::turma_server::TurmaServer;
use tonic::transport::Server;

use service::Aula;

fn usage() {
    eprintln!(
        "Uso: expressa-aula-server [--root DIR] [--bind HOST:PORT] [--timeout-ms N]\n\n  \
         --root         pasta das turmas (padrão: ./aula-data)\n  \
         --bind         endereço (padrão: 127.0.0.1:50051)\n  \
         --timeout-ms   limite de execução (padrão: 3000)"
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = PathBuf::from("./aula-data");
    let mut bind = "127.0.0.1:50051".to_string();
    let mut timeout_ms = 3000u64;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                usage();
                return Ok(());
            }
            "--root" => {
                root = PathBuf::from(args.next().ok_or("--root precisa de um diretório")?);
            }
            "--bind" => {
                bind = args.next().ok_or("--bind precisa de HOST:PORT")?;
            }
            "--timeout-ms" => {
                timeout_ms = args
                    .next()
                    .ok_or("--timeout-ms precisa de um número")?
                    .parse()?;
            }
            other => {
                usage();
                return Err(format!("argumento desconhecido: {other}").into());
            }
        }
    }

    std::fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let addr: SocketAddr = bind.parse()?;
    let aula = Aula {
        root: root.clone(),
        timeout: Duration::from_millis(timeout_ms),
    };

    eprintln!("expressa-aula-server em {addr}");
    eprintln!("arquivos em {}", root.display());

    Server::builder()
        .add_service(TurmaServer::new(aula.clone()))
        .add_service(RunnerServer::new(aula))
        .serve(addr)
        .await?;
    Ok(())
}
