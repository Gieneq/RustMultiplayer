use std::sync::{Arc, Mutex};

use rand::{seq::IndexedRandom, Rng};

use crate::{game::{math::Vector2F, world::{self, EntityId, PlayerRole, World}}, requests::{ClientRequest, ClientResponse, EntityCheckData, MoveDirection, SetNameError, UncoverResult}};

use super::{chat::ChatMessage, client_session::{ClientSessionData, ClientSessionId, ClientSessionState}, MultiplayerServerContext};

pub fn route_client_request(
    server_context: Arc<MultiplayerServerContext>,
    client_session_id: ClientSessionId, 
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    request_str: &str
) -> String {
    let response: ClientResponse = match serde_json::from_str::<ClientRequest>(request_str) {
        Ok(req) => match req {
            ClientRequest::Ping { payload} => {
                ClientResponse::Ping{payload}
            },
            ClientRequest::ReadChatMessages { max_count } => {
                read_chat_messages_route(max_count, server_context)
            },
            ClientRequest::SendChatMessage { msg } => {
                send_message_route(msg, client_session_id, clieant_session_data, server_context)
            },
            ClientRequest::GetClientSessionId => {
                ClientResponse::GetClientSessionId { id: client_session_id }
            },
            ClientRequest::GetPointsCount => {
                let sessiod_data_guard = clieant_session_data.lock().unwrap();
                ClientResponse::GetPointsCount { points_count: sessiod_data_guard.points }
            },
            ClientRequest::GetClientSessionData => {
                let sessiod_data_guard = clieant_session_data.lock().unwrap();
                ClientResponse::GetClientSessionData { data: sessiod_data_guard.clone() }
            },
            ClientRequest::SetName { new_name } => {
                set_name_route(server_context, clieant_session_data, new_name)
            },
            ClientRequest::SetReady { ready: set_to_ready } => {
                set_ready_route(set_to_ready, clieant_session_data)
            },
            ClientRequest::GetEntityId => {
                let sessiod_data_guard = clieant_session_data.lock().unwrap();
                ClientResponse::GetEntityId { id: sessiod_data_guard.get_entity_player_id() }
            },
            ClientRequest::WorldCheck => {
                world_check_route(server_context)
            },
            ClientRequest::ServerCheck => {
                server_check_route(server_context)
            },
            ClientRequest::Move{dir} => {
                move_route(dir, clieant_session_data, server_context)
            },
            ClientRequest::CheckGameplayState => {
                gameplay_state_route(server_context)
            },
            ClientRequest::GetRole => {
                get_role_route(clieant_session_data, server_context)
            },
            ClientRequest::GetStartCountdownTime => {
                get_countdown_time_route(server_context)
            },
            ClientRequest::TryUncover { id } => {
                try_uncover_route(server_context, client_session_id, clieant_session_data, id)
            }
        },
        Err(e) => ClientResponse::BadRequest { err: format!("request={request_str}, reason={e}") },
    };

    serde_json::to_string(&response).expect("Could not serialize response")
}

fn try_generate_name(server_context: Arc<MultiplayerServerContext>) -> Option<String> {
    const MAX_RETRIES_COUNT: usize = 100;
    const CORE_NAMES:[&str;5] = [
        "Beaver",
        "Goose",
        "Horse",
        "Pig",
        "Cat"
    ];

    let mut exhaust_counter = 0;
    let mut rng = rand::rng();
    loop {
        exhaust_counter += 1;
        let core_name = CORE_NAMES.choose(&mut rng).unwrap();
        let name_num: i32 = rng.random_range(0..255);
        let new_name = format!("{core_name}_{name_num}");
    
        if !server_context.is_name_used(&new_name) {
            return Some(new_name);
        } 
        
        if exhaust_counter > MAX_RETRIES_COUNT {
            break;
        }
    }

    None
}

fn set_name_route(
    server_context: Arc<MultiplayerServerContext>,
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    new_name: Option<String>
) -> ClientResponse {
    let new_name = if let Some(new_name) = new_name {
        if new_name.is_empty() {
            return ClientResponse::SetName { result: Err(SetNameError::NameEmpty) };
        } else if server_context.is_name_used(&new_name) {
            return ClientResponse::SetName { result: Err(SetNameError::NameAlreadyUsed) };
        } else {
            new_name
        }
    } else if let Some(new_name) = try_generate_name(server_context) {
        new_name
    } else {
        return ClientResponse::SetName { result: Err(SetNameError::NameGenerateExhausted) };
    };

    let mut sessiod_data_guard = clieant_session_data.lock().unwrap();
    if sessiod_data_guard.state == ClientSessionState::JustConnected {
        sessiod_data_guard.state = ClientSessionState::NameWasSet { name: new_name, ready_to_start: false, entity_player_id: None };
        ClientResponse::SetName { result: Ok(()) }
    } else {
        ClientResponse::BadState
    }
}

