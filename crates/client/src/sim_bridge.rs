//! Мост между клиентом и `sim_core::World`.
//!
//! Чистая логика без Bevy: продвижение тиков по реальному времени, пауза и
//! скорость. Тестируется без `App`.

use sim_core::entities::{Agent, AgentId, SystemId};
use sim_core::world::{World, WorldEvent};

/// Тиков в секунду реального времени (при `speed = 1.0`).
pub const TICKS_PER_SECOND: f32 = 1.0;

/// Минимальная скорость симуляции.
pub const MIN_SPEED: f32 = 0.1;

/// Максимальная скорость симуляции.
pub const MAX_SPEED: f32 = 10.0;

/// Максимум тиков за один вызов [`SimBridge::update`]: защита от лавины
/// тиков после долгого кадра.
pub const MAX_TICKS_PER_UPDATE: u32 = 100;

/// Мост между Bevy и `sim_core::World`.
pub struct SimBridge {
    /// Мир симуляции.
    pub world: World,
    /// Пауза.
    pub paused: bool,
    /// Скорость: 0.5 — медленно, 1.0 — норма, 2.0 — быстро.
    pub speed: f32,
    /// Накопитель времени до следующего тика (для дробной скорости).
    pub accumulator: f32,
}

impl SimBridge {
    /// Создаёт мост с миром на зерне `seed` и `agent_count` демо-агентами.
    pub fn new(seed: u64, agent_count: usize) -> Self {
        let mut world = World::new(seed);
        for _ in 0..agent_count {
            world.spawn_agent(Agent {
                id: AgentId(0),
                credits: 1000,
                location: SystemId(0),
            });
        }
        Self {
            world,
            paused: false,
            speed: 1.0,
            accumulator: 0.0,
        }
    }

    /// Продвигает симуляцию на `delta_seconds` реального времени.
    ///
    /// На паузе ничего не делает. Иначе накапливает
    /// `delta_seconds * speed * TICKS_PER_SECOND` и выполняет по тику за
    /// каждую целую единицу (не более [`MAX_TICKS_PER_UPDATE`] за вызов).
    /// Неположительные и нечисловые значения игнорируются.
    pub fn update(&mut self, delta_seconds: f32) {
        if self.paused || !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        self.accumulator += delta_seconds * self.speed * TICKS_PER_SECOND;
        let mut ticks = 0;
        while self.accumulator >= 1.0 && ticks < MAX_TICKS_PER_UPDATE {
            self.world.tick();
            self.accumulator -= 1.0;
            ticks += 1;
        }
        if self.accumulator >= 1.0 {
            // Лимит достигнут: остаток отбрасываем, чтобы не копить долг.
            self.accumulator = 0.0;
        }
    }

    /// Переключает паузу.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Устанавливает скорость, ограничивая её диапазоном
    /// [`MIN_SPEED`]..=[`MAX_SPEED`]. `NaN` заменяется на 1.0.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = if speed.is_nan() {
            1.0
        } else {
            speed.clamp(MIN_SPEED, MAX_SPEED)
        };
    }

    /// События за последние `n` тиков (включая текущий), по порядку.
    pub fn recent_events(&self, n: usize) -> Vec<&WorldEvent> {
        if n == 0 {
            return Vec::new();
        }
        let from = self
            .world
            .time
            .tick
            .saturating_sub(u64::try_from(n - 1).unwrap_or(u64::MAX));
        self.world.events_since(from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_world_with_agents() {
        let b = SimBridge::new(42, 5);
        assert_eq!(b.world.agents.len(), 5);
        assert!(!b.paused);
        assert_eq!(b.speed, 1.0);
    }

    #[test]
    fn update_advances_tick_at_normal_speed() {
        let mut b = SimBridge::new(42, 1);
        b.update(1.0);
        assert_eq!(b.world.time.tick, 1);
    }

    #[test]
    fn update_does_nothing_when_paused() {
        let mut b = SimBridge::new(42, 1);
        b.paused = true;
        b.update(1.0);
        assert_eq!(b.world.time.tick, 0);
    }

    #[test]
    fn update_accumulates_fractional_time() {
        let mut b = SimBridge::new(42, 1);
        b.update(0.5);
        assert_eq!(b.world.time.tick, 0);
        b.update(0.5);
        assert_eq!(b.world.time.tick, 1);
    }

    #[test]
    fn update_respects_speed_and_ignores_bad_delta() {
        let mut b = SimBridge::new(42, 1);
        b.set_speed(2.0);
        b.update(1.0);
        assert_eq!(b.world.time.tick, 2);
        b.update(-1.0);
        b.update(f32::NAN);
        assert_eq!(b.world.time.tick, 2);
    }

    #[test]
    fn update_caps_ticks_per_call() {
        let mut b = SimBridge::new(42, 1);
        b.update(10_000.0);
        assert_eq!(b.world.time.tick, u64::from(MAX_TICKS_PER_UPDATE));
        assert_eq!(b.accumulator, 0.0);
    }

    #[test]
    fn toggle_pause_flips_flag() {
        let mut b = SimBridge::new(42, 1);
        b.toggle_pause();
        assert!(b.paused);
        b.toggle_pause();
        assert!(!b.paused);
    }

    #[test]
    fn set_speed_clamps() {
        let mut b = SimBridge::new(42, 1);
        b.set_speed(100.0);
        assert_eq!(b.speed, 10.0);
        b.set_speed(-5.0);
        assert_eq!(b.speed, 0.1);
        b.set_speed(f32::NAN);
        assert_eq!(b.speed, 1.0);
    }

    #[test]
    fn recent_events_returns_last_n() {
        let mut b = SimBridge::new(42, 1);
        b.update(2.0);
        assert!(b.recent_events(1).len() <= 1);
        assert_eq!(b.recent_events(2).len(), 2);
        assert!(b.recent_events(0).is_empty());
    }
}
