pub mod rendering;

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use crate::game::{math::Vector2F, world::{EntityId, World}};

#[derive(Debug, thiserror::Error)]
pub enum ClientSessionError {

}

pub struct ClientSession {
    socket: tokio::net::TcpStream,
    address: std::net::SocketAddr,
}

impl ClientSession {
    pub fn new(conenction: (tokio::net::TcpStream, std::net::SocketAddr)) -> Self {
        let (socket, address) = conenction;
        Self {
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
                    Vector2F::new(4.8, 4.8)
                )
            },
            Err(e) => {
                panic!("Could not acquite mutex, reason {e}");
            },
        };

        player_id
    }

    /// Line is trimmed already
    fn on_client_request(player_id: EntityId, request: &str, world: Arc<Mutex<World>>) -> String {
        crate::requests::route_request(player_id, request, world)
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

    async fn process_client_connection(&mut self, world: Arc<Mutex<World>>) {
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
    }

    pub fn run(mut self, world: Arc<Mutex<World>>) -> Result<(), ClientSessionError> {
        let _client_session_handler = tokio::spawn(async move {
            self.process_client_connection(world).await
        });

        Ok(())
    }
}