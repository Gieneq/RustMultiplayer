use std::sync::{
    Arc, 
    Mutex
};

use serde::{Deserialize, Serialize};
use tokio::io::{
    AsyncBufReadExt, AsyncWriteExt
};

use crate::game::{
    math::Vector2F, 
    world::{
        self, EntityId, World
    }
};

use super::MultiplayerServerContext;

#[derive(Debug, thiserror::Error)]
pub enum ClientSessionError {

}

#[derive(Debug, Serialize, Deserialize, Clone, Copy,PartialEq)]
pub enum GameplayState {
    Lobby {
        ready: bool,
    },
    Ingame {
        entity_player_id: EntityId,
    },
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClientSessionState {
    #[default]
    JustConnected,
    NameWasSet {
        name: String,
        gameplay_state: GameplayState
    },
}

#[derive(Debug, Default, Serialize, Deserialize, Clone,PartialEq)]
pub struct ClientSessionData {
    pub state: ClientSessionState,
    pub points: u32,
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
    pub data: Arc<Mutex<ClientSessionData>>,
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

    fn on_client_connect(client_session_id: ClientSessionId, address: std::net::SocketAddr) {
        log::debug!("Client {client_session_id} connected with address: {address}");
    }

    fn on_client_request(
        server_context: Arc<MultiplayerServerContext>,
        client_session_id: ClientSessionId, 
        session_data: Arc<Mutex<ClientSessionData>>,
        request: &str, world: Arc<Mutex<World>>
    ) -> String {
        // 'request' line is trimmed already
        super::routes::route_client_request(
            server_context,
            client_session_id, 
            session_data,
            request, 
            world
        )
    }

    fn on_client_disconnect(client_session_id: ClientSessionId) {
        log::debug!("Client {client_session_id} disconnected");
    }

    async fn process_client_connection(
        &mut self, 
        server_context: Arc<MultiplayerServerContext>,
        session_data: Arc<Mutex<ClientSessionData>>,
        world: Arc<Mutex<World>>,
        session_disconnect_tx: tokio::sync::mpsc::Sender<ClientSessionDisconnectEvent>
    ) {
        log::info!("Processing client id={} connection: {:?}", self.id, self.address);
        Self::on_client_connect(self.id, self.address);

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
                        server_context.clone(),
                        self.id, 
                        session_data.clone(),
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

        Self::on_client_disconnect(self.id);
        
        if let Err(e) = session_disconnect_tx.send(ClientSessionDisconnectEvent { id: self.id }).await {
            log::warn!("Failed to send disconnect event for client {}: {}", self.id, e);
        }
        
    }

    pub fn run(
        mut self, 
        server_context: Arc<MultiplayerServerContext>,
        world: Arc<Mutex<World>>,
        session_disconnect_tx: tokio::sync::mpsc::Sender<ClientSessionDisconnectEvent>
    ) -> Result<ClientSessionHandler, ClientSessionError> {
        let client_session_id = self.id;
        
        // Not player attached yet
        let session_data = Arc::new(Mutex::new(ClientSessionData::default()));

        let session_data_shared = session_data.clone();
        let client_session_handler = tokio::spawn(async move {
            self.process_client_connection(
                server_context, 
                session_data_shared, 
                world, 
                session_disconnect_tx
            ).await
        });

        Ok(ClientSessionHandler {
            id: client_session_id,
            data: session_data,
            task_handler: client_session_handler
        })
    }
}

impl Default for GameplayState {
    fn default() -> Self {
        Self::Lobby { ready: false }
    }
}

impl ClientSessionData {
    pub fn get_entity_player_id(&self) -> Option<EntityId> {
        match &self.state {
            ClientSessionState::JustConnected => None,
            ClientSessionState::NameWasSet { name: _, gameplay_state } => match gameplay_state {
                GameplayState::Lobby {ready: _} => None,
                GameplayState::Ingame { entity_player_id } => Some(*entity_player_id),
            },
        }
    }

    pub fn get_name(&self) -> Option<&str> {
        match &self.state {
            ClientSessionState::JustConnected => None,
            ClientSessionState::NameWasSet { name, gameplay_state: _ } => Some(name.as_str()),
        }
    }
}