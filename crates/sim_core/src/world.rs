//! Оркестратор мира: объединяет модули `sim_core` в единый тик-цикл.
//!
//! [`World`] владеет всем состоянием симуляции (время, агенты, корабли,
//! рынки, очереди производства, контракты, мозги NPC) и продвигает его
//! методом [`World::tick`]. Порядок систем внутри тика фиксирован:
//! время → производство → рынок → контракты → ИИ. Все коллекции —
//! `BTreeMap`, поэтому обход идёт в порядке идентификаторов, а единственный
//! источник случайности — `ChaCha8Rng` с заданным зерном.

use std::collections::BTreeMap;
use std::fmt;

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::ai_agents::{AgentBrain, Plan, PlanError, WorldState};
use crate::character::Character;
use crate::contracts::{ContractRegistry, ContractState};
use crate::entities::{Agent, AgentId, Cargo, MarketOrder, Ship, ShipId, SystemId};
use crate::factions::FactionRegistry;
use crate::market::{Market, MarketError, Trade};
use crate::production::{CompletedBatch, ProductionQueue, Recipe, apply_batch};
use crate::time::GameTime;

/// Событие, записанное в журнал мира.
///
/// `Serialize`/`Deserialize` не выводятся: [`PlanError`] из `ai_agents.rs`
/// их не реализует, а менять чужой модуль нельзя.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldEvent {
    /// Начало тика.
    Tick {
        /// Номер тика.
        tick: u64,
    },
    /// Агент завершил партию производства.
    ProductionCompleted {
        /// Агент-владелец очереди.
        agent: AgentId,
        /// Завершённая партия.
        batch: CompletedBatch,
    },
    /// На рынке совершена сделка.
    TradeExecuted {
        /// Система, где находится рынок.
        market: SystemId,
        /// Сделка.
        trade: Trade,
    },
    /// Контракт истёк по дедлайну.
    ContractExpired {
        /// Идентификатор контракта.
        contract_id: u64,
    },
    /// Агент выполнил действие плана.
    ActionExecuted {
        /// Агент.
        agent: AgentId,
        /// Имя действия.
        action_name: String,
        /// Тик выполнения.
        tick: u64,
    },
    /// Планирование не удалось.
    ReplanFailed {
        /// Агент.
        agent: AgentId,
        /// Причина.
        error: PlanError,
    },
    /// План завершён (или оказался пустым: цель уже достигнута).
    PlanCompleted {
        /// Агент.
        agent: AgentId,
        /// Суммарная стоимость плана.
        total_cost: u32,
    },
}

/// Разум NPC-агента: мозг GOAP, его картина мира и текущий план.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMind {
    /// Известное агенту состояние мира.
    pub state: WorldState,
    /// Цели и действия агента.
    pub brain: AgentBrain,
    /// Выполняемый план, если есть.
    pub current_plan: Option<Plan>,
    /// Индекс следующего действия плана.
    pub plan_index: usize,
}

/// Ошибка операции над [`World`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldError {
    /// Агент с таким идентификатором не найден.
    AgentNotFound(AgentId),
    /// У агента уже есть мозг.
    BrainAlreadyAttached(AgentId),
    /// У агента уже есть персонаж.
    CharacterAlreadyAttached(AgentId),
    /// Рынок в системе не создан.
    MarketNotFound(SystemId),
    /// Ошибка рынка.
    MarketError(MarketError),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldError::AgentNotFound(id) => write!(f, "агент {} не найден", id.0),
            WorldError::BrainAlreadyAttached(id) => {
                write!(f, "у агента {} уже есть мозг", id.0)
            }
            WorldError::CharacterAlreadyAttached(id) => {
                write!(f, "у агента {} уже есть персонаж", id.0)
            }
            WorldError::MarketNotFound(id) => write!(f, "рынок в системе {} не найден", id.0),
            WorldError::MarketError(e) => write!(f, "ошибка рынка: {e}"),
        }
    }
}

impl std::error::Error for WorldError {}

impl From<MarketError> for WorldError {
    fn from(e: MarketError) -> Self {
        WorldError::MarketError(e)
    }
}

