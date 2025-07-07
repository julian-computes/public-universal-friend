use p2panda_net::{FromNetwork, Network, ToNetwork};

use crate::p2p::{ChatGroup, NetworkCommand, NetworkError, NetworkEvent, NetworkMessage};
use super::network::create_network;

/// State for the network background task
struct NetworkTaskState {
    network: Network<ChatGroup>,
    current_subscription: Option<(
        tokio::sync::mpsc::Sender<ToNetwork>,
        tokio::sync::mpsc::Receiver<FromNetwork>,
    )>,
    subscription_ready: Option<tokio::sync::oneshot::Receiver<()>>,
    event_tx: tokio::sync::mpsc::UnboundedSender<NetworkEvent>,
}

impl NetworkTaskState {
    pub async fn new(event_tx: tokio::sync::mpsc::UnboundedSender<NetworkEvent>) -> Option<Self> {
        match create_network().await {
            Ok(network) => {
                tracing::info!("Network created successfully in background task");
                Some(Self {
                    network,
                    current_subscription: None,
                    subscription_ready: None,
                    event_tx,
                })
            }
            Err(e) => {
                tracing::error!("Failed to create network in background task: {}", e);
                let _ = event_tx.send(NetworkEvent::Error(NetworkError::NetworkCreationFailed(
                    e.to_string(),
                )));
                None
            }
        }
    }

    pub async fn handle_command(&mut self, command: NetworkCommand) {
        match command {
            NetworkCommand::Subscribe(chat_group) => {
                self.handle_subscribe_command(chat_group).await;
            }
            NetworkCommand::SendMessage(message) => {
                self.handle_send_message_command(message).await;
            }
            NetworkCommand::Unsubscribe => {
                self.handle_unsubscribe_command().await;
            }
        }
    }

    async fn handle_subscribe_command(&mut self, chat_group: ChatGroup) {
        tracing::info!("Background task: subscribing to {:?}", chat_group);

        match self.network.subscribe(chat_group.clone()).await {
            Ok((tx, rx, ready)) => {
                tracing::info!("Successfully subscribed to chat group");
                self.current_subscription = Some((tx, rx));
                self.subscription_ready = Some(ready);
                let _ = self.event_tx.send(NetworkEvent::Subscribed(chat_group));
            }
            Err(e) => {
                tracing::error!("Failed to subscribe to chat group: {}", e);
                let _ = self
                    .event_tx
                    .send(NetworkEvent::Error(NetworkError::SubscriptionFailed(
                        e.to_string(),
                    )));
            }
        }
    }

    async fn handle_send_message_command(&mut self, message: NetworkMessage) {
        tracing::info!("Background task: sending message {:?}", message);

        if let Some((tx, _)) = &self.current_subscription {
            match serde_json::to_vec(&message) {
                Ok(serialized) => {
                    let to_network = ToNetwork::Message { bytes: serialized };
                    if let Err(e) = tx.send(to_network).await {
                        tracing::error!("Failed to send message: {}", e);
                        let _ = self
                            .event_tx
                            .send(NetworkEvent::Error(NetworkError::SendFailed(e.to_string())));
                    } else {
                        tracing::info!("Message sent successfully");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                    let _ =
                        self.event_tx
                            .send(NetworkEvent::Error(NetworkError::SerializationFailed(
                                e.to_string(),
                            )));
                }
            }
        } else {
            tracing::warn!("No active subscription to send message");
            let _ = self
                .event_tx
                .send(NetworkEvent::Error(NetworkError::ChannelClosed));
        }
    }

    async fn handle_unsubscribe_command(&mut self) {
        tracing::info!("Background task: unsubscribing");
        self.current_subscription = None;
        self.subscription_ready = None;
    }

    pub fn handle_network_message(&mut self, from_network: Option<FromNetwork>) {
        match from_network {
            Some(FromNetwork::GossipMessage {
                bytes,
                delivered_from,
            }) => {
                tracing::info!(
                    "Received gossip message: {} bytes from {:?}",
                    bytes.len(),
                    delivered_from
                );
                match serde_json::from_slice::<NetworkMessage>(&bytes) {
                    Ok(network_message) => {
                        tracing::info!("Parsed network message: {:?}", network_message);
                        let _ = self
                            .event_tx
                            .send(NetworkEvent::MessageReceived(network_message));
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse network message: {}", e);
                        let _ = self.event_tx.send(NetworkEvent::Error(
                            NetworkError::SerializationFailed(e.to_string()),
                        ));
                    }
                }
            }
            Some(FromNetwork::SyncMessage {
                header: _,
                payload,
                delivered_from,
            }) => {
                tracing::debug!("Received sync message from {:?}", delivered_from);
                if let Some(bytes) = payload {
                    match serde_json::from_slice::<NetworkMessage>(&bytes) {
                        Ok(network_message) => {
                            tracing::info!("Parsed sync message: {:?}", network_message);
                            let _ = self
                                .event_tx
                                .send(NetworkEvent::MessageReceived(network_message));
                        }
                        Err(e) => {
                            tracing::debug!("Sync message payload is not a chat message: {}", e);
                        }
                    }
                }
            }
            None => {
                tracing::warn!("Network message channel closed");
                self.current_subscription = None;
                self.subscription_ready = None;
                let _ = self
                    .event_tx
                    .send(NetworkEvent::Error(NetworkError::SubscriptionLost));
            }
        }
    }
}

/// Background task that handles all network operations
pub async fn network_background_task(
    mut command_rx: tokio::sync::mpsc::UnboundedReceiver<NetworkCommand>,
    event_tx: tokio::sync::mpsc::UnboundedSender<NetworkEvent>,
) {
    tracing::info!("Network background task started");

    // Initialize network task state
    let mut state = match NetworkTaskState::new(event_tx).await {
        Some(state) => state,
        // Error already sent via event_tx
        None => return,
    };

    // Main task loop
    loop {
        tokio::select! {
            // Handle incoming commands
            command = command_rx.recv() => {
                match command {
                    Some(cmd) => {
                        state.handle_command(cmd).await;
                    }
                    None => {
                        tracing::info!("Command channel closed, ending background task");
                        break;
                    }
                }
            }

            // Handle incoming network messages
            from_network = async {
                if let Some((_, rx)) = &mut state.current_subscription {
                    rx.recv().await
                } else {
                    // If no subscription, just wait indefinitely
                    std::future::pending().await
                }
            } => {
                state.handle_network_message(from_network);
            }
        }
    }

    tracing::info!("Network background task ended");
}
