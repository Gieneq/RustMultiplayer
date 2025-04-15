use std::time::SystemTime;
use chrono::{
    Local,
    DateTime
};

use super::client_session::ClientSessionId;

#[derive(Debug)]
pub enum ChatMessageSenderType {
    Server,
    Client {
        id: ClientSessionId,
        name: String,
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub receive_time: SystemTime,
    pub sender_type: ChatMessageSenderType,
    pub msg: String,
}

impl ChatMessage {
    pub fn new_from_server(msg: String) -> Self {
        Self { 
            receive_time: SystemTime::now(), 
            sender_type: ChatMessageSenderType::Server, 
            msg 
        }
    }

    pub fn new_from_client(msg: String, id: ClientSessionId, name: String) -> Self {
        Self { 
            receive_time: SystemTime::now(), 
            sender_type: ChatMessageSenderType::Client { id, name }, 
            msg 
        }
    }
}

impl std::fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let datetime: DateTime<Local> = self.receive_time.into();
        let name = match &self.sender_type {
            ChatMessageSenderType::Server => "SERVER",
            ChatMessageSenderType::Client { id: _, name } => name.as_str(),
        };
        
        write!(f, "{} <{}> {}", datetime.format("%H:%M"), name, self.msg)
    }
}

