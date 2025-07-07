use super::{ChatGroup, NetworkMessage};

/// Commands that can be sent to the background network task
#[derive(Debug, Clone)]
pub enum NetworkCommand {
    Subscribe(ChatGroup),
    SendMessage(NetworkMessage),
    Unsubscribe,
}

/// Events that the background network task sends back to the UI
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    MessageReceived(NetworkMessage),
    Error(NetworkError),
    Subscribed(ChatGroup),
}

/// Network error types
#[derive(Debug, Clone)]
pub enum NetworkError {
    SubscriptionLost,
    ChannelClosed,
    SendFailed(String),
    NetworkCreationFailed(String),
    SubscriptionFailed(String),
    SerializationFailed(String),
}
