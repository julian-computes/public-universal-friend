use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::config::Config;
use crate::room_manager::{Room, copy_to_clipboard};
use crate::translation_service::TranslationService;
use crate::tui::{AppState, State, chat_state::ChatState};

#[derive(Debug, Clone)]
pub enum MenuOption {
    CreateRoom,
    JoinRoom,
}

#[derive(Debug)]
pub struct MainMenuState {
    pub selected_option: MenuOption,
    pub room_name_input: String,
    pub room_id_input: String,
    pub input_mode: InputMode,
    pub status_message: String,
}

#[derive(Debug, Clone)]
pub enum InputMode {
    Menu,
    CreatingRoom,
    JoiningRoom,
}

impl Default for MainMenuState {
    fn default() -> Self {
        Self {
            selected_option: MenuOption::CreateRoom,
            room_name_input: String::new(),
            room_id_input: String::new(),
            input_mode: InputMode::Menu,
            status_message: String::new(),
        }
    }
}

impl MainMenuState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for MainMenuState {
    fn handle_key_event(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
        _config: &Config,
    ) -> Result<Option<AppState>> {
        match (key, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => Ok(Some(AppState::Quit)),
            _ => match self.input_mode {
                InputMode::Menu => self.handle_menu_input(key, modifiers),
                InputMode::CreatingRoom => self.handle_create_room_input(key, modifiers),
                InputMode::JoiningRoom => self.handle_join_room_input(key, modifiers),
            },
        }
    }

    fn render(&mut self, f: &mut Frame, _config: &Config) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        // Title
        let title = Paragraph::new("Public Universal Friend")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Main content area split to show status if needed
        let content_chunks = if !self.status_message.is_empty() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(chunks[1])
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0)])
                .split(chunks[1])
        };

        // Show status message if present
        if !self.status_message.is_empty() {
            let status = Paragraph::new(self.status_message.as_str())
                .style(Style::default().fg(Color::Green))
                .block(Block::default().borders(Borders::ALL).title("Status"));
            f.render_widget(status, content_chunks[0]);
        }

        // Main content
        let content_area = if !self.status_message.is_empty() {
            content_chunks[1]
        } else {
            content_chunks[0]
        };

        match self.input_mode {
            InputMode::Menu => self.render_menu(f, content_area),
            InputMode::CreatingRoom => self.render_create_room(f, content_area),
            InputMode::JoiningRoom => self.render_join_room(f, content_area),
        }

        // Help text
        let help_text = match self.input_mode {
            InputMode::Menu => "↑/↓ or j/k: Navigate, Enter: Select, Ctrl+Q: Quit",
            InputMode::CreatingRoom => "Type room name, Enter: Create, Esc: Back",
            InputMode::JoiningRoom => "Type room ID, Enter: Join, Esc: Back",
        };
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);
    }

    fn update(&mut self, _translation_service: &mut TranslationService, _config: &Config) {}
}

impl MainMenuState {
    fn handle_menu_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<Option<AppState>> {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_option = MenuOption::CreateRoom;
                Ok(None)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected_option = MenuOption::JoinRoom;
                Ok(None)
            }
            KeyCode::Enter => {
                match self.selected_option {
                    MenuOption::CreateRoom => {
                        self.input_mode = InputMode::CreatingRoom;
                        self.room_name_input.clear();
                    }
                    MenuOption::JoinRoom => {
                        self.input_mode = InputMode::JoiningRoom;
                        self.room_id_input.clear();
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn handle_create_room_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<Option<AppState>> {
        match key {
            KeyCode::Esc => {
                self.input_mode = InputMode::Menu;
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.room_name_input.push(c);
                Ok(None)
            }
            KeyCode::Backspace => {
                self.room_name_input.pop();
                Ok(None)
            }
            KeyCode::Enter => {
                if !self.room_name_input.is_empty() {
                    // Create room with BLAKE3 hash
                    let room = Room::new(self.room_name_input.clone());

                    // Copy room identifier to clipboard
                    match copy_to_clipboard(&room.identifier) {
                        Ok(()) => {
                            self.status_message = format!(
                                "Room created! ID copied to clipboard: {}",
                                room.identifier
                            );
                            tracing::info!("Created room: {}", room.identifier);
                        }
                        Err(e) => {
                            self.status_message = format!(
                                "Room created: {} (Clipboard copy failed: {})",
                                room.identifier, e
                            );
                            tracing::warn!("Failed to copy to clipboard: {}", e);
                        }
                    }

                    // Transition to chat with room context
                    Ok(Some(AppState::Chat(ChatState::with_room(room))))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn handle_join_room_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<Option<AppState>> {
        match key {
            KeyCode::Esc => {
                self.input_mode = InputMode::Menu;
                Ok(None)
            }
            KeyCode::Char(c) => {
                self.room_id_input.push(c);
                Ok(None)
            }
            KeyCode::Backspace => {
                self.room_id_input.pop();
                Ok(None)
            }
            KeyCode::Enter => {
                if !self.room_id_input.is_empty() {
                    // Validate and parse room identifier
                    match Room::from_identifier(self.room_id_input.clone()) {
                        Ok(room) => {
                            self.status_message = format!("Joining room: {}", room.name);
                            tracing::info!("Joining room: {}", room.identifier);

                            // Transition to chat with room context
                            Ok(Some(AppState::Chat(ChatState::with_room(room))))
                        }
                        Err(e) => {
                            self.status_message = format!("Invalid room ID: {}", e);
                            tracing::warn!("Failed to parse room ID: {}", e);
                            Ok(None)
                        }
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    fn render_menu(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let _items = vec![
            ListItem::new(Line::from(Span::raw("Create New Room"))),
            ListItem::new(Line::from(Span::raw("Join Existing Room"))),
        ];

        let selected_style = Style::default().fg(Color::Yellow).bg(Color::Blue);
        let normal_style = Style::default().fg(Color::White);

        // Manually highlight the selected item
        let styled_items: Vec<ListItem> = vec![
            ListItem::new(Line::from(Span::styled(
                "Create New Room",
                if matches!(self.selected_option, MenuOption::CreateRoom) {
                    selected_style
                } else {
                    normal_style
                },
            ))),
            ListItem::new(Line::from(Span::styled(
                "Join Existing Room",
                if matches!(self.selected_option, MenuOption::JoinRoom) {
                    selected_style
                } else {
                    normal_style
                },
            ))),
        ];

        let menu_list = List::new(styled_items)
            .block(Block::default().borders(Borders::ALL).title("Main Menu"));

        f.render_widget(menu_list, area.inner(Margin::new(2, 1)));
    }

    fn render_create_room(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let input = Paragraph::new(self.room_name_input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Room Name"));

        f.render_widget(input, chunks[0]);

        let instructions = Paragraph::new("Enter a name for your new chat room.\nA unique room ID will be generated and copied to your clipboard.")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Instructions"));

        f.render_widget(instructions, chunks[1]);
    }

    fn render_join_room(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let input = Paragraph::new(self.room_id_input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Room ID"));

        f.render_widget(input, chunks[0]);

        let instructions = Paragraph::new(
            "Paste the room ID that was shared with you.\nRoom IDs look like: hash-uuid-room-name",
        )
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Instructions"));

        f.render_widget(instructions, chunks[1]);
    }
}
