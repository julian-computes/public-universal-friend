use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::time::Duration;

use crate::chat_state::{ChatState, Message};
use crate::translation_service::TranslationService;

#[derive(Debug, Clone)]
pub enum AppState {
    Chat,
}

pub struct TuiApp {
    pub state: AppState,
    pub should_quit: bool,
    pub chat_state: ChatState,
    pub translation_service: TranslationService,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            state: AppState::Chat,
            should_quit: false,
            chat_state: ChatState::new(),
            translation_service: TranslationService::new(),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Process any completed translations
            self.process_translations();

            terminal.draw(|f| self.ui(f))?;

            if self.should_quit {
                break;
            }

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key_event(key.code, key.modifiers)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn process_translations(&mut self) {
        while let Some(response) = self.translation_service.try_recv_translation() {
            self.chat_state
                .update_translation(response.message_id, response.translation);
        }
    }

    fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match self.state {
            AppState::Chat => self.handle_chat_key_event(key, modifiers)?,
        }
        Ok(())
    }

    fn handle_chat_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match (key, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Char(c), KeyModifiers::NONE) => self.chat_state.input.push(c),
            (KeyCode::Char(c), KeyModifiers::SHIFT) => self.chat_state.input.push(c),
            (KeyCode::Backspace, _) => {
                self.chat_state.input.pop();
            }
            (KeyCode::Enter, _) => {
                if !self.chat_state.input.is_empty() {
                    let content = self.chat_state.input.clone();
                    let target_language = self.chat_state.target_language.clone();
                    let message = self.chat_state.add_message(content)?;
                    let message_id = message.id;
                    let message_content = message.content.clone();
                    self.chat_state.input.clear();

                    // Request translation for the new message
                    let request = crate::translation_service::TranslationRequest {
                        message_id,
                        content: message_content,
                        target_language,
                    };
                    if let Err(e) = self.translation_service.request_tx.send(request) {
                        tracing::warn!("Failed to request translation: {}", e);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn ui(&self, f: &mut Frame) {
        match self.state {
            AppState::Chat => self.render_chat_ui(f),
        }
    }

    fn render_chat_ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(f.area());

        self.render_input_pane(f, chunks[0]);
        self.render_translation_pane(f, chunks[1]);
    }

    fn render_input_pane(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        let messages: Vec<ListItem> = self
            .chat_state
            .messages
            .iter()
            .map(|msg| ListItem::new(Line::from(Span::raw(msg.display_original()))))
            .collect();

        let messages_list =
            List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));

        f.render_widget(messages_list, chunks[0]);

        let input = Paragraph::new(self.chat_state.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Input"));

        f.render_widget(input, chunks[1]);
    }

    fn render_translation_pane(&self, f: &mut Frame, area: Rect) {
        let translation_messages: Vec<ListItem> = self
            .chat_state
            .messages
            .iter()
            .map(|msg| ListItem::new(Line::from(Span::raw(msg.display_translation()))))
            .collect();

        let title = format!("Translations ({})", self.chat_state.target_language);
        let translations_list = List::new(translation_messages)
            .block(Block::default().borders(Borders::ALL).title(title));

        f.render_widget(translations_list, area);
    }
}