fn send_message_route(
    msg: String, 
    client_session_id: ClientSessionId, 
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    server_context: Arc<MultiplayerServerContext>
) -> ClientResponse {
    let message  = {
        let sesssion_data_guard = clieant_session_data.lock().unwrap();
        if let Some(name) = sesssion_data_guard.get_name() {
            ChatMessage::new_from_client(msg, client_session_id, name.to_string())
        } else {
            return ClientResponse::SendChatMessage { sent: false };
        }
    };

    let mut server_context_guard = server_context.chat.lock().unwrap();
    server_context_guard.push(message);
    ClientResponse::SendChatMessage { sent: true }
}

fn read_chat_messages_route(
    max_count: Option<usize>,
    server_context: Arc<MultiplayerServerContext>
) -> ClientResponse {
    let server_context_guard = server_context.chat.lock().unwrap();
    if server_context_guard.is_empty() {
        ClientResponse::ReadChatMessages { results: vec![] }
    } else {
        let elements_count = if let Some(max_count) = max_count {
            max_count.min(server_context_guard.len())
        } else {
            server_context_guard.len()
        };

        // Get last messages
        let results = server_context_guard
            .iter()
            .rev()
            .take(elements_count)
            .map(ToString::to_string)
            .collect();

        ClientResponse::ReadChatMessages { results }
    }
}

fn set_ready_route(
    set_to_ready: bool,
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
) -> ClientResponse {
    let mut sessiod_data_guard = clieant_session_data.lock().unwrap();
    match &mut sessiod_data_guard.state {
        ClientSessionState::JustConnected => ClientResponse::BadState,
        ClientSessionState::NameWasSet { name: _, ready_to_start, entity_player_id: _ } => {
            // set ready
            *ready_to_start = set_to_ready;
            ClientResponse::SetReady { was_set: set_to_ready }
        },
    }
}

fn gameplay_state_route(server_context: Arc<MultiplayerServerContext>) -> ClientResponse {
    let server_context_guard = server_context.gameplay_state.lock().unwrap();
    ClientResponse::CheckGameplayState { state: (&*server_context_guard).into() }
}

fn world_check_route(server_context: Arc<MultiplayerServerContext>) -> ClientResponse {
    let gameplay_state_guard = server_context.gameplay_state.lock().unwrap();
    match &*gameplay_state_guard {
        super::GameplayState::Lobby { counting_to_start: _, last_result:_ } => ClientResponse::BadState,
        super::GameplayState::GameRunning { world } => {
            ClientResponse::WorldCheck { 
                entities: EntityCheckData::vec_from_iter(world.iter_entities())
            }
        },
        super::GameplayState::Ending { countdown: _, result: _ } => ClientResponse::BadState,
    }
}

fn server_check_route(server_context: Arc<MultiplayerServerContext>) -> ClientResponse {
    let connections_count = server_context.get_connections_count();
    ClientResponse::ServerCheck { 
        msg: "Hello from server!".to_string(),
        connections: connections_count
    }
}

fn move_route(
    dir: MoveDirection,
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    server_context: Arc<MultiplayerServerContext>
)  -> ClientResponse {
    let player_entity_id = clieant_session_data.lock().unwrap().get_entity_player_id();
    let player_entity_id = match player_entity_id {
        Some(id) => id,
        None => {
            return ClientResponse::BadState;
        },
    };
    let mut gameplay_state_guard = server_context.gameplay_state.lock().unwrap();

    match &mut *gameplay_state_guard {
        super::GameplayState::Lobby { counting_to_start: _, last_result:_ } => ClientResponse::BadState,
        super::GameplayState::Ending { countdown: _ , result: _ } => ClientResponse::BadState,
        super::GameplayState::GameRunning { world } => {
            let player_info = world
                .get_entity_by_id(player_entity_id)
                .map(|player| (player.position, player.is_moving()));
            
            let was_moved = if let Some((player_pos, player_moving)) = player_info {
                if player_moving {
                    // Can move only after not moving
                    false
                } else {
                    let next_player_pos = player_pos + match dir {
                        MoveDirection::Up => Vector2F::new(0.0, 1.0),
                        MoveDirection::Down => Vector2F::new(0.0, -1.0),
                        MoveDirection::Left => Vector2F::new(-1.0, 0.0),
                        MoveDirection::Right => Vector2F::new(1.0, 0.0),
                    } * world::TILE_SIZE;
                    
                    world.try_start_move_entity_to(player_entity_id, next_player_pos).is_ok()
                }
            } else {
                false
            };

            ClientResponse::Move { started: was_moved }
        },
    }
}

