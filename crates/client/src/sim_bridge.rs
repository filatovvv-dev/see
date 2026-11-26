//! Мост между клиентом и `sim_core::World`.
//!
//! Чистая логика без Bevy: продвижение тиков по реальному времени, пауза и
//! скорость. Тестируется без `App`.

use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::TAU;

use narrative::NarrativeEngine;
use narrative::llm_adapter::MockLlmClient;
use sim_core::entities::{Agent, AgentId, Cargo, Ship, ShipId, SystemId};
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

/// Число демо-систем, между которыми распределяются демо-агенты.
const DEMO_SYSTEMS: u64 = 3;

/// Число планет-декораций вокруг системы.
const PLANETS_PER_SYSTEM: usize = 3;

/// Амплитуда смещения корабля относительно центра системы, px.
const SHIP_OFFSET: f32 = 20.0;

/// Палитра систем (RGBA), выбор по `SystemId % 6`.
const SYSTEM_PALETTE: [[f32; 4]; 6] = [
    [0.25, 0.35, 0.55, 1.0],
    [0.45, 0.30, 0.55, 1.0],
    [0.25, 0.50, 0.45, 1.0],
    [0.55, 0.40, 0.25, 1.0],
    [0.50, 0.25, 0.30, 1.0],
    [0.35, 0.45, 0.25, 1.0],
];

/// Палитра планет (RGBA), выбор по индексу планеты.
const PLANET_PALETTE: [[f32; 4]; 3] = [
    [0.70, 0.60, 0.45, 1.0],
    [0.45, 0.65, 0.75, 1.0],
    [0.75, 0.50, 0.50, 1.0],
];

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

/// Позиция системы на 2D-сцене (клиентская, не часть `sim_core`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemPosition {
    /// Координата X.
    pub x: f32,
    /// Координата Y.
    pub y: f32,
}

/// Цвет системы (RGBA) по `SystemId`: палитра из 6 цветов.
pub fn system_color(id: SystemId) -> [f32; 4] {
    SYSTEM_PALETTE[(id.0 % 6) as usize]
}

/// Цвет планеты (RGBA) по индексу: 3 варианта.
pub fn planet_color(index: usize) -> [f32; 4] {
    PLANET_PALETTE[index % PLANET_PALETTE.len()]
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
    /// Последняя сгенерированная сцена (для UI). В сохранение не входит.
    pub last_scene: Option<String>,
}

impl SimBridge {
    /// Создаёт мост с миром на зерне `seed` и `agent_count` демо-агентами.
    pub fn new(seed: u64, agent_count: usize) -> Self {
        let mut world = World::new(seed);
        for i in 0..agent_count {
            // Демо-агенты распределены по нескольким системам, у каждого
            // по одному кораблю: так на сцене видны системы и корабли.
            let owner = world.spawn_agent(Agent {
                id: AgentId(0),
                credits: 1000,
                location: SystemId(i as u64 % DEMO_SYSTEMS),
            });
            world.spawn_ship(Ship {
                id: ShipId(0),
                owner,
                cargo: Cargo::new(),
            });
        }
        Self {
            world,
            paused: false,
            speed: 1.0,
            accumulator: 0.0,
            generation: 1,
            last_scene: None,
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

    /// Известные системы: рынки и локации агентов, без повторов, по
    /// возрастанию `SystemId`.
    pub fn known_systems(&self) -> Vec<SystemId> {
        let mut systems: BTreeSet<SystemId> = self.world.markets.keys().copied().collect();
        systems.extend(self.world.agents.values().map(|a| a.location));
        systems.into_iter().collect()
    }

    /// Раскладка систем по кругу радиуса `radius` в порядке `SystemId`.
    ///
    /// Система с индексом `i` из `n` получает угол `2π·i/n`.
    pub fn system_layout(&self, radius: f32) -> BTreeMap<SystemId, SystemPosition> {
        let systems = self.known_systems();
        let n = systems.len();
        systems
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let angle = TAU * i as f32 / n as f32;
                (
                    id,
                    SystemPosition {
                        x: radius * angle.cos(),
                        y: radius * angle.sin(),
                    },
                )
            })
            .collect()
    }

    /// Планеты-декорации системы: 3 точки на орбите радиуса `orbit_radius`,
    /// координаты относительно центра системы. Фаза зависит от `SystemId`.
    pub fn planet_layout(&self, system: SystemId, orbit_radius: f32) -> Vec<SystemPosition> {
        (0..PLANETS_PER_SYSTEM)
            .map(|j| {
                let angle = TAU * j as f32 / PLANETS_PER_SYSTEM as f32 + system.0 as f32 * 0.1;
                SystemPosition {
                    x: orbit_radius * angle.cos(),
                    y: orbit_radius * angle.sin(),
                }
            })
            .collect()
    }

