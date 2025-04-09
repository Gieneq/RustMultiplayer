use rust_multiplayer::TEST_SERVER_ADRESS;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use tokio::net::TcpStream;
use std::net::SocketAddr;


async fn client_do_request_await_response(
    req: &str,
    buf_reader: &mut tokio::io::BufReader<tokio::net::tcp::ReadHalf<'_>>,
    write: &mut tokio::net::tcp::WriteHalf<'_>,
) -> String {
    let mut buf_string = String::new();

    write.write_all(req.as_bytes()).await.unwrap();
    write.write_all(b"\n").await.unwrap();
    write.flush().await.unwrap();

    buf_reader.read_line(&mut buf_string).await.unwrap();
    buf_string.trim().to_string()
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_timestamp_millis()
        .format_file(false)
        .format_line_number(true)
        .init();

    log::info!("Client attempts to connect to server {TEST_SERVER_ADRESS}...");

    let mut socket = TcpStream::connect(TEST_SERVER_ADRESS).await.unwrap();
    let client_address: SocketAddr = socket.local_addr().unwrap();
    log::info!("Client {client_address} connected!");

    let (read_half, mut write_half) = socket.split();
    let mut buf_reader = tokio::io::BufReader::new(read_half);

    let requests = [
        String::from("{\"type\":\"Healthcheck\"}"),
        String::from("{\"type\":\"GetId\"}"),
        String::from("{\"type\":\"WorldCheck\"}"),
    ];

    for request in requests {
        let response = client_do_request_await_response(
            &request,
            &mut buf_reader,
            &mut write_half,
        ).await;

        println!("'{request}' -> '{response}'");
    }
}