/// Мир симуляции: всё состояние и тик-цикл.
///
/// Намеренно не `Clone`: мир — единственный экземпляр состояния.
#[derive(Debug)]
pub struct World {
    /// Игровое время.
    pub time: GameTime,
    /// Генератор случайных чисел мира (ChaCha8, задаётся зерном).
    pub rng: ChaCha8Rng,
    /// Агенты.
    pub agents: BTreeMap<AgentId, Agent>,
    /// Корабли.
    pub ships: BTreeMap<ShipId, Ship>,
    /// Персонажи, привязанные к агентам.
    pub characters: BTreeMap<AgentId, Character>,
    /// Разумы NPC-агентов.
    pub minds: BTreeMap<AgentId, AgentMind>,
    /// Рынки по системам.
    pub markets: BTreeMap<SystemId, Market>,
    /// Очереди производства агентов.
    pub production_queues: BTreeMap<AgentId, ProductionQueue>,
    /// Склады агентов: сюда попадает продукция завершённых партий.
    /// В `Agent` груза нет (трюм есть только у `Ship`), а у агента может не
    /// быть кораблей, поэтому склад хранится отдельно.
    pub cargo: BTreeMap<AgentId, Cargo>,
    /// Реестр фракций.
    pub factions: FactionRegistry,
    /// Реестр контрактов.
    pub contracts: ContractRegistry,
    /// Журнал событий.
    pub events: Vec<WorldEvent>,
    /// Следующий свободный идентификатор агента.
    pub next_agent_id: u64,
    /// Следующий свободный идентификатор корабля.
    pub next_ship_id: u64,
}

impl World {
    /// Создаёт пустой мир с заданным зерном генератора.
    pub fn new(seed: u64) -> Self {
        Self {
            time: GameTime::new(),
            rng: ChaCha8Rng::seed_from_u64(seed),
            agents: BTreeMap::new(),
            ships: BTreeMap::new(),
            characters: BTreeMap::new(),
            minds: BTreeMap::new(),
            markets: BTreeMap::new(),
            production_queues: BTreeMap::new(),
            cargo: BTreeMap::new(),
            factions: FactionRegistry::new(),
            contracts: ContractRegistry::new(),
            events: Vec::new(),
            next_agent_id: 0,
            next_ship_id: 0,
        }
    }

    /// Выполняет один тик: время, производство, рынок, контракты, ИИ.
    pub fn tick(&mut self) {
        self.time.advance(1);
        self.events.push(WorldEvent::Tick {
            tick: self.time.tick,
        });
        self.run_production();
        self.run_market();
        self.run_contracts();
        self.run_ai();
    }