    /// Позиции кораблей: позиция системы владельца плюс детерминированное
    /// смещение `sin(ShipId·0.37)·20` по обеим осям.
    ///
    /// Корабли, чей владелец неизвестен, пропускаются.
    pub fn ship_positions(&self, system_radius: f32) -> BTreeMap<ShipId, SystemPosition> {
        let layout = self.system_layout(system_radius);
        self.world
            .ships
            .values()
            .filter_map(|ship| {
                let agent = self.world.agents.get(&ship.owner)?;
                let base = layout.get(&agent.location)?;
                let offset = (ship.id.0 as f32 * 0.37).sin() * SHIP_OFFSET;
                Some((
                    ship.id,
                    SystemPosition {
                        x: base.x + offset,
                        y: base.y + offset,
                    },
                ))
            })
            .collect()
    }

    /// Генерирует сцену через `narrative` (заглушка `MockLlmClient`, без
    /// сети) и сохраняет её в `last_scene`.
    ///
    /// Движок создаётся при каждом вызове: заглушка дешёвая и без состояния.
    /// При ошибке (например, пустой контекст) `last_scene` не меняется.
    pub fn generate_scene(&mut self, context: &str) -> Result<(), String> {
        let engine = NarrativeEngine::new(Box::new(MockLlmClient::default_mock()));
        let scene = engine.generate_scene(context).map_err(|e| e.to_string())?;
        self.last_scene = Some(scene);
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

    #[test]
    fn generate_scene_sets_last_scene() {
        let mut b = SimBridge::new(42, 1);
        assert!(b.last_scene.is_none());
        b.generate_scene("test").unwrap();
        let scene = b.last_scene.as_deref().unwrap_or_default();
        assert!(scene.contains("[mock]"));
        assert!(scene.contains("test"));
    }

    #[test]
    fn generate_scene_empty_context_fails() {
        let mut b = SimBridge::new(42, 1);
        b.generate_scene("first").unwrap();
        let before = b.last_scene.clone();
        assert!(b.generate_scene("").is_err());
        assert!(b.generate_scene("   ").is_err());
        assert_eq!(b.last_scene, before);
    }

    #[test]
    fn known_systems_collects_from_markets_and_agents() {
        let mut b = SimBridge::new(42, 0);
        assert!(b.known_systems().is_empty());
        assert!(b.system_layout(100.0).is_empty());
        b.world.create_market(SystemId(5));
        b.world.spawn_agent(Agent {
            id: AgentId(0),
            credits: 1,
            location: SystemId(3),
        });
        b.world.spawn_agent(Agent {
            id: AgentId(0),
            credits: 1,
            location: SystemId(5),
        });
        assert_eq!(b.known_systems(), vec![SystemId(3), SystemId(5)]);
    }

    #[test]
    fn system_layout_is_deterministic() {
        let a = SimBridge::new(42, 5);
        let b = SimBridge::new(42, 5);
        assert_eq!(a.system_layout(300.0), a.system_layout(300.0));
        assert_eq!(a.system_layout(300.0), b.system_layout(300.0));
        assert_eq!(a.system_layout(300.0).len(), 3);
    }

    #[test]
    fn planet_layout_returns_three_planets() {
        let b = SimBridge::new(42, 1);
        let planets = b.planet_layout(SystemId(2), 50.0);
        assert_eq!(planets.len(), 3);
        for p in planets {
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!((r - 50.0).abs() < 0.01, "r = {r}");
        }
        assert_ne!(
            b.planet_layout(SystemId(0), 50.0),
            b.planet_layout(SystemId(1), 50.0)
        );
    }

    #[test]
    fn ship_positions_place_ship_near_system() {
        let b = SimBridge::new(42, 5);
        let layout = b.system_layout(350.0);
        let ships = b.ship_positions(350.0);
        assert_eq!(ships.len(), 5);
        for (ship_id, pos) in &ships {
            let owner = b.world.ships[ship_id].owner;
            let center = layout[&b.world.agents[&owner].location];
            let d = ((pos.x - center.x).powi(2) + (pos.y - center.y).powi(2)).sqrt();
            assert!(d <= 30.0, "d = {d}");
        }
    }

    #[test]
    fn ship_with_unknown_owner_is_skipped() {
        let mut b = SimBridge::new(42, 0);
        b.world.spawn_ship(Ship {
            id: ShipId(0),
            owner: AgentId(99),
            cargo: Cargo::new(),
        });
        assert!(b.ship_positions(100.0).is_empty());
    }

    #[test]
    fn system_and_planet_colors_cycle() {
        assert_eq!(system_color(SystemId(0)), system_color(SystemId(6)));
        assert_ne!(system_color(SystemId(0)), system_color(SystemId(1)));
        assert_eq!(planet_color(0), planet_color(3));
    }
}