fn get_role_route(
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    server_context: Arc<MultiplayerServerContext>
) -> ClientResponse {
    // Entity idd is assigned if player is in InGamestag
    let entity_id = {
        let sessiod_data_guard = clieant_session_data.lock().unwrap();
        match sessiod_data_guard.get_entity_player_id() {
            Some(id) => id,
            None => {
                // EntityIdis not assignedyet, probably still lobby stage
                return ClientResponse::BadState;
            }
        }
    };
    
    let gameplay_state_guard = server_context.gameplay_state.lock().unwrap();
    match &*gameplay_state_guard {
        super::GameplayState::Lobby { counting_to_start: _, last_result:_ } => ClientResponse::BadState,
        super::GameplayState::GameRunning { world } => {
            match world.get_entity_by_id(entity_id) {
                Some(e) => {
                    // Rather should not happen this player will not be PlayerEntity type
                    match e.get_player_role() {
                        Some(player_role) => ClientResponse::GetRole { role: *player_role},
                        None => ClientResponse::EntityNotPlayer { id: entity_id }
                    }
                },
                None => ClientResponse::EntityNotFound { id: entity_id }
            }
        },
        super::GameplayState::Ending { countdown: _ , result: _ } => ClientResponse::BadState,
    }
}

fn get_countdown_time_route(server_context: Arc<MultiplayerServerContext>) -> ClientResponse {
    let gameplay_state_guard = server_context.gameplay_state.lock().unwrap();
    match &*gameplay_state_guard {
        super::GameplayState::Lobby { counting_to_start, last_result:_ } => {
            ClientResponse::GetStartCountdownTime { time: *counting_to_start }
        },
        super::GameplayState::GameRunning { world: _ } => ClientResponse::BadState,
        super::GameplayState::Ending { countdown: _ , result: _ } => ClientResponse::BadState,
    }
}

fn try_uncover_route(
    server_context: Arc<MultiplayerServerContext>,
    client_session_id: ClientSessionId, 
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    uncovering_entity_id: EntityId
) -> ClientResponse {
    let mut gameplay_state_guard = server_context.gameplay_state.lock().unwrap();
    match &mut *gameplay_state_guard {
        super::GameplayState::GameRunning { world } => {
            // Find entity of the client
            let client_entity_id = {
                let client_data_guard = clieant_session_data.lock().unwrap();
                match client_data_guard.get_entity_player_id() {
                    Some(entitiy_id) => entitiy_id,
                    None => {
                        return ClientResponse::OtherError { err: "Client handler no entity attached".to_string() };
                    }
                }
            };

            // Check if can uncover
            let can_uncover = {
                let (client_entity, other_entity) = match (world.get_entity_by_id(client_entity_id), world.get_entity_by_id(uncovering_entity_id)) {
                    (Some(entity_c), Some(entity_o)) => (entity_c, entity_o),
                    (None, _) => {
                        return ClientResponse::EntityNotFound { id: client_entity_id }
                    }, 
                    (_, None) => {
                        return ClientResponse::EntityNotFound { id: uncovering_entity_id }
                    }, 
                };

                World::is_entity_inrange(client_entity.position, other_entity.position)
            };

            // Can use unwrap, both entities are proved now
            let was_hider = if can_uncover {
                let was_hider = world.get_entity_by_id(uncovering_entity_id).unwrap()
                    .get_player_role()
                    .is_some_and(|role| matches!(role, PlayerRole::Hider { stats: _ }));

                if was_hider {
                    // Uncover hider player do not remove
                    let hider_entity = world.get_entity_by_id_mut(uncovering_entity_id).unwrap();
                    hider_entity.set_hider_covered(false).unwrap();
                    Some(true)

                } else {
                    // Remove NPC and punish seeker
                    world.remove_entity(uncovering_entity_id).unwrap();

                    let seeker_entity = world.get_entity_by_id_mut(client_entity_id).unwrap();
                    seeker_entity.punish_seeker().unwrap();
                    Some(false)
                }
            } else {
                // Notify not in range
                None
            };
            
            ClientResponse::TryUncover { 
                uncover_result: UncoverResult { was_hider } 
            }
        },
        _ => ClientResponse::BadState,
    }
}