//! Ядро симуляции проекта SEE.
//!
//! Этот крейт не должен содержать рендеринг, UI, ввод/вывод или вызовы LLM.
//! Все вычисления должны быть детерминированными (см. AGENTS.md).

pub mod ai_agents;
pub mod character;
pub mod contracts;
pub mod entities;
pub mod factions;
pub mod market;
pub mod production;
pub mod time;
