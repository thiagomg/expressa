use std::net::SocketAddr;
use std::path::PathBuf;

use expressa_aula_proto::turma_server::TurmaServer;
use expressa_aula_server::Aula;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

fn usage() {
    eprintln!(
        "Uso: expressa-aula-server [--root DIR] [--bind HOST:PORT]\n\n  \
         --root   pasta das turmas (padrão: ./aula-data)\n  \
         --bind   endereço (padrão: 127.0.0.1:50051)\n\n\
         O editor não inicia este programa. ./aula-data é relativo à pasta de\n\
         trabalho: na sala use um caminho absoluto, por exemplo\n\
         $HOME/expressa-aula-data ou C:\\Expressa\\aula-data, e --bind 0.0.0.0:50051.\n\
         Libere essa porta no firewall para os alunos conectarem.\n\n\
         Cada projeto fica em DIR/aluno/projeto. Arquivos de antes dos projetos\n\
         vão para DIR/aluno/principal no primeiro uso.\n\n\
         Só guarda os arquivos; o editor executa e depura os programas.\n\
         Não há senha: quem conecta escolhe o nome do aluno."
    );
}

fn porta_em_uso(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut atual = Some(err);
    while let Some(erro) = atual {
        if erro
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::AddrInUse)
        {
            return true;
        }
        let texto = erro.to_string();
        if texto.contains("Address already in use")
            || texto.contains("AddrInUse")
            || texto.contains("Only one usage of each socket address")
        {
            return true;
        }
        atual = erro.source();
    }
    false
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut root = PathBuf::from("./aula-data");
    let mut bind = "127.0.0.1:50051".to_string();

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
            other => {
                usage();
                return Err(format!("argumento desconhecido: {other}").into());
            }
        }
    }

    std::fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let addr: SocketAddr = bind.parse()?;
    // Bind first. A process that loses the port exits without touching the
    // tree of the server that is already listening.
    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(erro) if porta_em_uso(&erro) => {
            eprintln!(
                "a porta {addr} já está em uso; escolha outro --bind ou encerre o programa que a ocupa"
            );
            std::process::exit(1);
        }
        Err(erro) => return Err(erro.into()),
    };
    expressa_aula_server::limpar_sobras(&root)?;
    let aula = Aula::new(root.clone());

    eprintln!("expressa-aula-server em {addr}");
    eprintln!("arquivos em {}", root.display());

    let servindo = Server::builder()
        .add_service(TurmaServer::new(aula))
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
            if tokio::signal::ctrl_c().await.is_err() {
                std::future::pending::<()>().await;
            }
            eprintln!("encerrando; gravações em andamento terminam antes de sair");
        })
        .await;
    if let Err(erro) = servindo {
        if porta_em_uso(&erro) {
            eprintln!(
                "a porta {addr} já está em uso; escolha outro --bind ou encerre o programa que a ocupa"
            );
            std::process::exit(1);
        }
        return Err(erro.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Embrulho(std::io::Error);

    impl std::fmt::Display for Embrulho {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl std::error::Error for Embrulho {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }

    #[test]
    fn reconhece_porta_em_uso_aninhada() {
        let io = std::io::Error::new(std::io::ErrorKind::AddrInUse, "Address already in use");
        assert!(porta_em_uso(&Embrulho(io)));
        let outro = std::io::Error::other("transport error");
        assert!(!porta_em_uso(&outro));
    }
}
