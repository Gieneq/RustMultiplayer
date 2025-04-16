pub mod client_session;
pub mod routes;
pub mod chat;

use std::{
    collections::HashMap, 
    sync::{
        Arc, 
        Mutex
    }, 
    time::Duration
};

use chat::ChatMessage;

use client_session::{
    ClientSession, 
    ClientSessionDisconnectEvent, 
    ClientSessionId
};

use crate::game::world::World;

#[derive(Debug, thiserror::Error)]
pub enum MultiplayerServerError {
    #[error("IoError, reason='{0}'")]
    IoError(#[from] tokio::io::Error),

    #[error("Failed to shutdown server")]
    ShutdownError,

    #[error("Could not join task, reason='{0}'")]
    TaskJoinError(#[from] tokio::task::JoinError),
}

pub struct MultiplayerServerHandler {
    connection_task_handler: tokio::task::JoinHandle<()>,
    pub server_context: Arc<MultiplayerServerContext>,
    main_task_handler: tokio::task::JoinHandle<()>,
    shutdown_sender: tokio::sync::oneshot::Sender<()>,
    notify_no_connection: Arc<tokio::sync::Notify>,
    notify_any_connection: Arc<tokio::sync::Notify>,
}

#[derive(Debug)]
pub enum GameplayStateTransitionError {
    AlreadyInLobby,
    AlreadyGameRunning,
}

#[derive(Debug)]
pub enum GameplayState {
    Lobby {
        counting_to_start: Option<u32>,
    },
    GameRunning {
        world: World
    },
}

pub struct MultiplayerServerContext {
    pub client_sessions_handlers: Mutex<HashMap<ClientSessionId, client_session::ClientSessionHandler>>,
    pub chat: Mutex<Vec<ChatMessage>>,
    pub gameplay_state: Mutex<GameplayState>,
}

pub struct MultiplayerServer {
    listener: tokio::net::TcpListener,
}

impl MultiplayerServer {
    const MAIN_LOOP_INTERVAL: Duration = Duration::from_millis(32);

    pub async fn bind_any_local() -> Result<Self, MultiplayerServerError> {
        Self::bind("127.0.0.1:0").await
    }

    pub async fn bind<A: tokio::net::ToSocketAddrs>(addr: A) -> Result<Self, MultiplayerServerError> {
        Ok(Self {
            listener: tokio::net::TcpListener::bind(addr).await?,
        })
    }

    pub fn get_local_address(&self) -> Result<std::net::SocketAddr, std::io::Error> {
        self.listener.local_addr()
    }

    pub async fn run(self) -> Result<MultiplayerServerHandler, MultiplayerServerError> {
        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
        let (shutdown_server_sender, shutdown_server_receiver) = tokio::sync::oneshot::channel();

        let (client_disconnect_tx, client_disconnect_rx) = tokio::sync::mpsc::channel::<ClientSessionDisconnectEvent>(32);
        
        let server_context = Arc::new(MultiplayerServerContext {
            client_sessions_handlers: Mutex::new(HashMap::new()),
            chat: Mutex::new(Vec::default()),
            gameplay_state: Mutex::new(GameplayState::default())
        });
        let server_context_shared = server_context.clone();
        server_context_shared.chat.lock().unwrap().push(ChatMessage::new_from_server("Message of the day 'Pizza!'".to_string()));

        let notify_no_connection = Arc::new(tokio::sync::Notify::new());
        let notify_no_connection_shared = notify_no_connection.clone();

        let notify_any_connection = Arc::new(tokio::sync::Notify::new());
        let notify_any_connection_shared = notify_any_connection.clone();

        let connection_task_handler = tokio::spawn(async move {
            self.connection_procedure(
                shutdown_server_receiver,
                client_disconnect_rx,
                client_disconnect_tx,
                server_context_shared,
                notify_no_connection_shared,
                notify_any_connection_shared,
            ).await;
        });

        let server_context_shared_main_loop = server_context.clone();
        let main_task_handler = tokio::spawn(async move {
            Self::main_task_procedure(
                shutdown_receiver, 
                shutdown_server_sender, 
                server_context_shared_main_loop
            ).await;
        });

        Ok(MultiplayerServerHandler {
            connection_task_handler,
            server_context,
            main_task_handler,
            shutdown_sender,
            notify_no_connection,
            notify_any_connection
        })
    }
    
    async fn connection_procedure(
        self,
        mut shutdown_server_receiver: tokio::sync::oneshot::Receiver<()>,
        mut client_disconnect_rx: tokio::sync::mpsc::Receiver<ClientSessionDisconnectEvent>,
        client_disconnect_tx: tokio::sync::mpsc::Sender<ClientSessionDisconnectEvent>,
        server_context_shared: Arc<MultiplayerServerContext>,
        notify_no_connection_shared: Arc<tokio::sync::Notify>,
        notify_any_connection_shared: Arc<tokio::sync::Notify>,
    ) {
        let mut new_client_session_id= 0;
        loop {
            tokio::select! {
                _ = &mut shutdown_server_receiver => {
                    log::debug!("Received server shut down signal...");
                    break;
                },
                client_session_id = client_disconnect_rx.recv() => {
                    if let Some(client_session_id) = client_session_id {
                        println!("[{:?}] Disconnection client id={}.", std::time::Instant::now(), client_session_id.id);
                        log::debug!("Client session got disconencted {}", client_session_id.id);

                        // Move client session
                        let (client_session_handler, no_more_clients) = {
                            let mut client_sessions_handlers_guard = server_context_shared.client_sessions_handlers.lock().unwrap();
                            let removed_client = client_sessions_handlers_guard.remove(&client_session_id.id);
                            let no_more_clients = client_sessions_handlers_guard.is_empty();
                            (removed_client, no_more_clients)
                        };  

                        if no_more_clients {
                            notify_no_connection_shared.notify_one();
                        }
                        
                        // Await task finish
                        if let Some(client_session_handler) = client_session_handler {
                            if let Err(e) = client_session_handler.task_handler.await {
                                log::error!("Client session should close gracefully, reason={e}");
                            }
                        } else {
                            log::warn!("Attempt to remove not existing client session {}", client_session_id.id);
                        };
                        
                    } else {
                        log::warn!("Received None from client_disconnect_rx colelctor!");
                    }
                }
                incomming_connection = self.listener.accept() => {
                    if let Ok(connection) = incomming_connection {
                        let assigned_client_session_id = {
                            let tmp = new_client_session_id;
                            new_client_session_id += 1;
                            tmp
                        };

                        let client_session = ClientSession::new(connection, assigned_client_session_id);
                        
                        match client_session.run(server_context_shared.clone(), client_disconnect_tx.clone()) {
                            Ok(handler) => {
                                let clients_count = {
                                    let mut client_sessions_handlers_guard = server_context_shared.client_sessions_handlers.lock().unwrap();
                                    client_sessions_handlers_guard.insert(handler.id, handler);
                                    client_sessions_handlers_guard.len()
                                };
                                notify_any_connection_shared.notify_one();
                                println!("[{:?}] Appending connection {}, count={}", std::time::Instant::now(), assigned_client_session_id, clients_count);
                            },
                            Err(e) => {
                                log::error!("Failed to run client session: {:?}", e);
                            },
                        }
                    }
                },
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    log::trace!("Dummy sleep, to remove...");
                },
            }
        }
    }

    async fn main_task_procedure(
        mut shutdown_receiver: tokio::sync::oneshot::Receiver<()>, 
        shutdown_server_sender: tokio::sync::oneshot::Sender<()>, 
        server_context_shared_main_loop: Arc<MultiplayerServerContext>
    ) {
        loop {
            tokio::select! {
                _ = &mut shutdown_receiver => {
                    // Received shutdown signal
                    log::debug!("Received shut down signal...");

                    if shutdown_server_sender.send(()).is_err() {
                        log::error!("Could not emit signal to stop server!");
                    }

                    break;
                },
                _ = tokio::time::sleep(Self::MAIN_LOOP_INTERVAL) => {
                    // TODO add timing to have steady ticks/sec average
                    let mut gameplay_state_guard = server_context_shared_main_loop.gameplay_state.lock().unwrap();
                    match &mut *gameplay_state_guard {
                        GameplayState::Lobby { counting_to_start: _ } => {
                        
                        },
                        GameplayState::GameRunning { world } => {
                            // Execute every tick
                            world.tick();
                        },
                    }
                },
            }
        }
    }

}

impl MultiplayerServerHandler {
    pub async fn shutdown(self) -> Result<(), MultiplayerServerError> {
        log::debug!("Gracefully shutting down server...");
        self.shutdown_sender.send(()).map_err(|_| MultiplayerServerError::ShutdownError)?;
        self.main_task_handler.await?;
        self.connection_task_handler.await?;
        log::debug!("Server shut down successfully!");
        Ok(())
    }

