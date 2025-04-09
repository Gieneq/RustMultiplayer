pub mod rendering;
use std::{sync::{Arc, Mutex}, time::Duration};

use rendering::{AppData, EntityView};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::requests::{ClientRequest, ClientResponse, MoveDirection};


pub struct Client {
    socket: tokio::net::TcpStream
}

pub struct ClientHandle {
    task_handle: tokio::task::JoinHandle<()>,
    contol_signals_tx: std::sync::mpsc::Sender<MoveDirection>
}

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

impl Client {
    pub async fn connect<A: tokio::net::ToSocketAddrs + std::fmt::Debug>(addr: A) -> Client {
        log::info!("Client attempts to connect to server {addr:?}...");

        let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client_address = socket.local_addr().unwrap();
        log::info!("Client {client_address} connected!");
        Client {
            socket
        }
    }

    pub async fn run(mut self, app_data: Arc<Mutex<AppData>>) -> ClientHandle {
        let (contol_signals_tx, contol_signals_rx) = std::sync::mpsc::channel();

        let task_handle = tokio::task::spawn(async move {
            let (read_half, mut write_half) = self.socket.split();
            let mut buf_reader = tokio::io::BufReader::new(read_half);

            // store player id
            // TODO register instead jsut get id
            let player_id = {
                let response = client_do_request_await_response(
                    "{\"type\":\"GetId\"}",
                    &mut buf_reader,
                    &mut write_half
                ).await;

                if let Ok(ClientResponse::GetId { id }) = serde_json::from_str(&response) {
                    id
                } else {
                    panic!("PlayerGetID parse failed")
                }
            };

            loop {
                let response = client_do_request_await_response(
                    "{\"type\":\"WorldCheck\"}",
                    &mut buf_reader,
                    &mut write_half
                ).await;
                log::trace!("Client got response '{response}'.");

                if let ClientResponse::WorldCheck { entities } = serde_json::from_str(&response).unwrap() {
                    // Update shared data
                    if let Ok(mut app_data_guard) = app_data.lock() {
                        app_data_guard.entities.clear();
                        for entiy in entities {
                            if entiy.id == player_id {
                                app_data_guard.camera_position = entiy.position;
                            }

                            let color = [
                                entiy.color[0] as f32 / 255.0,
                                entiy.color[1] as f32 / 255.0,
                                entiy.color[2] as f32 / 255.0
                            ];

                            app_data_guard.entities.push(EntityView { 
                                position: entiy.position, 
                                size: entiy.size, 
                                color
                            });
                            
                        }
                    }
                }

                // Poll for control signals
                if let Ok(move_dir) = contol_signals_rx.try_recv() {
                    let request = serde_json::to_string(&ClientRequest::Move{dir: move_dir}).unwrap();
                    let response = client_do_request_await_response(
                        &request,
                        &mut buf_reader,
                        &mut write_half
                    ).await;
                    log::debug!("Client got response '{response}'.");
                }
                

                tokio::time::sleep(Duration::from_millis(32)).await;
            }

        });

        ClientHandle {
            task_handle,
            contol_signals_tx
        }
    }
}

impl ClientHandle {
    pub async fn wait_until_finished(self) -> Result<(), tokio::task::JoinError> {
        self.task_handle.await
    }

    pub fn move_headless(&self, direction: MoveDirection) {
        self.contol_signals_tx.send(direction).unwrap();
    }
}