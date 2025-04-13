use std::sync::{
    Arc, 
    Mutex
};

use tokio::io::{
    AsyncBufReadExt, AsyncWriteExt
};

use crate::game::{
    math::Vector2F, 
    world::{
        self, EntityId, World
    }
};

#[derive(Debug, thiserror::Error)]
pub enum ClientSessionError {

}

#[derive(Debug)]
pub struct ClientSession {
    id: ClientSessionId,
    socket: tokio::net::TcpStream,
    address: std::net::SocketAddr,
}

pub type ClientSessionId = u32;

#[derive(Debug)]
pub struct ClientSessionHandler {
    pub id: ClientSessionId,
    pub task_handler: tokio::task::JoinHandle<()>
}

#[derive(Debug)]
pub struct ClientSessionDisconnectEvent {
    pub id: ClientSessionId
}

impl ClientSession {
    pub fn new(conenction: (tokio::net::TcpStream, std::net::SocketAddr), new_id: ClientSessionId) -> Self {
        let (socket, address) = conenction;
        Self {
            id: new_id,
            socket, 
            address
        }
    }

    fn on_client_connect(world: Arc<Mutex<World>>) -> EntityId {
        let player_id: EntityId = match world.lock() {
            Ok(mut world_guard) => {
                world_guard.create_entity_player(
                    "Player", 
                    Vector2F::new(0.0, 0.0),
                    world::ENTITY_SIZE
                )
            },
            Err(e) => {
                panic!("Could not acquite mutex, reason {e}");
            },
        };

        player_id
    }

    // TODO Add on client register

    /// Line is trimmed already
    fn on_client_request(player_id: EntityId, request: &str, world: Arc<Mutex<World>>) -> String {
        super::routes::route_client_request(player_id, request, world)
    }

    fn on_client_disconnect(player_id: EntityId, world: Arc<Mutex<World>>) {
        match world.lock() {
            Ok(mut world_guard) => {
                world_guard.remove_entity(player_id).expect("Player should exist");
            },
            Err(e) => {
                panic!("Could not acquite mutex, reason {e}");
            },
        }
    }

    async fn process_client_connection(
        &mut self, 
        world: Arc<Mutex<World>>,
        session_disconnect_tx: tokio::sync::mpsc::Sender<ClientSessionDisconnectEvent>
    ) {
        log::info!("Processing client connection: {:?}", self.address);
        let player_entity_id: EntityId = Self::on_client_connect(world.clone());

        let (reader, mut writer) = self.socket.split();
        let mut buf_reader = tokio::io::BufReader::new(reader);
        let mut line_buff = String::new();

        loop {
            let cloned_world = world.clone();
            match buf_reader.read_line(&mut line_buff).await {
                Ok(0) => {
                    log::debug!("Client finished connection");
                    log::info!("Client see world: {:?}", cloned_world);
                    break;
                },
                Ok(_) => {
                    let line = line_buff.trim();
                    log::debug!("Client send line: '{}'", line);

                    let mut response = Self::on_client_request(
                        player_entity_id, 
                        line, 
                        cloned_world
                    );
                    log::debug!("Response with: '{}'", response);

                    response.push('\n');

                    if let Err(e) = writer.write_all(response.as_bytes()).await {
                        log::error!("Client could not send response {} reason: {e}", line_buff.trim());
                    }

                    if let Err(e) = writer.flush().await {
                        log::error!("Client could not flush reason: {e}");
                    }
                },
                Err(e) => {
                    log::error!("Client faile reason = {e}, finished connection");
                    break;
                }
            }
            line_buff.clear();
        }

        log::debug!("Client disconnected");
        Self::on_client_disconnect(player_entity_id, world);
        
        if let Err(e) = session_disconnect_tx.send(ClientSessionDisconnectEvent { id: self.id }).await {
            log::warn!("Failed to send disconnect event for client {}: {}", self.id, e);
        }
        
    }

    pub fn run(
        mut self, 
        world: Arc<Mutex<World>>,
        session_disconnect_tx: tokio::sync::mpsc::Sender<ClientSessionDisconnectEvent>
    ) -> Result<ClientSessionHandler, ClientSessionError> {
        let client_session_id = self.id;
        let client_session_handler = tokio::spawn(async move {
            self.process_client_connection(world, session_disconnect_tx).await
        });

        Ok(ClientSessionHandler {
            id: client_session_id,
            task_handler: client_session_handler
        })
    }
}