    /// Выполняет `ticks` тиков подряд.
    pub fn advance(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    /// Добавляет агента, присваивая ему следующий идентификатор
    /// (поле `id` переданного агента перезаписывается).
    pub fn spawn_agent(&mut self, agent: Agent) -> AgentId {
        let id = AgentId(self.next_agent_id);
        self.next_agent_id += 1;
        self.agents.insert(id, Agent { id, ..agent });
        id
    }

    /// Добавляет корабль, присваивая ему следующий идентификатор
    /// (поле `id` переданного корабля перезаписывается).
    pub fn spawn_ship(&mut self, ship: Ship) -> ShipId {
        let id = ShipId(self.next_ship_id);
        self.next_ship_id += 1;
        self.ships.insert(id, Ship { id, ..ship });
        id
    }

    /// Подключает мозг к агенту.
    pub fn attach_brain(&mut self, id: AgentId, brain: AgentBrain) -> Result<(), WorldError> {
        if !self.agents.contains_key(&id) {
            return Err(WorldError::AgentNotFound(id));
        }
        if self.minds.contains_key(&id) {
            return Err(WorldError::BrainAlreadyAttached(id));
        }
        self.minds.insert(
            id,
            AgentMind {
                state: WorldState::default(),
                brain,
                current_plan: None,
                plan_index: 0,
            },
        );
        Ok(())
    }

    /// Привязывает персонажа к агенту.
    pub fn attach_character(
        &mut self,
        id: AgentId,
        character: Character,
    ) -> Result<(), WorldError> {
        if !self.agents.contains_key(&id) {
            return Err(WorldError::AgentNotFound(id));
        }
        if self.characters.contains_key(&id) {
            return Err(WorldError::CharacterAlreadyAttached(id));
        }
        self.characters.insert(id, character);
        Ok(())
    }

    /// Создаёт рынок в системе. Существующий рынок не заменяется.
    pub fn create_market(&mut self, system: SystemId) {
        self.markets.entry(system).or_default();
    }

    /// Выставляет ордер на рынке системы.
    pub fn submit_order(&mut self, system: SystemId, order: MarketOrder) -> Result<(), WorldError> {
        let market = self
            .markets
            .get_mut(&system)
            .ok_or(WorldError::MarketNotFound(system))?;
        market.place_order(order)?;
        Ok(())
    }

    /// Ставит в очередь агента `qty` партий рецепта; отсчёт времени
    /// производства идёт от текущего тика.
    pub fn enqueue_production(
        &mut self,
        agent: AgentId,
        recipe: Recipe,
        qty: u32,
    ) -> Result<(), WorldError> {
        if !self.agents.contains_key(&agent) {
            return Err(WorldError::AgentNotFound(agent));
        }
        let now = self.time.tick;
        self.production_queues
            .entry(agent)
            .or_default()
            .enqueue(recipe, u64::from(qty), now);
        Ok(())
    }

    /// События тиков, начиная с `tick` включительно.
    ///
    /// Каждый тик начинается с события `Tick`, а события пишутся по порядку,
    /// поэтому всё, что идёт после первого `Tick { tick: t }` с `t >= tick`,
    /// относится к тикам не раньше `tick`. Явный номер тика в каждом событии
    /// не нужен.
    pub fn events_since(&self, tick: u64) -> impl Iterator<Item = &WorldEvent> + '_ {
        let start = self
            .events
            .iter()
            .position(|e| matches!(e, WorldEvent::Tick { tick: t } if *t >= tick))
            .unwrap_or(self.events.len());
        self.events[start..].iter()
    }

    /// Очищает журнал событий.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Число событий в журнале.
    pub fn events_len(&self) -> usize {
        self.events.len()
    }

    /// Завершает готовые партии и складывает продукцию на склад агента.
    fn run_production(&mut self) {
        let now = self.time.tick;
        for (&agent, queue) in self.production_queues.iter_mut() {
            for batch in queue.tick(now) {
                apply_batch(&batch, self.cargo.entry(agent).or_default());
                self.events
                    .push(WorldEvent::ProductionCompleted { agent, batch });
            }
        }
    }

    /// Сводит ордера на всех рынках.
    fn run_market(&mut self) {
        for (&market, m) in self.markets.iter_mut() {
            for trade in m.match_orders() {
                self.events
                    .push(WorldEvent::TradeExecuted { market, trade });
            }
        }
    }

    /// Истекает просроченные контракты и пишет событие по каждому.
    fn run_contracts(&mut self) {
        let now = self.time.tick;
        // `expire_overdue` возвращает только число, поэтому запоминаем
        // кандидатов заранее и проверяем их состояние после вызова.
        let candidates: Vec<u64> = self
            .contracts
            .iter()
            .filter(|c| !c.state().is_terminal() && c.deadline_tick <= now)
            .map(|c| c.id)
            .collect();
        if candidates.is_empty() {
            return;
        }
        self.contracts.expire_overdue(now);
        for id in candidates {
            if self
                .contracts
                .get(id)
                .is_some_and(|c| c.state() == ContractState::Expired)
            {
                self.events
                    .push(WorldEvent::ContractExpired { contract_id: id });
            }
        }
    }

