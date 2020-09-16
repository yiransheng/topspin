use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;

use log;

pub async fn run_log_server(
    sender: Sender<(String, BufReader<TcpStream>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = TcpListener::bind("127.0.0.1:9527").await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let sender_ = sender.clone();
        tokio::spawn(async move {
            let _ = handle_stream(sender_, stream).await;
        });
    }
}

async fn handle_stream(
    mut sender: Sender<(String, BufReader<TcpStream>)>,
    stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    stream.read_line(&mut line).await?;
    log::info!("Connecting to: {}", &line);
    sender.send((line, stream)).await?;

    Ok(())
}
