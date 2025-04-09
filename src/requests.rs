use std::sync::{Arc, Mutex};

use serde::{
    Deserialize, 
    Serialize
};

use crate::game::{
    math::Vector2F, 
    world::{Entity, EntityId, World}
};

#[derive(Serialize, Deserialize)]
pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    GetId,
    WorldCheck,
    Healthcheck,
    Move {
        dir: MoveDirection
    },
}

#[derive(Serialize, Deserialize)]
pub struct EntityCheckData {
    pub position: Vector2F,
    pub size: Vector2F,
    pub color: [u8; 3],
    pub id: EntityId,
    pub name: String,
    pub is_npc: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientResponse {
    GetId {
        id: EntityId
    },
    WorldCheck {
        entities: Vec<EntityCheckData>
    },
    Healthcheck {
        msg: String
    },
    BadRequest {
        err: String
    },
    OtherError {
        err: String
    },
    Move {
        started: bool
    },
}



impl EntityCheckData {
    fn vec_from_iter<'a, I: Iterator<Item = &'a Entity>>(iter: I) -> Vec<Self> {
        iter.map(|e| {
            EntityCheckData {
                name: e.name.clone(),
                id: e.id,
                color: e.color,
                position: e.position,
                is_npc: !e.is_player(),
                size: e.size
            }
        })
        .collect()
    }
}

pub fn route_request(player_id: EntityId, request_str: &str, world: Arc<Mutex<World>>) -> String {
    let response: ClientResponse = match serde_json::from_str::<ClientRequest>(request_str) {
        Ok(req) => match req {
            ClientRequest::GetId => {
                ClientResponse::GetId { id: player_id }
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
            ClientRequest::Healthcheck => {
                match world.lock() {
                    Ok(world_guard) => {
                        let players_count = world_guard.iter_entities().filter(|e| e.is_player()).count();
                        ClientResponse::Healthcheck { msg: format!("Hello from server! Players active {players_count}.") }
                    },
                    Err(e) => {
                        ClientResponse::OtherError { err: e.to_string() }
                    }
                }
            },
            ClientRequest::Move{dir} => {
                let was_moved = match world.lock() {
                    Ok(mut world_guard) => {
                        let (player_pos, player_moving) = {
                            let player = world_guard.get_entity_by_id(player_id).unwrap();
                            (player.position, player.is_moving())
                        };
                        if player_moving {
                            // can move only after not moving
                            false
                        } else {
                            let next_player_pos = player_pos + match dir {
                                MoveDirection::Up => Vector2F::new(0.0, 1.0),
                                MoveDirection::Down => Vector2F::new(0.0, -1.0),
                                MoveDirection::Left => Vector2F::new(-1.0, 0.0),
                                MoveDirection::Right => Vector2F::new(1.0, 0.0),
                            } * 5.0;
                            
                            if world_guard.is_tile_occupied(&next_player_pos) {
                                false
                            } else {
                                world_guard.try_start_move_entity_to(player_id, next_player_pos).is_ok()
                            }
                        }
                    }
                    Err(_) => {
                        false
                    }
                };
                ClientResponse::Move {
                    started: was_moved
                }
            }
        },
        Err(e) => ClientResponse::BadRequest { err: format!("request={request_str}, reason={e}") },
    };

    serde_json::to_string(&response).expect("Could not serialize response")
}