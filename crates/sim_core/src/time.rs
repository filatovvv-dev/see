//! Игровое время симуляции (проект SEE).
//!
//! Единственный источник времени для `sim_core`. Симуляция оперирует дискретными
//! тиками: 1 тик = 1 игровая минута. Системное время (`SystemTime`, `Instant`)
//! здесь запрещено — см. AGENTS.md, раздел «Главное правило детерминизма».

use serde::{Deserialize, Serialize};

/// Количество тиков в одном игровом часу.
///
/// Один тик соответствует одной игровой минуте, поэтому час содержит 60 тиков.
pub const TICKS_PER_GAME_HOUR: u64 = 60;

/// Дискретное игровое время, выраженное в тиках симуляции.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    /// Текущий номер тика симуляции.
    pub tick: u64,
}

impl GameTime {
    /// Создаёт новое игровое время с нулевым тиком.
    pub fn new() -> Self {
        Self { tick: 0 }
    }

    /// Продвигает игровое время на `delta_ticks` тиков вперёд.
    pub fn advance(&mut self, delta_ticks: u64) {
        self.tick += delta_ticks;
    }

    /// Возвращает текущее время в игровых часах.
    pub fn to_hours(&self) -> f64 {
        self.tick as f64 / TICKS_PER_GAME_HOUR as f64
    }
}

impl Default for GameTime {
    /// Эквивалентно [`GameTime::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_at_zero_tick() {
        assert_eq!(GameTime::new().tick, 0);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(GameTime::default(), GameTime::new());
    }

    #[test]
    fn advance_increments_tick() {
        let mut t = GameTime::new();
        t.advance(10);
        assert_eq!(t.tick, 10);
        t.advance(5);
        assert_eq!(t.tick, 15);
    }

    #[test]
    fn advance_by_zero_is_noop() {
        let mut t = GameTime::new();
        t.advance(7);
        t.advance(0);
        assert_eq!(t.tick, 7);
    }

    #[test]
    fn to_hours_after_120_ticks_is_two() {
        let mut t = GameTime::new();
        t.advance(120);
        assert_eq!(t.to_hours(), 2.0);
    }

    #[test]
    fn to_hours_handles_partial_hour() {
        let mut t = GameTime::new();
        t.advance(30);
        assert!((t.to_hours() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ticks_per_game_hour_constant() {
        assert_eq!(TICKS_PER_GAME_HOUR, 60);
    }

    #[test]
    fn serde_roundtrip() {
        let mut t = GameTime::new();
        t.advance(99);
        let json = serde_json::to_string(&t).expect("serialize");
        let back: GameTime = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, t);
    }
}
