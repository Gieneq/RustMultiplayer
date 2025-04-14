use serde::{
    Deserialize, 
    Serialize
};

use crate::{app::server::client_session::{ClientSessionData, ClientSessionId}, game::{
    math::Vector2F, 
    world::{
        Entity, 
        EntityId
    }
}};

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
    Ping {
        payload: Option<String>
    },
    GetClientSessionId,
    GetClientSessionData,
    GetPointsCount,
    SetName {
        new_name: String,
    },
    SetReady {
        ready: bool
    },
    GetEntityId,
    WorldCheck,
    ServerCheck,
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
    Ping {
        payload: Option<String>
    },
    GetClientSessionId {
        id: ClientSessionId
    },
    GetClientSessionData {
        data: ClientSessionData
    },
    GetPointsCount {
        points_count: u32
    },
    SetName {
        was_set: bool
    },
    SetReady {
        was_set: bool
    },
    GetEntityId {
        id: Option<EntityId>
    },
    WorldCheck {
        entities: Vec<EntityCheckData>
    },
    ServerCheck {
        msg: String,
        connections: usize,
    },
    BadRequest {
        err: String
    },
    BadState,
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