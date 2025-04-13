use serde::{
    Deserialize, 
    Serialize
};

use crate::game::{
    math::Vector2F, 
    world::{
        Entity, 
        EntityId
    }
};

#[derive(Debug, Serialize, Deserialize)]
pub enum MoveDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    GetId,
    WorldCheck,
    Healthcheck,
    Move {
        dir: MoveDirection
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityCheckData {
    pub position: Vector2F,
    pub size: Vector2F,
    pub color: [u8; 3],
    pub id: EntityId,
    pub name: String,
    pub is_npc: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientResponse {
    GetId {
        id: EntityId
    },
    WorldCheck {
        entities: Vec<EntityCheckData>
    },
    Healthcheck {
        msg: String,
        connections: usize,
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
    pub fn vec_from_iter<'a, I: Iterator<Item = &'a Entity>>(iter: I) -> Vec<Self> {
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