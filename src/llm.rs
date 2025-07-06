use anyhow::Result;
use kalosm::language::{ChatModelExt, Llama};
use tokio::sync::OnceCell;
use tracing::{debug, instrument};

static LLAMA: OnceCell<Llama> = OnceCell::const_new();

/// Llm completes tasks by generating text.
pub trait Llm {
    async fn run_task(
        &self,
        task_description: impl ToString,
        task_input_text: impl ToString,
    ) -> Result<String>;
}

impl Llm for Llama {
    /// Generate text using guidelines and input text.
    async fn run_task(&self, guidelines: impl ToString, input: impl ToString) -> Result<String> {
        self.task(guidelines)
            .run(input)
            .await
            .map_err(anyhow::Error::from)
    }
}

/// Ensure that all AI models are loaded and ready for inference.
#[instrument]
pub async fn initialize_ai() -> Result<()> {
    debug!("Initializing AI");
    let llm = get_llm().await?;
    debug!("Created Llama instance");

    llm.task("Say hello back.").run("Hello!").await?;
    debug!("Warmed Llama instance");

    Ok(())
}

/// Get the lazily initialized Llama instance.
pub async fn get_llm() -> Result<&'static Llama> {
    LLAMA
        .get_or_try_init(|| async { Llama::new_chat().await })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize model: {}", e))
}
