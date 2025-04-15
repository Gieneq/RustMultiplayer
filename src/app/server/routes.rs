use std::sync::{Arc, Mutex};

use rand::{seq::IndexedRandom, Rng};

use crate::{game::{math::Vector2F, world::{self, World}}, requests::{ClientRequest, ClientResponse, EntityCheckData, MoveDirection, SetNameError}};

use super::{client_session::{ClientSessionData, ClientSessionId, ClientSessionState, GameplayState}, MultiplayerServerContext};

pub fn route_client_request(
    server_context: Arc<MultiplayerServerContext>,
    client_session_id: ClientSessionId, 
    clieant_session_data: Arc<Mutex<ClientSessionData>>,
    request_str: &str, 
    world: Arc<Mutex<World>>
) -> String {
    let response: ClientResponse = match serde_json::from_str::<ClientRequest>(request_str) {
        Ok(req) => match req {
            ClientRequest::Ping { payload} => {
                ClientResponse::Ping{payload}
            },
            ClientRequest::GetClientSessionId => {
                ClientResponse::GetClientSessionId { id: client_session_id }
            },
            ClientRequest::GetPointsCount => {
                match clieant_session_data.lock() {
                    Ok(sessiod_data_guard) => {
                        ClientResponse::GetPointsCount { points_count: sessiod_data_guard.points }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::GetClientSessionData => {
                match clieant_session_data.lock() {
                    Ok(sessiod_data_guard) => {
                        ClientResponse::GetClientSessionData { data: sessiod_data_guard.clone() }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::SetName { new_name } => {
                set_name_route(server_context, clieant_session_data, new_name)
            },
            ClientRequest::SetReady { ready: set_to_ready } => {
                match clieant_session_data.lock() {
                    Ok(mut sessiod_data_guard) => {
                        match &mut sessiod_data_guard.state {
                            ClientSessionState::JustConnected => {
                                ClientResponse::BadState
                            },
                            ClientSessionState::NameWasSet { name: _, gameplay_state } => match gameplay_state {
                                GameplayState::Lobby { ready } => {
                                    // set ready
                                    *ready = set_to_ready;
                                    ClientResponse::SetReady { was_set: set_to_ready }
                                },
                                GameplayState::Ingame { entity_player_id: _ } => ClientResponse::BadState,
                            },
                        }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::GetEntityId => {
                match clieant_session_data.lock() {
                    Ok(sessiod_data_guard) => {
                        let entity_player_id = sessiod_data_guard.get_entity_player_id();
                        ClientResponse::GetEntityId { id: entity_player_id }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::WorldCheck => {
                match world.lock() {
                    Ok(world_guard) => {
                        ClientResponse::WorldCheck { 
                            entities: EntityCheckData::vec_from_iter(world_guard.iter_entities())
                        }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::ServerCheck => {
                match world.lock() {
                    Ok(world_guard) => {
                        let players_count = world_guard.iter_entities().filter(|e| e.is_player()).count();
                        ClientResponse::ServerCheck { 
                            msg: "Hello from server!".to_string(),
                            connections: players_count
                        }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::Move{dir} => {
                let player_id = clieant_session_data.lock().unwrap().get_entity_player_id();
                let was_moved = if let Some(player_id) = player_id {
                    match world.lock() {
                        Ok(mut world_guard) => {
                            if let Some((player_pos, player_moving)) = world_guard.get_entity_by_id(player_id).map(|player| (player.position, player.is_moving())) {
                                if player_moving {
                                    // can move only after not moving
                                    false
                                } else {
                                    let next_player_pos = player_pos + match dir {
                                        MoveDirection::Up => Vector2F::new(0.0, 1.0),
                                        MoveDirection::Down => Vector2F::new(0.0, -1.0),
                                        MoveDirection::Left => Vector2F::new(-1.0, 0.0),
                                        MoveDirection::Right => Vector2F::new(1.0, 0.0),
                                    } * world::TILE_SIZE;
                                    
                                    world_guard.try_start_move_entity_to(player_id, next_player_pos).is_ok()
                                }
                            } else {
                                false
                            }
                        }
                        Err(_) => {
                            false
                        }
                    }
                } else {
                    false
                };

                ClientResponse::Move {
                    started: was_moved
                }
            },
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

    match clieant_session_data.lock() {
        Ok(mut sessiod_data_guard) => {
            if sessiod_data_guard.state == ClientSessionState::JustConnected {
                sessiod_data_guard.state = ClientSessionState::NameWasSet { name: new_name, gameplay_state: GameplayState::Lobby { ready: false } };
                ClientResponse::SetName { result: Ok(()) }
            } else {
                ClientResponse::BadState
            }
        },
        Err(e) => {
            ClientResponse::OtherError { err: e.to_string() }
        }
    }
}