    pub async fn await_any_connection(&self) {
        self.notify_any_connection.notified().await
    }

    pub async fn await_all_disconnect(&self) {
        self.notify_no_connection.notified().await
    }

    pub fn connections_count(&self) -> usize {
        let client_sessions_handlers_guard = self.server_context.client_sessions_handlers.lock().unwrap();
        client_sessions_handlers_guard.len()
    }
}

impl MultiplayerServerContext {
    pub fn is_name_used(&self, name: &str) -> bool {
        let guard = self.client_sessions_handlers.lock().unwrap();
        guard.iter().any(|(_, v)| v.data.lock().unwrap().get_name() == Some(name))
    }

    pub fn get_connections_count(&self) -> usize {
        let guard = self.client_sessions_handlers.lock().unwrap();
        guard.len()
    }
}

impl Default for GameplayState {
    fn default() -> Self {
        Self::Lobby { counting_to_start: None }
    }
}

impl GameplayState {
    pub fn try_transition_to_game_running(&mut self) -> Result<(), GameplayStateTransitionError> {
        if let GameplayState::GameRunning { world: _ } = self {
            return Err(GameplayStateTransitionError::AlreadyGameRunning)
        }

        *self = GameplayState::GameRunning { world: World::new() };
        Ok(())
    }
    
    pub fn try_transition_to_lobby(&mut self) -> Result<(), GameplayStateTransitionError> {
        if let GameplayState::Lobby { counting_to_start: _ } = self {
            return Err(GameplayStateTransitionError::AlreadyInLobby)
        }

        *self = GameplayState::Lobby { counting_to_start: None };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::math::Vector2F;

    #[tokio::test]
    async fn test_server_creation() {
        let server = MultiplayerServer::bind_any_local().await.unwrap();
        let server_address = server.get_local_address().unwrap();
        println!("{server_address:?}");
        let server_handler = server.run().await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        server_handler.shutdown().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_server_adding_entities() {
        let server = MultiplayerServer::bind_any_local().await.unwrap();
        let server_handler = server.run().await.unwrap();
    
        {
            let mut gameplay_state_guard = server_handler.server_context.gameplay_state.lock().unwrap();
            gameplay_state_guard.try_transition_to_game_running().unwrap();

            if let GameplayState::GameRunning { world } = &mut *gameplay_state_guard {
                world.create_entity_npc("Tuna", Vector2F::new(10.5, 20.3), Vector2F::new(1.0, 1.0));
            }
        }
    
        tokio::time::sleep(Duration::from_millis(3000)).await;
        server_handler.shutdown().await.unwrap();
    }
}
