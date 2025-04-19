pub mod gui_client;
use std::{io::{BufRead, Write}, time::{Duration, Instant}};

use crate::requests::{ClientRequest, ClientResponse};

#[derive(Debug)]
pub struct PingSessionResult {
    pub ping_session_duration: Duration,
    pub results: Vec<Option<Duration>>,
    pub loss_rate: f32,
    pub average_duration: Option<Duration>,
}

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

    pub fn ping(&self, count: usize, interval: Duration, payload: Option<String>, timeout: Duration) -> PingSessionResult {
        assert!(count > 0);
        let mut results = Vec::with_capacity(count);

        let ping_session_start = Instant::now();

        for _ in 0..count {
            let playload_cloned = payload.clone();
            let start = Instant::now();
            results.push(self.make_request_with_timeout(ClientRequest::Ping {payload: playload_cloned}, Some(timeout)).ok().map(|v| start.elapsed()));
            
            std::thread::sleep(interval);
        }

        let loss_packets_count = results.iter().filter(|v| v.is_none()).count();
        let success_packets_count = results.len() - loss_packets_count;

        let loss_rate = 100.0 * loss_packets_count as f32 / results.len() as f32;
        
        let average_duration = if success_packets_count == 0{
            None
        } else {
            let accumulated_duration = results.iter().filter(|v| v.is_some()).fold(Duration::ZERO, |acc, v| acc + v.unwrap());
            let average_duration_secs = accumulated_duration.as_secs_f64() / success_packets_count as f64;
            Some(Duration::from_secs_f64(average_duration_secs))
        };
        
        PingSessionResult {
            ping_session_duration: ping_session_start.elapsed(),
            results,
            average_duration,
            loss_rate,
        }
    }

    pub fn make_request(&self, req: ClientRequest) -> Result<ClientResponse, MultiplayerClientRequestError> {
        const COMMON_TIMEOUT_MILLIS: u64 = 100;
        self.make_request_with_timeout(req, Some(Duration::from_millis(COMMON_TIMEOUT_MILLIS)))
    }

    pub fn wait_until_finished(self) -> std::thread::Result<()> {
        self.thread_handle.join()
    }
    
    pub fn shutdown(self) -> std::thread::Result<()> {
        if let Err(e) = self.request_shutdown_tx.send(()) {
            log::warn!("Couldnt send shutdown signal, rason {e}");
        }
        unimplemented!("blah");
        self.wait_until_finished()
    }
}