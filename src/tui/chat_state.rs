use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect, Size},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};
use std::collections::HashSet;
use tui_scrollview::{ScrollView, ScrollViewState};

use crate::config::Config;
use crate::entities::chat::Chat;
use crate::p2p::{ChatGroup, ChatNetworkService, NetworkError, NetworkEvent, NetworkMessage};
use crate::room_manager::Room;
use crate::translation_service::{TranslationRequest, TranslationService};
use crate::tui::{AppState, State};

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug)]
pub struct ChatState {
    pub chat: Chat,
    pub input: String,
    pub translation_requests_sent: HashSet<u64>,
    pub room: Room,
    pub chat_group: ChatGroup,
    pub network_service: ChatNetworkService,
    pub pending_outgoing_messages: Vec<String>,
    pub subscribed: bool,
    pub connection_status: ConnectionStatus,
    pub show_translations: bool,
    pub messages_scroll_state: ScrollViewState,
    pub translations_scroll_state: ScrollViewState,
}

impl ChatState {
    pub fn with_room(room: Room) -> Self {
        let chat_group = room.to_chat_group();
        let mut network_service = ChatNetworkService::new();

        // Initialize the background network task
        network_service.initialize_channels();

        Self {
            chat: Chat::new(),
            input: String::new(),
            translation_requests_sent: HashSet::new(),
            room,
            chat_group,
            network_service,
            pending_outgoing_messages: Vec::new(),
            subscribed: false,
            connection_status: ConnectionStatus::Connecting,
            show_translations: true, // Default to showing translations
            messages_scroll_state: ScrollViewState::default(),
            translations_scroll_state: ScrollViewState::default(),
        }
    }

    fn scroll_to_bottom(&mut self) {
        // Auto-scroll to the bottom by setting scroll position to max
        self.messages_scroll_state.scroll_to_bottom();
        self.translations_scroll_state.scroll_to_bottom();
    }
}

