use serde::{
    Deserialize, 
    Serialize
};

use crate::{
    app::server::{client_session::{
        ClientSessionData, 
        ClientSessionId
    }, GameplayResult, GameplayState}, 
    game::{
        math::Vector2F, 
        world::{
            Entity, 
            EntityId, PlayerRole
        }
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
pub enum GameplayStateBrief {
    Lobby {
        counting_to_start: Option<u32>,
        last_result: Option<GameplayResult>
    },
    GameRunning,
    Ending {
        countdown: u32,
        result: GameplayResult
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UncoverResult {
    // None - out of range
    // Some - in range with info:
    //   * true - was hider
    //   * false - was NPC
    pub was_hider: Option<bool>,
}

impl From<&GameplayState> for GameplayStateBrief {
    fn from(value: &GameplayState) -> Self {
        match value {
            GameplayState::Lobby { counting_to_start: counting, last_result: res } => GameplayStateBrief::Lobby {
                counting_to_start: *counting,
                last_result: *res
            },
            GameplayState::GameRunning { world: _ } => GameplayStateBrief::GameRunning,
            GameplayState::Ending { countdown: counting , result: res} => GameplayStateBrief::Ending {
                countdown: *counting, 
                result: *res
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    Ping {
        payload: Option<String>
    },
    SendChatMessage {
        msg: String,
    },
    ReadChatMessages {
        max_count: Option<usize>
    },
    GetClientSessionId,
    GetClientSessionData,
    GetPointsCount,
    SetName {
        new_name: Option<String>,
    },
    SetReady {
        ready: bool
    },
    GetEntityId,
    WorldCheck,
    ServerCheck,
    CheckGameplayState,
    Move {
        dir: MoveDirection
    },
    GetRole,
    GetStartCountdownTime,
    TryUncover {
        id: EntityId
    }
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
pub enum SetNameError {
    NameEmpty,
    NameAlreadyUsed,
    NameGenerateExhausted,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientResponse {
    Ping {
        payload: Option<String>
    },
    SendChatMessage {
        sent: bool
    },
    ReadChatMessages {
        results: Vec<String>,
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
        result: Result<(), SetNameError>
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
    CheckGameplayState {
        state: GameplayStateBrief
    },
    BadRequest {
        err: String
    },
    BadState,
    EntityNotPlayer {
        id: EntityId
    },
    EntityNotFound {
        id: EntityId
    },
    OtherError {
        err: String
    },
    Move {
        started: bool
    },
    GetRole {
        role: PlayerRole
    },
    GetStartCountdownTime {
        time: Option<u32>
    },
    TryUncover {
        uncover_result: UncoverResult
    }
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

impl UncoverResult {
    pub fn was_in_range(&self) -> bool {
        self.was_hider.is_some()
    }
}