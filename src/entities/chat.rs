use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub content: String,
    pub timestamp: SystemTime,
    pub translation: Option<String>,
    pub translation_language: Option<String>,
    pub sender: String,
}

impl Message {
    pub fn new(content: String, sender: String) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);

        Self {
            id,
            content,
            timestamp: SystemTime::now(),
            translation: None,
            translation_language: None,
            sender,
        }
    }

    pub fn with_translation(mut self, translation: String, language: String) -> Self {
        self.translation = Some(translation);
        self.translation_language = Some(language);
        self
    }

    pub fn display_original(&self) -> String {
        format!("{}: {}", self.sender, self.content)
    }

    pub fn display_translation(&self) -> String {
        match &self.translation {
            Some(trans) => format!("{}: {}", self.sender, trans),
            None => format!("{}: Translating...", self.sender),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub messages: Vec<Message>,
    pub target_language: String,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            target_language: "Spanish".to_string(),
        }
    }
}

impl Chat {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, content: String, sender: String) -> anyhow::Result<&Message> {
        let message = Message::new(content, sender);
        self.messages.push(message);
        self.messages
            .last()
            .ok_or(anyhow::anyhow!("No message found"))
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
