use crate::llm::{get_llm, initialize_ai};
use crate::translation::Translator;
use anyhow::Result;
use kalosm::language::*;

mod llm;
mod translation;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    initialize_ai().await?;

    let llm = get_llm().await?;
    let translator = Translator::new(llm.clone());

    loop {
        let input = prompt_input("\n> ")?;
        let translation = translator
            .translate(input.trim().to_string(), "French".to_string())
            .await?;
        println!("{translation}");
    }
}
