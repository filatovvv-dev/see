//! Сборка промптов для LLM.

/// Строит промпт для LLM из контекста.
pub fn build_prompt(context: &str) -> String {
    format!("Ты — рассказчик в мире SEE. Сгенерируй короткую сцену по контексту:\n{context}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_contains_context() {
        let p = build_prompt("Мерказ");
        assert!(p.contains("Мерказ"));
        assert!(p.contains("рассказчик"));
    }
}
