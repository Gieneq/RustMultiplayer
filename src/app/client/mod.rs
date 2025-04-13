pub mod rendering;
use std::{io::{BufRead, Write}, sync::{Arc, Mutex}, time::Duration};

use rendering::{AppData, EntityView};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::requests::{ClientRequest, ClientResponse, MoveDirection};


// pub struct Client {
//     socket: tokio::net::TcpStream
// }

// pub struct ClientHandle {
//     task_handle: tokio::task::JoinHandle<()>,
//     contol_signals_tx: std::sync::mpsc::Sender<MoveDirection>
// }

// async fn client_do_request_await_response(
//     req: &str,
//     buf_reader: &mut tokio::io::BufReader<tokio::net::tcp::ReadHalf<'_>>,
//     write: &mut tokio::net::tcp::WriteHalf<'_>,
// ) -> String {
//     let mut buf_string = String::new();

//     write.write_all(req.as_bytes()).await.unwrap();
//     write.write_all(b"\n").await.unwrap();
//     write.flush().await.unwrap();

//     buf_reader.read_line(&mut buf_string).await.unwrap();
//     buf_string.trim().to_string()
// }

// impl Client {
//     pub async fn connect<A: tokio::net::ToSocketAddrs + std::fmt::Debug>(addr: A) -> Client {
//         log::info!("Client attempts to connect to server {addr:?}...");

//         let socket = tokio::net::TcpStream::connect(addr).await.unwrap();
//         let client_address = socket.local_addr().unwrap();
//         log::info!("Client {client_address} connected!");
//         Client {
//             socket
//         }
//     }

//     pub async fn run(mut self, app_data: Arc<Mutex<AppData>>) -> ClientHandle {
//         let (contol_signals_tx, contol_signals_rx) = std::sync::mpsc::channel();

//         let task_handle = tokio::task::spawn(async move {
//             let (read_half, mut write_half) = self.socket.split();
//             let mut buf_reader = tokio::io::BufReader::new(read_half);

//             // store player id
//             // TODO register instead jsut get id
//             let player_id = {
//                 let response = client_do_request_await_response(
//                     "{\"type\":\"GetId\"}",
//                     &mut buf_reader,
//                     &mut write_half
//                 ).await;

//                 if let Ok(ClientResponse::GetId { id }) = serde_json::from_str(&response) {
//                     id
//                 } else {
//                     panic!("PlayerGetID parse failed")
//                 }
//             };

//             loop {
//                 let response = client_do_request_await_response(
//                     "{\"type\":\"WorldCheck\"}",
//                     &mut buf_reader,
//                     &mut write_half
//                 ).await;
//                 log::trace!("Client got response '{response}'.");

//                 if let ClientResponse::WorldCheck { entities } = serde_json::from_str(&response).unwrap() {
//                     // Update shared data
//                     if let Ok(mut app_data_guard) = app_data.lock() {
//                         app_data_guard.entities.clear();
//                         for entiy in entities {
//                             if entiy.id == player_id {
//                                 app_data_guard.camera_position = entiy.position;
//                             }

//                             let color = [
//                                 entiy.color[0] as f32 / 255.0,
//                                 entiy.color[1] as f32 / 255.0,
//                                 entiy.color[2] as f32 / 255.0
//                             ];

//                             app_data_guard.entities.push(EntityView { 
//                                 position: entiy.position, 
//                                 size: entiy.size, 
//                                 color
//                             });
                            
//                         }
//                     }
//                 }

//                 // Poll for control signals
//                 if let Ok(move_dir) = contol_signals_rx.try_recv() {
//                     let request = serde_json::to_string(&ClientRequest::Move{dir: move_dir}).unwrap();
//                     let response = client_do_request_await_response(
//                         &request,
//                         &mut buf_reader,
//                         &mut write_half
//                     ).await;
//                     log::debug!("Client got response '{response}'.");
//                 }
                

//                 tokio::time::sleep(Duration::from_millis(32)).await;
//             }

//         });

//         ClientHandle {
//             task_handle,
//             contol_signals_tx
//         }
//     }
// }

// impl ClientHandle {
//     pub async fn wait_until_finished(self) -> Result<(), tokio::task::JoinError> {
//         self.task_handle.await
//     }

//     pub fn move_headless(&self, direction: MoveDirection) {
//         self.contol_signals_tx.send(direction).unwrap();
//     }
// }

///////////////////////////////////////////////////////

