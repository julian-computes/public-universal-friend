use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    backend::Backend, layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
    Terminal,
};
use std::time::Duration;

pub struct TuiApp {
    pub should_quit: bool,
    pub input: String,
    pub messages: Vec<String>,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            should_quit: false,
            input: String::new(),
            messages: Vec::new(),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if self.should_quit {
                break;
            }

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key_event(key.code, key.modifiers);
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match (key, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Char(c), KeyModifiers::NONE) => self.input.push(c),
            (KeyCode::Char(c), KeyModifiers::SHIFT) => self.input.push(c),
            (KeyCode::Backspace, _) => {
                self.input.pop();
            }
            (KeyCode::Enter, _) => {
                if !self.input.is_empty() {
                    let message = format!("User: {}", self.input);
                    self.messages.push(message);
                    self.input.clear();
                }
            }
            _ => {}
        }
    }

    fn ui(&self, f: &mut Frame) {
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
            .messages
            .iter()
            .map(|msg| ListItem::new(Line::from(Span::raw(msg))))
            .collect();

        let messages_list =
            List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));

        f.render_widget(messages_list, chunks[0]);

        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Input"));

        f.render_widget(input, chunks[1]);
    }

    fn render_translation_pane(&self, f: &mut Frame, area: Rect) {
        let translation_messages: Vec<ListItem> = self
            .messages
            .iter()
            .map(|msg| ListItem::new(Line::from(Span::raw(format!("{} (translated)", msg)))))
            .collect();

        let translations_list = List::new(translation_messages)
            .block(Block::default().borders(Borders::ALL).title("Translations"));

        f.render_widget(translations_list, area);
    }
}
