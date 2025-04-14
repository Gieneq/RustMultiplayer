pub mod client_session;
pub mod routes;

use std::{
    collections::HashMap, sync::{
        Arc, 
        Mutex
    }, time::Duration
};

use client_session::{ClientSession, ClientSessionDisconnectEvent, ClientSessionId};

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
    pub world: Arc<Mutex<World>>,
    connection_task_handler: tokio::task::JoinHandle<()>,
    client_sessions_handlers: Arc<Mutex<HashMap<ClientSessionId, client_session::ClientSessionHandler>>>,
    main_task_handler: tokio::task::JoinHandle<()>,
    shutdown_sender: tokio::sync::oneshot::Sender<()>,
    notify_no_connection: Arc<tokio::sync::Notify>,
    notify_any_connection: Arc<tokio::sync::Notify>,
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
        let world = Arc::new(Mutex::new(World::new()));
        let world_shared_clients = world.clone();
        let world_shared = world.clone();

        let (shutdown_sender, mut shutdown_receiver) = tokio::sync::oneshot::channel();
        let (shutdown_server_sender, mut shutdown_server_receiver) = tokio::sync::oneshot::channel();

        let client_sessions_handlers: Arc<Mutex<HashMap<ClientSessionId, client_session::ClientSessionHandler>>> = Arc::new(Mutex::new(HashMap::new()));
        let (client_disconnect_tx, mut client_disconnect_rx) = tokio::sync::mpsc::channel::<ClientSessionDisconnectEvent>(32);
        
        let client_sessions_handlers_shared = client_sessions_handlers.clone();

        let notify_no_connection = Arc::new(tokio::sync::Notify::new());
        let notify_no_connection_shared = notify_no_connection.clone();

        let notify_any_connection = Arc::new(tokio::sync::Notify::new());
        let notify_any_connection_shared = notify_any_connection.clone();

        let connection_task_handler = tokio::spawn(async move {
            let mut new_client_session_id= 0;
            loop {
                tokio::select! {
                    _ = &mut shutdown_server_receiver => {
                        log::debug!("Received server shut down signal...");
                        break;
                    },
                    client_session_id = client_disconnect_rx.recv() => {
                        if let Some(client_session_id) = client_session_id {
                            println!("Disconnection client id={}.", client_session_id.id);
                            log::debug!("Client session got disconencted {}", client_session_id.id);

                            // Move client session
                            let (client_session_handler, no_more_clients) = {
                                let mut client_sessions_handlers_guard = client_sessions_handlers.lock().expect("Locking failed");
                                let removed_client = client_sessions_handlers_guard.remove(&client_session_id.id);
                                let no_more_clients = client_sessions_handlers_guard.is_empty();
                                (removed_client, no_more_clients)
                            };  

                            if no_more_clients {
                                notify_no_connection_shared.notify_one();
                            }                
                            
                            // Await task finish
                            if let Some(client_session_handler) = client_session_handler {
                                client_session_handler.task_handler.await.expect("Client session should close gracefully");
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
                            
                            match client_session.run(world_shared_clients.clone(), client_disconnect_tx.clone()) {
                                Ok(handler) => {
                                    let clients_count = {
                                        let mut client_sessions_handlers_guard = client_sessions_handlers.lock().expect("Locking failed");
                                        client_sessions_handlers_guard.insert(handler.id, handler);
                                        client_sessions_handlers_guard.len()
                                    };
                                    notify_any_connection_shared.notify_one();
                                    println!("Appending connection, count={}", clients_count);
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
        });

        let main_task_handler = tokio::spawn(async move {
            loop {
                // TODO add timing to have steady ticks/sec average
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
                        // Execute every tick
                        if let Ok(mut world_lock) = world_shared.lock() {
                            world_lock.tick();
                        }
                    },
                }
            }
        });

        Ok(MultiplayerServerHandler {
            world,
            connection_task_handler,
            client_sessions_handlers: client_sessions_handlers_shared,
            main_task_handler,
            shutdown_sender,
            notify_no_connection,
            notify_any_connection
        })
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
        let client_sessions_handlers_guard = self.client_sessions_handlers.lock().expect("Locking failed");
        client_sessions_handlers_guard.len()
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
            let mut world = server_handler.world.lock().unwrap();
            world.create_entity_npc("Tuna", Vector2F::new(10.5, 20.3), Vector2F::new(1.0, 1.0));
        }
    
        tokio::time::sleep(Duration::from_millis(3000)).await;
        server_handler.shutdown().await.unwrap();
    }
}