    /// Один шаг GOAP для каждого агента с разумом: не более одного действия
    /// за тик.
    fn run_ai(&mut self) {
        let tick = self.time.tick;
        for (&agent, mind) in self.minds.iter_mut() {
            let needs_plan = match &mind.current_plan {
                None => true,
                Some(plan) => mind.plan_index >= plan.actions.len(),
            };
            if needs_plan {
                mind.current_plan = None;
                mind.plan_index = 0;
                match mind.brain.plan(&mind.state) {
                    Ok(plan) if plan.actions.is_empty() => {
                        // Цель уже достигнута: план не сохраняем.
                        self.events.push(WorldEvent::PlanCompleted {
                            agent,
                            total_cost: 0,
                        });
                    }
                    Ok(plan) => mind.current_plan = Some(plan),
                    Err(error) => self.events.push(WorldEvent::ReplanFailed { agent, error }),
                }
            }

            let Some(plan) = mind.current_plan.as_ref() else {
                continue;
            };
            let total_cost = plan.total_cost;
            let len = plan.actions.len();
            let step = plan
                .actions
                .get(mind.plan_index)
                .filter(|action| action.is_applicable(&mind.state))
                .map(|action| (action.apply(&mind.state), action.name.clone()));

            match step {
                Some((next_state, action_name)) => {
                    mind.state = next_state;
                    mind.plan_index += 1;
                    self.events.push(WorldEvent::ActionExecuted {
                        agent,
                        action_name,
                        tick,
                    });
                    if mind.plan_index >= len {
                        self.events
                            .push(WorldEvent::PlanCompleted { agent, total_cost });
                        mind.current_plan = None;
                        mind.plan_index = 0;
                    }
                }
                None => {
                    // План больше не применим: replan на следующем тике.
                    mind.current_plan = None;
                    mind.plan_index = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_agents::{Action, Condition, Effect, Fact, Goal};
    use crate::character::Brain;
    use crate::character::FactionId;
    use crate::contracts::{Contract, ContractKind};
    use crate::entities::ItemId;

    fn agent() -> Agent {
        Agent {
            id: AgentId(999),
            credits: 100,
            location: SystemId(1),
        }
    }

    fn two_step_brain() -> AgentBrain {
        let a = Fact::new("a");
        let done = Fact::new("done");
        let mut desired = WorldState::default();
        desired.set(done.clone(), 1);
        AgentBrain::new(
            vec![Goal {
                name: "finish".into(),
                desired,
                priority: 1,
            }],
            vec![
                Action {
                    name: "first".into(),
                    preconditions: vec![],
                    effects: vec![Effect::Set(a.clone(), 1)],
                    cost: 1,
                },
                Action {
                    name: "second".into(),
                    preconditions: vec![Condition::Eq(a, 1)],
                    effects: vec![Effect::Set(done, 1)],
                    cost: 1,
                },
            ],
        )
    }

    fn recipe(time_ticks: u64) -> Recipe {
        Recipe {
            id: 1,
            inputs: BTreeMap::new(),
            outputs: BTreeMap::from([(ItemId(7), 3)]),
            time_ticks,
        }
    }

    fn count<F: Fn(&WorldEvent) -> bool>(w: &World, f: F) -> usize {
        w.events.iter().filter(|e| f(e)).count()
    }

    #[test]
    fn new_world_starts_at_zero() {
        let w = World::new(1);
        assert_eq!(w.time.tick, 0);
        assert!(w.agents.is_empty());
        assert!(w.events.is_empty());
        assert_eq!(w.next_agent_id, 0);
        assert_eq!(w.next_ship_id, 0);
    }

    #[test]
    fn same_seed_same_rng() {
        let a = World::new(42);
        let b = World::new(42);
        let c = World::new(43);
        assert_eq!(a.rng, b.rng);
        assert_ne!(a.rng, c.rng);
    }

    #[test]
    fn spawn_agent_assigns_id() {
        let mut w = World::new(1);
        assert_eq!(w.spawn_agent(agent()), AgentId(0));
        assert_eq!(w.spawn_agent(agent()), AgentId(1));
        assert_eq!(w.agents[&AgentId(1)].id, AgentId(1));
    }

    #[test]
    fn spawn_ship_assigns_id() {
        let mut w = World::new(1);
        let ship = Ship {
            id: ShipId(55),
            owner: AgentId(0),
            cargo: Cargo::new(),
        };
        assert_eq!(w.spawn_ship(ship.clone()), ShipId(0));
        assert_eq!(w.spawn_ship(ship), ShipId(1));
        assert_eq!(w.ships[&ShipId(1)].id, ShipId(1));
    }

    #[test]
    fn attach_brain_to_agent() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        assert!(w.attach_brain(id, two_step_brain()).is_ok());
        assert!(w.minds.contains_key(&id));
    }

    #[test]
    fn attach_brain_to_missing_agent_fails() {
        let mut w = World::new(1);
        assert_eq!(
            w.attach_brain(AgentId(5), two_step_brain()),
            Err(WorldError::AgentNotFound(AgentId(5)))
        );
    }

    #[test]
    fn attach_brain_twice_fails() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.attach_brain(id, two_step_brain()).unwrap();
        assert_eq!(
            w.attach_brain(id, two_step_brain()),
            Err(WorldError::BrainAlreadyAttached(id))
        );
    }

    #[test]
    fn attach_character_to_agent() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        let ch = Character::new(Brain::new(100, 10));
        assert!(w.attach_character(id, ch.clone()).is_ok());
        assert!(w.characters.contains_key(&id));
        assert_eq!(
            w.attach_character(id, ch.clone()),
            Err(WorldError::CharacterAlreadyAttached(id))
        );
        assert_eq!(
            w.attach_character(AgentId(9), ch),
            Err(WorldError::AgentNotFound(AgentId(9)))
        );
    }

    #[test]
    fn create_market_and_submit_order() {
        let mut w = World::new(1);
        w.create_market(SystemId(1));
        let order = MarketOrder {
            order_id: 1,
            item: ItemId(1),
            price: 10,
            quantity: 5,
            is_buy: false,
        };
        assert!(w.submit_order(SystemId(1), order).is_ok());
        assert_eq!(w.markets[&SystemId(1)].best_ask(ItemId(1)), Some(10));
    }

    #[test]
    fn submit_order_to_missing_market_fails() {
        let mut w = World::new(1);
        let order = MarketOrder {
            order_id: 1,
            item: ItemId(1),
            price: 10,
            quantity: 5,
            is_buy: true,
        };
        assert_eq!(
            w.submit_order(SystemId(3), order),
            Err(WorldError::MarketNotFound(SystemId(3)))
        );
    }

    #[test]
    fn submit_order_with_zero_quantity_is_market_error() {
        let mut w = World::new(1);
        w.create_market(SystemId(1));
        let order = MarketOrder {
            order_id: 1,
            item: ItemId(1),
            price: 10,
            quantity: 0,
            is_buy: true,
        };
        assert_eq!(
            w.submit_order(SystemId(1), order),
            Err(WorldError::MarketError(MarketError::ZeroQuantity))
        );
    }

    #[test]
    fn enqueue_production_for_missing_agent_fails() {
        let mut w = World::new(1);
        assert_eq!(
            w.enqueue_production(AgentId(0), recipe(1), 1),
            Err(WorldError::AgentNotFound(AgentId(0)))
        );
    }

    #[test]
    fn enqueue_production_and_tick_completes() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.enqueue_production(id, recipe(3), 2).unwrap();
        w.advance(2);
        assert_eq!(w.cargo.get(&id).map_or(0, |c| c.get(ItemId(7))), 0);
        w.advance(1);
        assert_eq!(w.cargo[&id].get(ItemId(7)), 6);
        assert_eq!(
            count(&w, |e| matches!(e, WorldEvent::ProductionCompleted { .. })),
            1
        );
    }

