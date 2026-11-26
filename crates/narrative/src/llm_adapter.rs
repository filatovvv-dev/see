//! Адаптер LLM: trait клиента и заглушка для тестов.
//!
//! Реальный HTTP-клиент (через `reqwest`/`tokio`) появится в следующих PR.

use std::fmt;

/// Ошибка LLM-адаптера.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    /// Сеть недоступна (для будущего HTTP-клиента).
    NetworkUnavailable,
    /// Превышен лимит запросов.
    RateLimited,
    /// Некорректный ответ.
    InvalidResponse(String),
    /// Внутренняя ошибка.
    Internal(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::NetworkUnavailable => write!(f, "сеть недоступна"),
            LlmError::RateLimited => write!(f, "превышен лимит запросов"),
            LlmError::InvalidResponse(msg) => write!(f, "некорректный ответ: {msg}"),
            LlmError::Internal(msg) => write!(f, "внутренняя ошибка: {msg}"),
        }
    }
}

impl std::error::Error for LlmError {}

/// Клиент LLM. Реализации: [`MockLlmClient`] (заглушка), позже — реальный HTTP.
pub trait LlmClient: Send + Sync {
    /// Генерирует текст по промпту.
    fn generate(&self, prompt: &str) -> Result<String, LlmError>;
}

/// Заглушка клиента: детерминированный ответ без сети и случайности.
///
/// Никогда не возвращает ошибку. Либо отвечает фиксированной строкой
/// ([`MockLlmClient::new`]), либо эхом промпта `"[mock] <prompt>"`
/// ([`MockLlmClient::default_mock`]).
#[derive(Debug, Clone, Default)]
pub struct MockLlmClient {
    /// Фиксированный ответ; `None` — эхо промпта с пометкой `[mock]`.
    response: Option<String>,
}

impl MockLlmClient {
    /// Создаёт заглушку, возвращающую `response` на любой промпт.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: Some(response.into()),
        }
    }

    /// Создаёт заглушку по умолчанию: ответ `"[mock] <prompt>"`.
    pub fn default_mock() -> Self {
        Self { response: None }
    }
}

impl LlmClient for MockLlmClient {
    fn generate(&self, prompt: &str) -> Result<String, LlmError> {
        Ok(match &self.response {
            Some(fixed) => fixed.clone(),
            None => format!("[mock] {prompt}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_client_returns_mock_response() {
        let out = MockLlmClient::default_mock().generate("test");
        assert_eq!(out, Ok("[mock] test".to_string()));
    }

    #[test]
    fn mock_client_custom_response() {
        let out = MockLlmClient::new("hello").generate("x");
        assert_eq!(out, Ok("hello".to_string()));
    }

    #[test]
    fn llm_error_display() {
        assert!(LlmError::NetworkUnavailable.to_string().contains("сеть"));
        assert!(LlmError::RateLimited.to_string().contains("лимит"));
        assert!(LlmError::Internal("x".into()).to_string().contains("x"));
    }
}
