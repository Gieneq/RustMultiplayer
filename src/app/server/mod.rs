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
    ClientSessionId, ClientSessionState
};
use rand::seq::{IndexedRandom, IteratorRandom};
use serde::{Deserialize, Serialize};

use crate::game::{math::Vector2F, world::{get_tiled_value, World, WorldError, ENTITY_SIZE}};

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

#[derive(Debug, thiserror::Error)]
pub enum StartGameError {
    #[error("NoFreeTiles")]
    NoFreeTiles,

    #[error("WorldErrorHappen, reason='{0}'")]
    WorldErrorHappen(#[from] WorldError)
}

#[derive(Debug)]
pub enum GameplayStateTransitionError {
    BadState,
    AlreadyInState,
}

#[derive(Debug)]
pub enum GameplayState {
    Lobby {
        counting_to_start: Option<u32>,
        last_result: Option<GameplayResult>,
    },
    GameRunning {
        world: World
    },
    Ending {
        countdown: u32,
        result: GameplayResult
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GameplayResult {
    SeekerWin {
        reward: u32,
    },
    HidersWin {
        reward: u32,
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
    
    //TODO reject connecting other clients of game is in progress
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
                    Self::main_loop_procedure(server_context_shared_main_loop.clone());
                },
            }
        }
    }

    fn main_loop_procedure(server_context: Arc<MultiplayerServerContext>) {
        const CLIENTS_REQUIRED_TO_START: usize = 2;
        const TICKS_TO_COUNTDOWN: u32 = 10;

        let mut gameplay_state_guard = server_context.gameplay_state.lock().unwrap();

        if let GameplayState::Lobby { counting_to_start, last_result:_ } = &mut *gameplay_state_guard {
            let all_ready = server_context.are_all_clients_ready();
            let enough_clients = server_context.get_connections_count() >= CLIENTS_REQUIRED_TO_START;

            // Counting transitions
            match counting_to_start {
                Some(_) if !all_ready || !enough_clients => {
                    // Should stop counting
                    *counting_to_start = None;
                },
                Some(count) => {
                    // Count down
                    *count = count.saturating_sub(1);
                }
                None if all_ready && enough_clients => {
                    // Should start counting
                    *counting_to_start = Some(TICKS_TO_COUNTDOWN);
                },
                _ => {}
            }

            if let Some(count) = counting_to_start {
                // Countdown exhausted
                if *count == 0 {
                    gameplay_state_guard.try_transition_from_lobby_to_gamerunning().unwrap();
                    if let GameplayState::GameRunning { world } = &mut *gameplay_state_guard {       
                        let start_game_reuslt = Self::start_new_game(world, &server_context.client_sessions_handlers);

                        if start_game_reuslt.is_err() {
                            gameplay_state_guard.unexpected_transition_to_lobby();
                        }
                    }
                    return;
                }
            }
        }
        
        if let GameplayState::GameRunning { world } = &mut *gameplay_state_guard {
            // TODO some game end check
            let result = Self::check_gameplay_result(world);

            if let Some(result) = result {
                // Has result, detach entities from clients, transition to ending countdownstage 

                // All not ready, EntityIds to None
                server_context.detach_entities_from_clients();

                gameplay_state_guard.try_transition_from_gamerunning_to_ending(result).unwrap();
                return;
            } else {
                // No result yet
                world.tick();
            }
        }

        if let GameplayState::Ending { countdown, result: _ }= &mut *gameplay_state_guard {
            *countdown = countdown.saturating_sub(1);
            
            if *countdown == 0 || server_context.get_connections_count() == 0 {
                gameplay_state_guard.try_transition_from_ending_to_lobby().unwrap();
            }
        }

    }

