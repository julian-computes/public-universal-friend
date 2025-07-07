use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{Frame, Terminal, backend::Backend};
use std::time::Duration;

use crate::translation_service::TranslationService;

pub mod chat_state;
pub mod main_menu_state;

use chat_state::ChatState;
use main_menu_state::MainMenuState;

#[derive(Debug)]
pub enum AppState {
    MainMenu(MainMenuState),
    Chat(ChatState),
    Quit,
}

impl Default for AppState {
    fn default() -> Self {
        Self::MainMenu(MainMenuState::new())
    }
}

pub trait State {
    fn handle_key_event(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<AppState>>;
    fn render(&self, f: &mut Frame);
    fn update(&mut self, translation_service: &mut TranslationService);
}

pub struct TuiApp {
    pub state: AppState,
    pub translation_service: TranslationService,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            translation_service: TranslationService::new(),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        let new_state = match &mut self.state {
            AppState::MainMenu(main_menu_state) => {
                main_menu_state.handle_key_event(key, modifiers)?
            }
            AppState::Chat(chat_state) => chat_state.handle_key_event(key, modifiers)?,
            AppState::Quit => None,
        };

        if let Some(new_state) = new_state {
            self.state = new_state;
        }

        Ok(())
    }

    pub fn render(&self, f: &mut Frame) {
        match &self.state {
            AppState::MainMenu(main_menu_state) => main_menu_state.render(f),
            AppState::Chat(chat_state) => chat_state.render(f),
            AppState::Quit => {} // No rendering needed for quit state
        }
    }

    pub fn update(&mut self) {
        match &mut self.state {
            AppState::MainMenu(main_menu_state) => {
                main_menu_state.update(&mut self.translation_service);
            }
            AppState::Chat(chat_state) => {
                chat_state.update(&mut self.translation_service);
            }
            AppState::Quit => {}
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Update state (process translations, etc.)
            self.update();

            terminal.draw(|f| self.render(f))?;

            if matches!(self.state, AppState::Quit) {
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
}