impl State for ChatState {
    fn handle_key_event(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        config: &Config,
    ) -> Result<Option<AppState>> {
        match (key, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Ok(Some(AppState::Quit)),
            (KeyCode::Char('t'), KeyModifiers::CONTROL) => {
                // Toggle translations panel (only if AI is not disabled)
                if !config.disable_ai {
                    self.show_translations = !self.show_translations;
                }
                Ok(None)
            }
            (KeyCode::Char(c), KeyModifiers::NONE) => {
                self.input.push(c);
                Ok(None)
            }
            (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.input.push(c);
                Ok(None)
            }
            (KeyCode::Backspace, _) => {
                self.input.pop();
                Ok(None)
            }
            (KeyCode::Enter, _) => {
                if !self.input.is_empty() {
                    let content = self.input.clone();
                    let _message = self
                        .chat
                        .add_message(content.clone(), config.username.clone())?;

                    // Auto-scroll to bottom when new message is added
                    self.scroll_to_bottom();

                    // Queue message for network broadcasting
                    self.pending_outgoing_messages.push(content);

                    self.input.clear();
                }
                Ok(None)
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                // Scroll up in messages
                self.messages_scroll_state.scroll_up();
                self.translations_scroll_state.scroll_up();
                Ok(None)
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                // Scroll down in messages
                self.messages_scroll_state.scroll_down();
                self.translations_scroll_state.scroll_down();
                Ok(None)
            }
            (KeyCode::PageUp, _) => {
                // Scroll up by page
                for _ in 0..10 {
                    self.messages_scroll_state.scroll_up();
                    self.translations_scroll_state.scroll_up();
                }
                Ok(None)
            }
            (KeyCode::PageDown, _) => {
                // Scroll down by page
                for _ in 0..10 {
                    self.messages_scroll_state.scroll_down();
                    self.translations_scroll_state.scroll_down();
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, config: &Config) {
        // Main vertical layout: messages area and input at bottom
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(f.area());

        let messages_area = main_chunks[0];
        let input_area = main_chunks[1];

        // Render input at bottom (full width)
        render_input_box(f, self, input_area);

        // Determine if we should show translations (AI enabled and user wants to see them)
        if self.show_translations && !config.disable_ai {
            // Split messages area horizontally: messages | translations
            let message_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(messages_area);

            render_messages_pane(f, self, message_chunks[0]);
            render_translation_pane(f, self, message_chunks[1], config);
        } else {
            // Show only messages (full width)
            render_messages_pane(f, self, messages_area);
        }
    }

    fn update(&mut self, translation_service: &mut TranslationService, config: &Config) {
        // Process any completed translations
        while let Some(response) = translation_service.try_recv_translation() {
            self.chat
                .update_translation(response.message_id, response.translation);
        }

        // Request translation for messages that need it and haven't been requested yet
        // Only if AI is not disabled
        if !config.disable_ai {
            for message in &self.chat.messages {
                if message.translation.is_none()
                    && !self.translation_requests_sent.contains(&message.id)
                {
                    let request = TranslationRequest {
                        message_id: message.id,
                        content: message.content.clone(),
                        target_language: config.target_language.clone(),
                    };
                    if let Err(e) = translation_service.request_tx.send(request) {
                        tracing::warn!("Failed to request translation: {}", e);
                    } else {
                        // Mark this message as having a translation request sent
                        self.translation_requests_sent.insert(message.id);
                    }
                }
            }
        }

        // Handle network operations via background task
        // Subscribe to chat group if we haven't already
        if !self.subscribed && self.network_service.command_tx.is_some() {
            if let Err(e) = self.network_service.subscribe(self.chat_group.clone()) {
                tracing::warn!("Failed to subscribe to chat group: {}", e);
            }
            // Mark as subscription attempted to prevent spamming
            self.subscribed = true;
        }

        // Send pending outgoing messages via background task
        for content in self.pending_outgoing_messages.drain(..) {
            let network_message = NetworkMessage::new(content, config.username.clone());

            if let Err(e) = self.network_service.send_message(network_message) {
                tracing::warn!("Failed to queue network message: {}", e);
            }
        }

        // Process incoming network events
        while let Ok(Some(event)) = self.network_service.try_receive_event() {
            match event {
                NetworkEvent::MessageReceived(network_message) => {
                    // Add received message to chat
                    if let Err(e) = self
                        .chat
                        .add_message(network_message.content, network_message.sender_id)
                    {
                        tracing::warn!("Failed to add received message: {}", e);
                    } else {
                        // Auto-scroll to bottom when new message is received
                        self.scroll_to_bottom();
                    }
                }
                NetworkEvent::Subscribed(group) => {
                    tracing::info!("Successfully subscribed to chat group: {:?}", group);
                    self.connection_status = ConnectionStatus::Connected;
                }
                NetworkEvent::Error(error) => {
                    tracing::warn!("Network error: {:?}", error);
                    // Reset subscription state on connection-related errors
                    match error {
                        NetworkError::SubscriptionLost | NetworkError::ChannelClosed => {
                            self.subscribed = false;
                            self.connection_status = ConnectionStatus::Disconnected;
                        }
                        NetworkError::NetworkCreationFailed(ref msg)
                        | NetworkError::SubscriptionFailed(ref msg) => {
                            self.subscribed = false;
                            self.connection_status = ConnectionStatus::Error(msg.clone());
                        }
                        NetworkError::SendFailed(_) | NetworkError::SerializationFailed(_) => {
                            // Don't reset subscription for temporary send/serialization failures
                            // Keep current connection status
                        }
                    }
                }
            }
        }
    }
}

fn render_messages_pane(f: &mut Frame, chat_state: &mut ChatState, area: Rect) {
    // Check if we need to show error details
    let has_error = matches!(chat_state.connection_status, ConnectionStatus::Error(_));

    let constraints = if has_error {
        vec![Constraint::Min(0), Constraint::Length(2)]
    } else {
        vec![Constraint::Min(0)]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let connection_indicator = match &chat_state.connection_status {
        ConnectionStatus::Connecting => "Connecting...",
        ConnectionStatus::Connected => "Connected",
        ConnectionStatus::Disconnected => "Disconnected",
        ConnectionStatus::Error(_) => "Error",
    };

    let title = format!(
        "Messages - {} [{}]",
        chat_state.room.name, connection_indicator
    );

    render_with_scroll_state(
        f,
        chat_state,
        chunks[0],
        title,
        |msg| msg.display_original(),
        ScrollType::Messages,
    );

    // Show error details if there's an error
    if has_error {
        if let ConnectionStatus::Error(ref error_msg) = chat_state.connection_status {
            let error_widget = Paragraph::new(error_msg.as_str())
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Connection Error"),
                );
            f.render_widget(error_widget, chunks[1]);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ScrollType {
    Messages,
    Translations,
}

fn render_with_scroll_state<F>(
    f: &mut Frame,
    chat_state: &mut ChatState,
    area: Rect,
    title: String,
    content_extractor: F,
    scroll_type: ScrollType,
) where
    F: Fn(&crate::entities::chat::Message) -> String,
{
    // Extract the data we need before borrowing the scroll state
    let content: Vec<String> = chat_state
        .chat
        .messages
        .iter()
        .flat_map(|msg| {
            let text = content_extractor(msg);
            wrap_text(&text, area.width.saturating_sub(4) as usize)
        })
        .collect();

    let content_height = content.len() as u16;
    let content_size = Size::new(area.width.saturating_sub(2), content_height.max(1));

    let mut scroll_view = ScrollView::new(content_size);

    // Render each line as a separate paragraph
    for (i, line) in content.iter().enumerate() {
        let line_area = Rect::new(0, i as u16, area.width.saturating_sub(2), 1);
        scroll_view.render_widget(Paragraph::new(line.as_str()), line_area);
    }

    // Render with border
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner_area = block.inner(area);
    block.render(area, f.buffer_mut());

    let scroll_state = match scroll_type {
        ScrollType::Messages => &mut chat_state.messages_scroll_state,
        ScrollType::Translations => &mut chat_state.translations_scroll_state,
    };

    scroll_view.render(inner_area, f.buffer_mut(), scroll_state);
}

fn render_input_box(f: &mut Frame, chat_state: &ChatState, area: Rect) {
    let input = Paragraph::new(chat_state.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .wrap(Wrap { trim: false });

    f.render_widget(input, area);
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();

        // If adding this word would exceed the width, start a new line
        if current_width + word_len + 1 > max_width && !current_line.is_empty() {
            lines.push(current_line.trim().to_string());
            current_line = word.to_string();
            current_width = word_len;
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_len;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line.trim().to_string());
    }

    if lines.is_empty() {
        vec![text.to_string()]
    } else {
        lines
    }
}

fn render_translation_pane(f: &mut Frame, chat_state: &mut ChatState, area: Rect, config: &Config) {
    let title = format!("Translations ({})", config.target_language);

    render_with_scroll_state(
        f,
        chat_state,
        area,
        title,
        |msg| msg.display_translation(),
        ScrollType::Translations,
    );
}