    fn check_gameplay_result(world: &World) -> Option<GameplayResult> {
        const SMALL_HIDERS_REWARD: u32 = 5; // Seeker has gone
        // const MEDIUM_HIDERS_REWARD: u32 = 10;
        // const BIGL_HIDERS_REWARD: u32 = 20; // All surviwed
        
        const SMALL_SEEKER_REWARD: u32 = 5;

        // let clients_guard = clients.lock().unwrap();
        let summary = world.get_seeker_hiders_summary();
        // TODO use summary to togglegameplay state

        let seeker_win = if let Some((_, seeker_stats)) = summary.seeker {
            if seeker_stats.remaining_failures == 0 || seeker_stats.remaining_ticks == 0 {
                Some(GameplayResult::HidersWin { reward: SMALL_HIDERS_REWARD })
            } else {
                None
            }
        } else {
            // Seeker has gone, hiders win
            Some(GameplayResult::HidersWin { reward: SMALL_HIDERS_REWARD })
        };

        if seeker_win.is_some() {
            return seeker_win;
        }

        summary.hiders.iter()
            .all(|(_, h)| !h.covered)
            .then_some(GameplayResult::SeekerWin { reward: SMALL_SEEKER_REWARD })
    }

    fn start_new_game(
        world: &mut World,
        clients: &Mutex<HashMap<u32, client_session::ClientSessionHandler>>
    ) -> Result<(), StartGameError> {
        const MAPSIZE_GENERATION_FACTOR: usize = 3;

        log::info!("Game just started!");
        let mut rng = rand::rng();

        let hiders_count = {
            let clients_guard = clients.lock().unwrap();
            clients_guard.len().saturating_sub(1)
        };

        let generation_range = get_tiled_value((hiders_count.min(1) * MAPSIZE_GENERATION_FACTOR) as i32);

        // Generate world
        Self::generate_world(world, &mut rng, generation_range)?;

        // Assign entity to clients
        Self::assign_world_entities_to_clients(world, clients, &mut rng, generation_range)?;

        // Add NPCs
        Self::place_npcs_around_world(world, &mut rng, generation_range, hiders_count)?;

        Ok(())
    }

    fn generate_world(
        _world: &mut World, 
        _rng: &mut rand::prelude::ThreadRng,
        _generation_range: f32
    ) -> Result<(), StartGameError> {
        // In future some obstacles, scenery
        Ok(())
    }

    fn place_npcs_around_world(
        world: &mut World, 
        rng: &mut rand::prelude::ThreadRng,
        generation_range: f32,
        hiders_count: usize
    ) -> Result<(), StartGameError> {
        const NPCS_PER_HIDER: usize = 2;
        let free_tiles = world.get_free_tiles_positions(Vector2F::zero(), generation_range);
        
        // Need at least 1 spot for NPCs
        if free_tiles.is_empty() {
            return Err(StartGameError::NoFreeTiles);
        }

        let nps_count = {
            let expectednpc_count = NPCS_PER_HIDER * hiders_count;
            free_tiles.len().min(expectednpc_count)
        };

        let npcs_initial_positions = free_tiles.choose_multiple(rng, nps_count);
        for &initial_position in npcs_initial_positions {
            let _entity_id = world.create_entity_npc("NPC", initial_position, ENTITY_SIZE);
        }

        Ok(())
    }

    fn assign_world_entities_to_clients(
        world: &mut World, clients: &Mutex<HashMap<u32, 
        client_session::ClientSessionHandler>>, 
        rng: &mut rand::prelude::ThreadRng,
        generation_range: f32
    ) -> Result<(), StartGameError> {
        const SEEKING_MAX_TIME: u32 = 2000;
        const SEEKING_MAX_TRIES: usize = 3;

        let mut clients_guard = clients.lock().unwrap();

        let seeker_client_id = *clients_guard.keys().choose(rng).unwrap();

        let free_tiles = world.get_free_tiles_positions(Vector2F::zero(), generation_range);
        
        // Need at least 1 spot for NPCs
        if free_tiles.len() <= clients_guard.len() {
            return Err(StartGameError::NoFreeTiles);
        }

        // TODO spread hiders and seekers, some Voronoi can work
        let mut initial_clients_positions = free_tiles.choose_multiple(rng, clients_guard.len());
        
        for (_, client) in clients_guard.iter_mut() {
            let mut client_data = client.data.lock().unwrap();
            if let ClientSessionState::NameWasSet { name, ready_to_start, entity_player_id } = &mut client_data.state {
                let intial_position = initial_clients_positions.next().expect("Client should have initial position selected");
                let assigned_id = world.create_entity_player(&name, *intial_position, ENTITY_SIZE);
                *ready_to_start = false;
                *entity_player_id = Some(assigned_id);
    
                // Assign seeker role to one entity
                if seeker_client_id == client.id {
                    world.select_entity_as_seeker(assigned_id, SEEKING_MAX_TIME, SEEKING_MAX_TRIES)?;
                }
    
                log::info!("Client '{name}' gets Entity assigned id={assigned_id}");
            }
        }

        Ok(())
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
        let clients_guard = self.client_sessions_handlers.lock().unwrap();
        clients_guard.iter().any(|(_, v)| v.data.lock().unwrap().get_name() == Some(name))
    }