    #[test]
    fn matching_orders_emit_trade_event() {
        let mut w = World::new(1);
        w.create_market(SystemId(1));
        for (order_id, is_buy) in [(1, false), (2, true)] {
            w.submit_order(
                SystemId(1),
                MarketOrder {
                    order_id,
                    item: ItemId(1),
                    price: 10,
                    quantity: 2,
                    is_buy,
                },
            )
            .unwrap();
        }
        w.tick();
        assert_eq!(
            count(&w, |e| matches!(e, WorldEvent::TradeExecuted { .. })),
            1
        );
    }

    #[test]
    fn overdue_contract_expires_once() {
        let mut w = World::new(1);
        w.contracts
            .register(Contract::new(
                4,
                FactionId(1),
                ContractKind::Production {
                    required: BTreeMap::new(),
                },
                10,
                1,
                2,
            ))
            .unwrap();
        w.advance(5);
        assert_eq!(
            count(&w, |e| matches!(
                e,
                WorldEvent::ContractExpired { contract_id: 4 }
            )),
            1
        );
    }

    #[test]
    fn tick_advances_time_by_one() {
        let mut w = World::new(1);
        w.tick();
        assert_eq!(w.time.tick, 1);
    }

    #[test]
    fn advance_advances_by_n() {
        let mut w = World::new(1);
        w.advance(5);
        assert_eq!(w.time.tick, 5);
    }

