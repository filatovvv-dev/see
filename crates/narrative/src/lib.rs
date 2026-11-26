//! Текстовый слой проекта SEE: движок повествования поверх LLM-клиента.

use std::fmt;

pub mod cache;
pub mod llm_adapter;
pub mod prompt_builder;
pub mod template_gen;
pub mod validator;

use llm_adapter::{LlmClient, LlmError};
use prompt_builder::build_prompt;

/// Ошибка движка повествования.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrativeError {
    /// Ошибка LLM-клиента.
    Llm(LlmError),
    /// Пустой контекст.
    EmptyContext,
}

impl fmt::Display for NarrativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NarrativeError::Llm(e) => write!(f, "ошибка LLM: {e}"),
            NarrativeError::EmptyContext => write!(f, "пустой контекст"),
        }
    }
}

impl std::error::Error for NarrativeError {}

/// Движок повествования: принимает клиент, строит промпт, возвращает текст.
pub struct NarrativeEngine {
    client: Box<dyn LlmClient>,
}

impl NarrativeEngine {
    /// Создаёт движок с заданным LLM-клиентом.
    pub fn new(client: Box<dyn LlmClient>) -> Self {
        Self { client }
    }

    /// Генерирует сцену по контексту.
    ///
    /// Пустой (или состоящий из пробелов) контекст отклоняется до вызова
    /// клиента с [`NarrativeError::EmptyContext`].
    pub fn generate_scene(&self, context: &str) -> Result<String, NarrativeError> {
        if context.trim().is_empty() {
            return Err(NarrativeError::EmptyContext);
        }
        self.client
            .generate(&build_prompt(context))
            .map_err(NarrativeError::Llm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_adapter::MockLlmClient;

    fn engine() -> NarrativeEngine {
        NarrativeEngine::new(Box::new(MockLlmClient::default_mock()))
    }

    #[test]
    fn engine_with_mock_generates_text() {
        let out = engine().generate_scene("Мерказ").unwrap();
        assert!(out.starts_with("[mock] "));
        assert!(out.contains("Мерказ"));
        assert!(out.contains("рассказчик"));
    }

    #[test]
    fn engine_empty_context_fails() {
        assert_eq!(
            engine().generate_scene(""),
            Err(NarrativeError::EmptyContext)
        );
    }

    #[test]
    fn engine_whitespace_context_fails() {
        assert_eq!(
            engine().generate_scene("   "),
            Err(NarrativeError::EmptyContext)
        );
    }

    #[test]
    fn narrative_error_display() {
        assert!(NarrativeError::EmptyContext.to_string().contains("пустой"));
        assert!(
            NarrativeError::Llm(LlmError::RateLimited)
                .to_string()
                .contains("лимит")
        );
    }
}
