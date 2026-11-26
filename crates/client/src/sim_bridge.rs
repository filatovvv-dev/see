//! Мост между клиентом и `sim_core::World`.
//!
//! Чистая логика без Bevy: продвижение тиков по реальному времени, пауза и
//! скорость. Тестируется без `App`.

use std::collections::BTreeMap;
use std::f32::consts::TAU;

use sim_core::entities::{Agent, AgentId, SystemId};
use sim_core::world::{World, WorldEvent};

/// Тиков в секунду реального времени (при `speed = 1.0`).
pub const TICKS_PER_SECOND: f32 = 1.0;

/// Путь файла сохранения по умолчанию (в текущей директории).
pub const SAVE_PATH: &str = "save.json";

/// Минимальная скорость симуляции.
pub const MIN_SPEED: f32 = 0.1;

/// Максимальная скорость симуляции.
pub const MAX_SPEED: f32 = 10.0;

/// Максимум тиков за один вызов [`SimBridge::update`]: защита от лавины
/// тиков после долгого кадра.
pub const MAX_TICKS_PER_UPDATE: u32 = 100;

/// Амплитуда колебания агента вокруг базовой позиции, px.
pub const WOBBLE_AMPLITUDE: f32 = 10.0;

/// Скорость колебания: радиан на тик.
pub const WOBBLE_SPEED: f32 = 0.1;

/// Палитра агентов (RGBA), выбор по `AgentId % 8`.
const AGENT_PALETTE: [[f32; 4]; 8] = [
    [0.90, 0.30, 0.30, 1.0],
    [0.95, 0.60, 0.20, 1.0],
    [0.95, 0.85, 0.30, 1.0],
    [0.40, 0.80, 0.40, 1.0],
    [0.30, 0.75, 0.85, 1.0],
    [0.35, 0.50, 0.95, 1.0],
    [0.65, 0.40, 0.90, 1.0],
    [0.90, 0.45, 0.75, 1.0],
];

/// Позиция агента на 2D-сцене (клиентская, не часть `sim_core`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentPosition {
    /// Координата X.
    pub x: f32,
    /// Координата Y.
    pub y: f32,
}

