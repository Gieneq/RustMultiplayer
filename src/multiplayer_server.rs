use std::{
    sync::{
        Arc, 
        Mutex
    },
    time::Duration
};

use crate::{
    game::world::World, 
    multiplayer_client::ClientSession
};

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
    main_task_handler: tokio::task::JoinHandle<()>,
    shutdown_sender: tokio::sync::oneshot::Sender<()>,
}

pub struct MultiplayerServer {
    listener: tokio::net::TcpListener,
}

impl MultiplayerServer {
    // const MAIN_LOOP_INTERVAL: Duration = Duration::from_millis(250); // Slow for testing purpose
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

        let connection_task_handler = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_server_receiver => {
                        log::debug!("Received server shut down signal...");
                        break;
                    },
                    incomming_connection = self.listener.accept() => {
                        if let Ok(connection) = incomming_connection {
                            let client_session = ClientSession::new(connection);
                             //TODO consider storing handler.await.unwrap();
                            client_session.run(world_shared_clients.clone()).unwrap()
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
            main_task_handler,
            shutdown_sender,
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
}

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
        // world.create_entity_npc("Starlette", Vector2F::new(-2.5, 0.0));
    }

    tokio::time::sleep(Duration::from_millis(11000)).await;
    server_handler.shutdown().await.unwrap();
}