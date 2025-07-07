use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Represents a chat message that can be sent over the p2p network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub content: String,
    pub timestamp: SystemTime,
    pub sender_id: String,
}

impl NetworkMessage {
    pub fn new(content: String, sender_id: String) -> Self {
        Self {
            content,
            timestamp: SystemTime::now(),
            sender_id,
        }
    }
}
