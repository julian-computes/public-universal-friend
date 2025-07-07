use anyhow::{Error, anyhow};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub content: String,
    pub timestamp: SystemTime,
    pub translation: Option<String>,
    pub translation_language: Option<String>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

impl Message {
    pub fn new(content: String) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        Self {
            id,
            content,
            timestamp: SystemTime::now(),
            translation: None,
            translation_language: None,
        }
    }

    pub fn with_translation(mut self, translation: String, language: String) -> Self {
        self.translation = Some(translation);
        self.translation_language = Some(language);
        self
    }

    pub fn display_original(&self) -> String {
        format!("User: {}", self.content)
    }

    pub fn display_translation(&self) -> String {
        match &self.translation {
            Some(trans) => format!("User: {trans}"),
            None => "Translating...".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatState {
    pub messages: Vec<Message>,
    pub input: String,
    pub target_language: String,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            target_language: "French".to_string(),
        }
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, content: String) -> Result<&Message, Error> {
        let message = Message::new(content);
        self.messages.push(message);
        self.messages.last().ok_or(anyhow!("No message found"))
    }

    pub fn update_translation(&mut self, message_id: u64, translation: String) {
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            msg.translation = Some(translation);
            msg.translation_language = Some(self.target_language.clone());
        }
    }

    pub fn set_target_language(&mut self, language: String) {
        self.target_language = language;
        // Clear existing translations when language changes
        for msg in &mut self.messages {
            msg.translation = None;
            msg.translation_language = None;
        }
    }
}