    pub fn get_connections_count(&self) -> usize {
        let clients_guard = self.client_sessions_handlers.lock().unwrap();
        clients_guard.len()
    }

    pub fn are_all_clients_ready(&self) -> bool {
        let clients_guard = self.client_sessions_handlers.lock().unwrap();
        clients_guard.iter().all(|(_, client)| {
            let data_lock = client.data.lock().unwrap();
            match &data_lock.state {
                client_session::ClientSessionState::JustConnected => false,
                client_session::ClientSessionState::NameWasSet { name: _, ready_to_start, entity_player_id: _ } => *ready_to_start,
            }
        })
    }

    pub fn detach_entities_from_clients(&self) {
        let mut clients_guard = self.client_sessions_handlers.lock().unwrap();
        clients_guard.iter_mut().for_each(|(_, client)| {
            let mut client_data_guard = client.data.lock().unwrap();
            if let ClientSessionState::NameWasSet { name: _, ready_to_start, entity_player_id } = &mut client_data_guard.state {
                *ready_to_start = false;
                *entity_player_id = None;
            }
        });

    }
}

impl Default for GameplayState {
    fn default() -> Self {
        Self::Lobby { counting_to_start: None, last_result: None }
    }
}

impl GameplayState {
    pub fn try_transition_from_lobby_to_gamerunning(&mut self) -> Result<(), GameplayStateTransitionError> {
        match self {
            GameplayState::Lobby { counting_to_start: _, last_result: _ } => {
                *self = GameplayState::GameRunning { world: World::new() };
                Ok(())
            },
            GameplayState::GameRunning { world: _ } => Err(GameplayStateTransitionError::AlreadyInState),
            GameplayState::Ending { countdown: _, result: _ } => Err(GameplayStateTransitionError::BadState),
        }
    }
    
    pub fn try_transition_from_gamerunning_to_ending(&mut self, result: GameplayResult) -> Result<(), GameplayStateTransitionError> {
        const ENDING_COUNTDOWN: u32 = 100;

        match self {
            GameplayState::Lobby { counting_to_start: _, last_result: _, } => Err(GameplayStateTransitionError::BadState),
            GameplayState::GameRunning { world: _, } => {
                *self = GameplayState::Ending { countdown: ENDING_COUNTDOWN, result };
                Ok(())
            },
            GameplayState::Ending { countdown: _, result: _, } => Err(GameplayStateTransitionError::AlreadyInState),
        }
    }

        
    pub fn try_transition_from_ending_to_lobby(&mut self) -> Result<(), GameplayStateTransitionError> {
        match self {
            GameplayState::Lobby { counting_to_start: _, last_result: _ } => Err(GameplayStateTransitionError::AlreadyInState),
            GameplayState::GameRunning { world: _ } => Err(GameplayStateTransitionError::BadState),
            GameplayState::Ending { countdown: _, result } => {
                *self = GameplayState::Lobby { counting_to_start: None, last_result: Some(*result) };
                Ok(())
            },
        }
    }

    pub fn unexpected_transition_to_lobby(&mut self) {
        *self = GameplayState::Lobby { counting_to_start: None, last_result: None };
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
            gameplay_state_guard.try_transition_from_lobby_to_gamerunning().unwrap();

            if let GameplayState::GameRunning { world } = &mut *gameplay_state_guard {
                world.create_entity_npc("Tuna", Vector2F::new(10.5, 20.3), Vector2F::new(1.0, 1.0));
            }
        }
    
        tokio::time::sleep(Duration::from_millis(3000)).await;
        server_handler.shutdown().await.unwrap();
    }
}
