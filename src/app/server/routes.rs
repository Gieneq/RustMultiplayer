use std::sync::{Arc, Mutex};

use crate::{game::{math::Vector2F, world::{self, EntityId, World}}, requests::{ClientRequest, ClientResponse, EntityCheckData, MoveDirection}};

pub fn route_client_request(player_id: EntityId, request_str: &str, world: Arc<Mutex<World>>) -> String {
    // TODO probably player id will be passed to JSON - connection should not mean there is player entity
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
                            } * world::TILE_SIZE;
                            
                            world_guard.try_start_move_entity_to(player_id, next_player_pos).is_ok()
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