    #[test]
    fn agent_with_brain_plans_on_tick() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.attach_brain(id, two_step_brain()).unwrap();
        w.tick();
        assert!(
            count(&w, |e| matches!(
                e,
                WorldEvent::PlanCompleted { .. } | WorldEvent::ActionExecuted { .. }
            )) > 0
        );
    }

    #[test]
    fn agent_without_brain_does_not_plan() {
        let mut w = World::new(1);
        w.spawn_agent(agent());
        w.advance(3);
        assert_eq!(
            count(&w, |e| matches!(
                e,
                WorldEvent::ActionExecuted { .. } | WorldEvent::ReplanFailed { .. }
            )),
            0
        );
    }

    #[test]
    fn agent_executes_plan_over_ticks() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.attach_brain(id, two_step_brain()).unwrap();
        w.tick();
        assert_eq!(
            count(&w, |e| matches!(e, WorldEvent::ActionExecuted { .. })),
            1
        );
        assert_eq!(
            count(&w, |e| matches!(e, WorldEvent::PlanCompleted { .. })),
            0
        );
        w.tick();
        assert_eq!(
            count(&w, |e| matches!(e, WorldEvent::ActionExecuted { .. })),
            2
        );
        assert_eq!(
            count(&w, |e| matches!(
                e,
                WorldEvent::PlanCompleted { total_cost: 2, .. }
            )),
            1
        );
        assert!(w.minds[&id].current_plan.is_none());
    }

    #[test]
    fn already_reached_goal_gives_empty_plan_without_storing() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.attach_brain(id, two_step_brain()).unwrap();
        w.advance(2);
        w.clear_events();
        w.tick();
        assert_eq!(
            w.events[1],
            WorldEvent::PlanCompleted {
                agent: id,
                total_cost: 0
            }
        );
        assert!(w.minds[&id].current_plan.is_none());
    }

    #[test]
    fn brain_without_goals_reports_replan_failed() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.attach_brain(id, AgentBrain::new(vec![], vec![])).unwrap();
        w.tick();
        assert!(w.events.contains(&WorldEvent::ReplanFailed {
            agent: id,
            error: PlanError::NoPlanFound
        }));
    }

    #[test]
    fn events_record_tick() {
        let mut w = World::new(1);
        w.tick();
        assert_eq!(w.events[0], WorldEvent::Tick { tick: 1 });
        assert_eq!(w.events_len(), 1);
    }

    #[test]
    fn events_since_filters() {
        let mut w = World::new(1);
        let id = w.spawn_agent(agent());
        w.attach_brain(id, two_step_brain()).unwrap();
        w.advance(4);
        let ticks: Vec<u64> = w
            .events_since(2)
            .filter_map(|e| match e {
                WorldEvent::Tick { tick } => Some(*tick),
                _ => None,
            })
            .collect();
        assert_eq!(ticks, vec![2, 3, 4]);
        assert!(
            w.events_since(2)
                .any(|e| matches!(e, WorldEvent::ActionExecuted { tick: 2, .. }))
        );
        assert!(
            !w.events_since(2)
                .any(|e| matches!(e, WorldEvent::ActionExecuted { tick: 1, .. }))
        );
        assert_eq!(w.events_since(99).count(), 0);
    }

    #[test]
    fn clear_events_empties_log() {
        let mut w = World::new(1);
        w.advance(2);
        w.clear_events();
        assert_eq!(w.events_len(), 0);
    }

    fn build(seed: u64) -> World {
        let mut w = World::new(seed);
        let a = w.spawn_agent(agent());
        let b = w.spawn_agent(agent());
        w.attach_brain(a, two_step_brain()).unwrap();
        w.attach_brain(b, two_step_brain()).unwrap();
        w.enqueue_production(b, recipe(4), 1).unwrap();
        w.create_market(SystemId(1));
        w
    }

    #[test]
    fn identical_seed_identical_tick_log() {
        let mut a = build(42);
        let mut b = build(42);
        a.advance(10);
        b.advance(10);
        assert_eq!(a.events, b.events);
        assert!(!a.events.is_empty());
    }
}
