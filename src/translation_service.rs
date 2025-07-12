use anyhow::Result;
use tokio::sync::{OnceCell, mpsc};
use tracing::{debug, error, warn};

use crate::entities::chat::Message;
use crate::llm::get_llm;
use crate::translation::Translator;

static IS_TRANSLATION_WORKER_DISABLED: OnceCell<bool> = OnceCell::const_new();

#[derive(Debug, Clone)]
pub struct TranslationRequest {
    pub message_id: u64,
    pub content: String,
    pub target_language: String,
}

#[derive(Debug, Clone)]
pub struct TranslationResponse {
    pub message_id: u64,
    pub translation: String,
    pub language: String,
}

pub struct TranslationService {
    pub request_tx: mpsc::UnboundedSender<TranslationRequest>,
    pub response_rx: mpsc::UnboundedReceiver<TranslationResponse>,
}

impl TranslationService {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::unbounded_channel::<TranslationRequest>();
        let (response_tx, response_rx) = mpsc::unbounded_channel::<TranslationResponse>();

        // Spawn background translation worker
        tokio::spawn(translation_worker(request_rx, response_tx));

        Self {
            request_tx,
            response_rx,
        }
    }

    pub fn request_translation(&self, message: &Message, target_language: String) -> Result<()> {
        let request = TranslationRequest {
            message_id: message.id,
            content: message.content.clone(),
            target_language,
        };

        self.request_tx
            .send(request)
            .map_err(|e| anyhow::anyhow!("Failed to send translation request: {}", e))?;

        Ok(())
    }

    pub fn try_recv_translation(&mut self) -> Option<TranslationResponse> {
        self.response_rx.try_recv().ok()
    }
}

pub fn disable_translation_worker() -> Result<()> {
    IS_TRANSLATION_WORKER_DISABLED.set(true)?;
    Ok(())
}

async fn translation_worker(
    mut request_rx: mpsc::UnboundedReceiver<TranslationRequest>,
    response_tx: mpsc::UnboundedSender<TranslationResponse>,
) {
    if let Some(is_translation_worker_disabled) = IS_TRANSLATION_WORKER_DISABLED.get() {
        if *is_translation_worker_disabled {
            debug!("Translation disabled");
            return;
        }
    }

    debug!("Translation worker started");

    // Initialize translator once for the worker
    let translator = match get_llm().await {
        Ok(llm) => Translator::new(llm.clone()),
        Err(e) => {
            error!("Failed to initialize translator: {}", e);
            return;
        }
    };

    while let Some(request) = request_rx.recv().await {
        debug!(
            "Processing translation request for message {}",
            request.message_id
        );

        match translator
            .translate(&request.content, &request.target_language)
            .await
        {
            Ok(translation) => {
                let response = TranslationResponse {
                    message_id: request.message_id,
                    translation,
                    language: request.target_language,
                };

                if let Err(e) = response_tx.send(response) {
                    error!("Failed to send translation response: {}", e);
                }
            }
            Err(e) => {
                warn!(
                    "Translation failed for message {}: {}",
                    request.message_id, e
                );
            }
        }
    }

    debug!("Translation worker stopped");
}