/// Цвет агента (RGBA) по `AgentId`: детерминированно из палитры из 8 цветов.
pub fn agent_color(id: AgentId) -> [f32; 4] {
    // `% 8` < 8, поэтому приведение к индексу безопасно.
    AGENT_PALETTE[(id.0 % 8) as usize]
}

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
    /// Поколение мира: увеличивается при каждой успешной загрузке.
    /// Клиент по нему понимает, что спрайты нужно пересоздать. Начинается с 1,
    /// чтобы первая синхронизация спрайтов создала их с нуля.
    pub generation: u64,
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
            generation: 1,
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

    /// Раскладка агентов по кругу радиуса `radius` в порядке `AgentId`.
    ///
    /// Агент с индексом `i` из `n` получает угол `2π·i/n`.
    pub fn agent_layout(&self, radius: f32) -> BTreeMap<AgentId, AgentPosition> {
        let n = self.world.agents.len();
        self.world
            .agents
            .keys()
            .enumerate()
            .map(|(i, &id)| {
                let angle = TAU * i as f32 / n as f32;
                (
                    id,
                    AgentPosition {
                        x: radius * angle.cos(),
                        y: radius * angle.sin(),
                    },
                )
            })
            .collect()
    }

    /// Позиции агентов с учётом текущего тика: к базовой раскладке
    /// добавляется смещение `sin(tick·0.1 + i)·10` по обеим осям.
    ///
    /// Мир не меняется, тик только читается.
    pub fn agent_positions(&self, radius: f32) -> BTreeMap<AgentId, AgentPosition> {
        let tick = self.world.time.tick as f32;
        self.agent_layout(radius)
            .into_iter()
            .enumerate()
            .map(|(i, (id, pos))| {
                let offset = (tick * WOBBLE_SPEED + i as f32).sin() * WOBBLE_AMPLITUDE;
                (
                    id,
                    AgentPosition {
                        x: pos.x + offset,
                        y: pos.y + offset,
                    },
                )
            })
            .collect()
    }

    /// Сохраняет мир в файл `path`.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        self.world.save_to_file(path).map_err(|e| e.to_string())
    }

    /// Загружает мир из файла `path`, целиком заменяя текущий.
    ///
    /// При ошибке текущий мир не меняется.
    pub fn load(&mut self, path: &std::path::Path) -> Result<(), String> {
        let world = World::load_from_file(path).map_err(|e| e.to_string())?;
        self.world = world;
        self.generation += 1;
        Ok(())
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

    #[test]
    fn agent_layout_is_deterministic() {
        let a = SimBridge::new(42, 5);
        let b = SimBridge::new(42, 5);
        assert_eq!(a.agent_layout(200.0), a.agent_layout(200.0));
        assert_eq!(a.agent_layout(200.0), b.agent_layout(200.0));
        assert_eq!(a.agent_layout(200.0).len(), 5);
    }

    #[test]
    fn agent_layout_positions_on_circle() {
        let b = SimBridge::new(42, 7);
        for pos in b.agent_layout(200.0).values() {
            let r2 = pos.x * pos.x + pos.y * pos.y;
            assert!((r2 - 200.0 * 200.0).abs() < 1.0, "r2 = {r2}");
        }
    }

    #[test]
    fn agent_layout_of_empty_world_is_empty() {
        assert!(SimBridge::new(42, 0).agent_layout(200.0).is_empty());
    }

    #[test]
    fn agent_positions_depend_on_tick() {
        let mut b = SimBridge::new(42, 5);
        let base = b.agent_layout(200.0);
        b.update(2.0);
        let moved = b.agent_positions(200.0);
        assert_eq!(moved.len(), base.len());
        assert_ne!(moved, base);
        // Чтение позиций не меняет мир.
        assert_eq!(b.world.time.tick, 2);
    }

    #[test]
    fn agent_color_is_palette_cycle() {
        assert_eq!(agent_color(AgentId(0)), agent_color(AgentId(8)));
        assert_ne!(agent_color(AgentId(0)), agent_color(AgentId(1)));
        assert_eq!(agent_color(AgentId(3)), AGENT_PALETTE[3]);
    }

    #[test]
    fn save_and_load_roundtrip_via_bridge() {
        let path = std::env::temp_dir().join(format!("see_test_save_{}.json", std::process::id()));
        let mut a = SimBridge::new(42, 3);
        a.update(3.0);
        a.save(&path).unwrap();

        let mut b = SimBridge::new(1, 0);
        let loaded = b.load(&path);
        let _ = std::fs::remove_file(&path);
        loaded.unwrap();
        assert_eq!(b.world.time.tick, a.world.time.tick);
        assert_eq!(b.world.agents.len(), 3);
        assert_eq!(b.world.events, a.world.events);
    }

    #[test]
    fn load_missing_file_fails() {
        let path = std::env::temp_dir().join(format!("see_missing_{}.json", std::process::id()));
        let mut b = SimBridge::new(42, 2);
        b.update(1.0);
        assert!(b.load(&path).is_err());
        // Неудачная загрузка не затрагивает текущий мир.
        assert_eq!(b.world.time.tick, 1);
        assert_eq!(b.world.agents.len(), 2);
    }

    #[test]
    fn load_increments_generation() {
        let path = std::env::temp_dir().join(format!("see_test_gen_{}.json", std::process::id()));
        let a = SimBridge::new(42, 2);
        a.save(&path).unwrap();
        let mut b = SimBridge::new(1, 0);
        let before = b.generation;
        let loaded = b.load(&path);
        let _ = std::fs::remove_file(&path);
        loaded.unwrap();
        assert_eq!(b.generation, before + 1);
    }

    #[test]
    fn failed_load_does_not_increment_generation() {
        let path = std::env::temp_dir().join(format!("see_nogen_{}.json", std::process::id()));
        let mut b = SimBridge::new(42, 2);
        let before = b.generation;
        assert!(b.load(&path).is_err());
        assert_eq!(b.generation, before);
    }
}
