use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::collections::HashSet;

use crate::entities::chat::Chat;
use crate::translation_service::{TranslationRequest, TranslationService};
use crate::tui::{AppState, State};

#[derive(Debug, Clone)]
pub struct ChatState {
    pub chat: Chat,
    pub input: String,
    pub translation_requests_sent: HashSet<u64>,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            chat: Chat::new(),
            input: String::new(),
            translation_requests_sent: HashSet::new(),
        }
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ChatState {
    fn handle_key_event(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<AppState>> {
        match (key, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Ok(Some(AppState::Quit)),
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
                    let _message = self.chat.add_message(content)?;
                    self.input.clear();
                    // Note: Translation request will be handled in update() method
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());

        render_input_pane(f, self, chunks[0]);
        render_translation_pane(f, self, chunks[1]);
    }

    fn update(&mut self, translation_service: &mut TranslationService) {
        // Process any completed translations
        while let Some(response) = translation_service.try_recv_translation() {
            self.chat
                .update_translation(response.message_id, response.translation);
        }

        // Request translation for messages that need it and haven't been requested yet
        for message in &self.chat.messages {
            if message.translation.is_none()
                && !self.translation_requests_sent.contains(&message.id)
            {
                let request = TranslationRequest {
                    message_id: message.id,
                    content: message.content.clone(),
                    target_language: self.chat.target_language.clone(),
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
}

fn render_input_pane(f: &mut Frame, chat_state: &ChatState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let messages: Vec<ListItem> = chat_state
        .chat
        .messages
        .iter()
        .map(|msg| ListItem::new(Line::from(Span::raw(msg.display_original()))))
        .collect();

    let messages_list =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));

    f.render_widget(messages_list, chunks[0]);

    let input = Paragraph::new(chat_state.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));

    f.render_widget(input, chunks[1]);
}

fn render_translation_pane(f: &mut Frame, chat_state: &ChatState, area: Rect) {
    let translation_messages: Vec<ListItem> = chat_state
        .chat
        .messages
        .iter()
        .map(|msg| ListItem::new(Line::from(Span::raw(msg.display_translation()))))
        .collect();

    let title = format!("Translations ({})", chat_state.chat.target_language);
    let translations_list =
        List::new(translation_messages).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(translations_list, area);
}