#[derive(Debug, thiserror::Error)]
pub enum MultiplayerClientError {
    #[error("IoError, reason='{0}'")]
    IoError(#[from] std::io::Error),
}


#[derive(Debug, thiserror::Error)]
pub enum MultiplayerClientRequestError {
    #[error("IoError, reason='{0}'")]
    IoError(#[from] std::io::Error),

    #[error("SerdeError, reason='{0}'")]
    SerdeError(#[from] serde_json::Error),

    #[error("Server closed")]
    ServerClosed,

    #[error("TimeoutReceive reason='{0}'")]
    TimeoutReceive(#[from] std::sync::mpsc::RecvTimeoutError),

    #[error("RecvError reason='{0}'")]
    RecvError(#[from] std::sync::mpsc::RecvError),

    #[error("SendError channel reason='{0}'")]
    SendError(#[from] std::sync::mpsc::SendError<ClientRequest>),
}


const RW_TIMOUT_SECS: u64 = 2;

pub struct MultiplayerClient {
    socket: std::net::TcpStream
}


pub struct MultiplayerClientHandle {
    thread_handle: std::thread::JoinHandle<()>,
    request_shutdown_tx: std::sync::mpsc::Sender<()>,
    requests_tx: std::sync::mpsc::Sender<ClientRequest>,
    response_rx: std::sync::mpsc::Receiver<Result<ClientResponse, MultiplayerClientRequestError>>,
}

impl MultiplayerClient {
    pub fn connect<A: std::net::ToSocketAddrs + std::fmt::Debug>(addr: A) -> Result<Self, MultiplayerClientError> {
        log::info!("Client attempts to connect to server {addr:?}...");
    
        let socket = std::net::TcpStream::connect(addr)?;
        socket.set_read_timeout(Some(Duration::from_secs(RW_TIMOUT_SECS)))?;
        socket.set_write_timeout(Some(Duration::from_secs(RW_TIMOUT_SECS)))?;

        log::info!("Client {} connected!", socket.local_addr().unwrap());

        Ok(Self { socket })
    }
    
    pub fn run(self) -> Result<MultiplayerClientHandle, MultiplayerClientError> {
        let (requests_tx, requests_rx) = std::sync::mpsc::channel();
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        let (request_shutdown_tx, _request_shutdown_rx) = std::sync::mpsc::channel();

        let thread_handle = std::thread::spawn(move || {
            let mut stream = self.socket;
            let mut buf_reader = std::io::BufReader::new(stream.try_clone().unwrap());

            loop {
                // TODO poll request_shutdown_rx also, consider crossbeam
                match requests_rx.recv() {
                    Ok(client_request) => {
                        let client_request_serialized = serde_json::to_string(&client_request).expect("Could not serialize request");

                        if let Err(e) = writeln!(stream, "{}", client_request_serialized) {
                            response_tx.send(Err(e.into())).ok();
                            continue;
                        }
                        
                        // Serialized request was sent,await response
                        let mut response_line_buffer = String::new();
                        match buf_reader.read_line(&mut response_line_buffer) {
                            Ok(0) => {
                                log::warn!("Server got closed");
                                response_tx.send(Err(MultiplayerClientRequestError::ServerClosed)).ok();
                                break;
                            },
                            Ok(_) => {
                                let serialized_response: ClientResponse = {
                                    match serde_json::from_str(&response_line_buffer) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            log::error!("Could not serialize response, reason {e}");
                                            response_tx.send(Err(e.into())).ok();
                                            continue;
                                        }
                                    }
                                };

                                response_tx.send(Ok(serialized_response)).ok();
                            },
                            Err(e) => {
                                log::error!("Other error during receiving response {e}");
                                response_tx.send(Err(e.into())).ok();
                            },
                        }
                    },
                    Err(_) => {
                        log::info!("Request channel closed. Exiting client loop");
                        break;
                    }
                }
            }
        });

        Ok(MultiplayerClientHandle {
            thread_handle,
            request_shutdown_tx,
            requests_tx,
            response_rx
        })
    }
}

impl MultiplayerClientHandle {
    pub fn make_request_with_timeout(&self, req: ClientRequest, timeout: Option<Duration>) -> Result<ClientResponse, MultiplayerClientRequestError> {
        self.requests_tx.send(req)?;

        if let Some(timeout) = timeout {
            self.response_rx.recv_timeout(timeout)?
        } else {
            self.response_rx.recv()?
        }
    }

    pub fn make_request(&self, req: ClientRequest) -> Result<ClientResponse, MultiplayerClientRequestError> {
        const COMMON_TIMEOUT_MILLIS: u64 = 100;
        self.make_request_with_timeout(req, Some(Duration::from_millis(COMMON_TIMEOUT_MILLIS)))
    }

    // TODO rename
    pub fn join(self) -> std::thread::Result<()> {
        self.thread_handle.join()
    }
    
    pub fn shutdown(self) -> std::thread::Result<()> {
        if let Err(e) = self.request_shutdown_tx.send(()) {
            log::warn!("Couldnt send shutdown signal, rason {e}");
        }
        unimplemented!("blah");
        self.join()
    }
}