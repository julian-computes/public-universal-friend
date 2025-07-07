use anyhow::Result;

use super::task::network_background_task;
use crate::p2p::{ChatGroup, NetworkCommand, NetworkEvent, NetworkMessage};

/// Handles network communication for a specific chat group using background tasks.
#[derive(Debug)]
pub struct ChatNetworkService {
    /// Send commands to the background network task
    pub command_tx: Option<tokio::sync::mpsc::UnboundedSender<NetworkCommand>>,
    /// Receive events from the background network task  
    pub event_rx: Option<tokio::sync::mpsc::UnboundedReceiver<NetworkEvent>>,
}

impl ChatNetworkService {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            event_rx: None,
        }
    }

    /// Initialize the network service with command/event channels
    pub fn initialize_channels(&mut self) -> tokio::sync::mpsc::UnboundedSender<NetworkCommand> {
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        self.command_tx = Some(command_tx.clone());
        self.event_rx = Some(event_rx);

        // Spawn the background network task
        tokio::spawn(network_background_task(command_rx, event_tx));

        command_tx
    }

    /// Send a message to the network via the background task
    pub fn send_message(&self, message: NetworkMessage) -> Result<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::SendMessage(message))
                .map_err(|e| anyhow::anyhow!("Failed to send network command: {}", e))?;
        }
        Ok(())
    }

    /// Subscribe to a chat group via the background task
    pub fn subscribe(&self, chat_group: ChatGroup) -> Result<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(NetworkCommand::Subscribe(chat_group))
                .map_err(|e| anyhow::anyhow!("Failed to send subscribe command: {}", e))?;
        }
        Ok(())
    }

    /// Try to receive a network event (non-blocking)
    pub fn try_receive_event(&mut self) -> Result<Option<NetworkEvent>> {
        if let Some(rx) = &mut self.event_rx {
            if let Ok(event) = rx.try_recv() {
                return Ok(Some(event));
            }
        }
        Ok(None)
    }
}
