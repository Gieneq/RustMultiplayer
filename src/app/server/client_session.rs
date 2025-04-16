use std::sync::{
    Arc, 
    Mutex
};

use serde::{
    Deserialize, 
    Serialize
};
use tokio::io::{
    AsyncBufReadExt, 
    AsyncWriteExt
};

use crate::game::world::{
    EntityId, 
    World
};

use super::MultiplayerServerContext;

#[derive(Debug, thiserror::Error)]
pub enum ClientSessionError {

}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClientSessionState {
    #[default]
    JustConnected,
    NameWasSet {
        name: String,
        ready_to_start: bool,
        entity_player_id: Option<EntityId>,
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
        request: &str
    ) -> String {
        // 'request' line is trimmed already
        super::routes::route_client_request(
            server_context,
            client_session_id, 
            session_data,
            request
        )
    }

    fn on_client_disconnect(client_session_id: ClientSessionId) {
        log::debug!("Client {client_session_id} disconnected");
    }

    async fn process_client_connection(
        &mut self, 
        server_context: Arc<MultiplayerServerContext>,
        session_data: Arc<Mutex<ClientSessionData>>,
        session_disconnect_tx: tokio::sync::mpsc::Sender<ClientSessionDisconnectEvent>
    ) {
        log::info!("Processing client id={} connection: {:?}", self.id, self.address);
        Self::on_client_connect(self.id, self.address);

        let (reader, mut writer) = self.socket.split();
        let mut buf_reader = tokio::io::BufReader::new(reader);
        let mut line_buff = String::new();

        loop {
            match buf_reader.read_line(&mut line_buff).await {
                Ok(0) => {
                    log::debug!("Client finished connection");
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

impl ClientSessionData {
    pub fn get_entity_player_id(&self) -> Option<EntityId> {
        match &self.state {
            ClientSessionState::JustConnected => None,
            ClientSessionState::NameWasSet { name: _, ready_to_start: _, entity_player_id } => *entity_player_id,
        }
    }

    pub fn get_name(&self) -> Option<&str> {
        match &self.state {
            ClientSessionState::JustConnected => None,
            ClientSessionState::NameWasSet { name, ready_to_start: _, entity_player_id: _ } => Some(name.as_str()),
        }
    }
}