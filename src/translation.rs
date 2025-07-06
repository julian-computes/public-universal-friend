use crate::llm::Llm;
use anyhow::Result;

/// A Translator translates texts.
pub struct Translator<L: Llm> {
    llm: L,
}

impl<L: Llm> Translator<L> {
    pub fn new(llm: L) -> Self {
        Self { llm }
    }

    /// Translate text into a target language
    pub async fn translate(
        &self,
        text: impl ToString,
        target_language: impl ToString,
    ) -> Result<String> {
        let guidelines = Self::translation_guidelines(target_language);
        let translation = self.llm.run_task(guidelines, text).await?;
        let cleaned = translation.trim();

        Ok(cleaned.to_string())
    }

    fn translation_guidelines(target_language: impl ToString) -> String {
        format!(
            r#"You are a translator. Follow these examples exactly:

Example 1:
Input: "Hello"
Output: Bonjour

Example 2:
Input: "Good morning"
Output: Bonjour

Example 3:
Input: "How are you?"
Output: Comment allez-vous ?

Now translate to {target_language}. Respond with ONLY the translation:"#,
            target_language = target_language.to_string()
        )
    }
}
