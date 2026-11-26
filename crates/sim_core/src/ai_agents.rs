//! GOAP-планировщик (Goal-Oriented Action Planning) для NPC-агентов.
//!
//! Мир описывается набором фактов с целочисленными значениями
//! ([`WorldState`]). Действия ([`Action`]) имеют условия применимости и
//! эффекты, цели ([`Goal`]) задают желаемые значения фактов. Планировщик
//! ([`Planner`]) ищет самую дешёвую последовательность действий алгоритмом A*
//! (только поиск вперёд).
//!
//! Модуль детерминирован: используются только `BTreeMap`, `Vec` и
//! `BinaryHeap` с полным порядком ключей. Ничего случайного и никакого
//! системного времени (см. AGENTS.md, раздел «Главное правило детерминизма»).

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::entities::{ItemId, SystemId};

/// Максимальная глубина поиска по умолчанию (число действий в плане).
const DEFAULT_MAX_DEPTH: usize = 16;

/// Максимальное число раскрытых узлов по умолчанию.
const DEFAULT_MAX_EXPANSIONS: usize = 10_000;

/// Имя факта мира (например, `"credits"`, `"at_system_3"`, `"has_item_7"`).
///
/// Обёртка над `String`; в коллекциях используется детерминированный
/// порядок (`BTreeMap`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fact(pub String);

impl Fact {
    /// Создаёт факт с заданным именем.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Состояние мира: значения фактов.
///
/// Булевы факты кодируются как `0` (ложь) и `1` (истина). Отсутствующий факт
/// равен `0`. Чтобы состояния с одинаковым смыслом были равны, методы
/// [`WorldState::set`] и [`WorldState::add`] не хранят нулевые значения:
/// запись нуля удаляет факт.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct WorldState(pub BTreeMap<Fact, i64>);

impl WorldState {
    /// Значение факта; `0`, если факт отсутствует.
    pub fn get(&self, fact: &Fact) -> i64 {
        self.0.get(fact).copied().unwrap_or(0)
    }

    /// Устанавливает значение факта. Нулевое значение удаляет факт.
    pub fn set(&mut self, fact: Fact, value: i64) {
        if value == 0 {
            self.0.remove(&fact);
        } else {
            self.0.insert(fact, value);
        }
    }

    /// Прибавляет `delta` к значению факта с насыщением (без переполнения).
    pub fn add(&mut self, fact: &Fact, delta: i64) {
        let value = self.get(fact).saturating_add(delta);
        self.set(fact.clone(), value);
    }

    /// Проверяет, удовлетворяет ли состояние желаемому `desired`.
    ///
    /// Все факты из `desired` должны совпадать по значению. Лишние факты
    /// состояния игнорируются: `desired` — желаемое подмножество.
    pub fn matches(&self, desired: &WorldState) -> bool {
        desired
            .0
            .iter()
            .all(|(fact, value)| self.get(fact) == *value)
    }

    /// Число фактов из `desired`, значения которых в состоянии не совпадают.
    fn unsatisfied_count(&self, desired: &WorldState) -> u64 {
        let count = desired
            .0
            .iter()
            .filter(|(fact, value)| self.get(fact) != **value)
            .count();
        u64::try_from(count).unwrap_or(u64::MAX)
    }
}

/// Условие применимости действия.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    /// Факт равен значению.
    Eq(Fact, i64),
    /// Факт не меньше значения.
    Gte(Fact, i64),
    /// Факт не больше значения.
    Lte(Fact, i64),
    /// Факт больше значения.
    Gt(Fact, i64),
    /// Факт меньше значения.
    Lt(Fact, i64),
}

impl Condition {
    /// Выполняется ли условие в состоянии `state`.
    pub fn is_satisfied(&self, state: &WorldState) -> bool {
        match self {
            Condition::Eq(fact, value) => state.get(fact) == *value,
            Condition::Gte(fact, value) => state.get(fact) >= *value,
            Condition::Lte(fact, value) => state.get(fact) <= *value,
            Condition::Gt(fact, value) => state.get(fact) > *value,
            Condition::Lt(fact, value) => state.get(fact) < *value,
        }
    }
}

/// Эффект действия.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Установить значение.
    Set(Fact, i64),
    /// Прибавить (с насыщением).
    Add(Fact, i64),
    /// Вычесть (с насыщением).
    Sub(Fact, i64),
}

impl Effect {
    /// Применяет эффект к состоянию `state`.
    pub fn apply(&self, state: &mut WorldState) {
        match self {
            Effect::Set(fact, value) => state.set(fact.clone(), *value),
            Effect::Add(fact, delta) => state.add(fact, *delta),
            Effect::Sub(fact, delta) => {
                let value = state.get(fact).saturating_sub(*delta);
                state.set(fact.clone(), value);
            }
        }
    }
}

/// Действие агента.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// Уникальное имя (используется для детерминированного выбора среди
    /// равных по стоимости планов).
    pub name: String,
    /// Условия применимости: должны выполняться все.
    pub preconditions: Vec<Condition>,
    /// Эффекты, которые применяются по порядку.
    pub effects: Vec<Effect>,
    /// Стоимость. Больше — дороже; `0` — бесплатно.
    pub cost: u32,
}

impl Action {
    /// Применимо ли действие в состоянии `state` (выполнены все условия).
    pub fn is_applicable(&self, state: &WorldState) -> bool {
        self.preconditions
            .iter()
            .all(|condition| condition.is_satisfied(state))
    }

    /// Возвращает новое состояние после применения эффектов.
    ///
    /// Входное состояние не меняется. Применимость не проверяется: её
    /// проверяет вызывающий код через [`Action::is_applicable`].
    pub fn apply(&self, state: &WorldState) -> WorldState {
        let mut next = state.clone();
        for effect in &self.effects {
            effect.apply(&mut next);
        }
        next
    }
}

/// Цель агента.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    /// Уникальное имя (для логов и выбора среди равных по приоритету целей).
    pub name: String,
    /// Желаемое состояние: все указанные факты должны принять эти значения.
    pub desired: WorldState,
    /// Приоритет. Больше — важнее. Цели выбираются по убыванию приоритета,
    /// затем по имени.
    pub priority: u32,
}

/// План — последовательность действий.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Действия в порядке выполнения.
    pub actions: Vec<Action>,
    /// Суммарная стоимость действий (с насыщением на `u32::MAX`).
    pub total_cost: u32,
}

impl Plan {
    /// Применяет весь план к состоянию `state` и возвращает результат.
    ///
    /// Условия применимости не проверяются; входное состояние не меняется.
    pub fn apply(&self, state: &WorldState) -> WorldState {
        self.actions
            .iter()
            .fold(state.clone(), |current, action| action.apply(&current))
    }
}

/// Ошибка планирования.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanError {
    /// План, достигающий цели, не найден: пространство поиска исчерпано.
    NoPlanFound,
    /// Превышен лимит поиска (глубина или число раскрытых узлов): план мог
    /// существовать, но поиск был остановлен.
    SearchExhausted,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::NoPlanFound => write!(f, "no plan achieves the goal"),
            PlanError::SearchExhausted => write!(f, "plan search limit exceeded"),
        }
    }
}

impl std::error::Error for PlanError {}

/// Ключ очереди A*: `(f, g, ранг последнего действия, порядковый номер, узел)`.
///
/// Кортеж сравнивается слева направо и задаёт полный порядок, поэтому выбор
/// узла не зависит от внутреннего устройства кучи.
type QueueKey = (u64, u32, usize, usize, usize);

/// Узел поиска: состояние, стоимость пути и ссылка на родителя.
struct Node {
    /// Состояние мира в узле.
    state: WorldState,
    /// Накопленная стоимость пути до узла.
    g: u32,
    /// Глубина (число действий от начального состояния).
    depth: usize,
    /// Индекс родительского узла.
    parent: Option<usize>,
    /// Индекс действия (в исходном срезе), которое привело в узел.
    action: Option<usize>,
}

/// GOAP-планировщик на A* (поиск вперёд).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Planner {
    /// Максимальная глубина поиска (число действий в плане).
    max_depth: usize,
    /// Максимальное число раскрытых узлов.
    max_expansions: usize,
}

impl Planner {
    /// Создаёт планировщик с заданными лимитами глубины и числа раскрытий.
    pub fn new(max_depth: usize, max_expansions: usize) -> Self {
        Self {
            max_depth,
            max_expansions,
        }
    }

    /// Планировщик с лимитами по умолчанию: глубина 16, 10 000 раскрытий.
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_MAX_DEPTH, DEFAULT_MAX_EXPANSIONS)
    }

    /// Строит план достижения цели `goal` из состояния `current`.
    ///
    /// Приоритет узла — `f = g + h`, где `g` — накопленная стоимость, а `h` —
    /// число неудовлетворённых фактов цели. Эта эвристика не гарантирует
    /// оптимальность, если одно действие закрывает несколько фактов сразу.
    ///
    /// Порядок выбора детерминирован: узлы сравниваются по `(f, g)`, затем по
    /// имени последнего действия (при равных именах — по индексу в `actions`),
    /// затем по порядку добавления в очередь. Соседи раскрываются в порядке
    /// имён действий, поэтому при равной стоимости побеждает действие с
    /// лексикографически меньшим именем независимо от порядка в `actions`.
    ///
    /// Если цель уже достигнута, возвращается пустой план нулевой стоимости.
    ///
    /// # Ошибки
    ///
    /// - [`PlanError::SearchExhausted`] — исчерпан лимит раскрытий либо часть
    ///   узлов отброшена по лимиту глубины, а плана в пределах лимитов нет.
    /// - [`PlanError::NoPlanFound`] — пространство поиска перебрано полностью,
    ///   плана нет.
    ///
    /// Лимит глубины может отбросить путь, который короче по числу действий,
    /// но дороже по стоимости, если то же состояние уже достигнуто дешевле.
    pub fn plan(
        &self,
        current: &WorldState,
        goal: &Goal,
        actions: &[Action],
    ) -> Result<Plan, PlanError> {
        // Порядок раскрытия соседей и ранг действия: по имени, затем по индексу.
        let mut order: Vec<usize> = (0..actions.len()).collect();
        order.sort_by(|&a, &b| actions[a].name.cmp(&actions[b].name).then(a.cmp(&b)));
        let mut rank = vec![0usize; actions.len()];
        for (position, &index) in order.iter().enumerate() {
            rank[index] = position + 1;
        }

        let mut nodes: Vec<Node> = vec![Node {
            state: current.clone(),
            g: 0,
            depth: 0,
            parent: None,
            action: None,
        }];
        let mut best_g: BTreeMap<WorldState, u32> = BTreeMap::new();
        best_g.insert(current.clone(), 0);

        let mut open: BinaryHeap<Reverse<QueueKey>> = BinaryHeap::new();
        let mut seq: usize = 0;
        open.push(Reverse((
            current.unsatisfied_count(&goal.desired),
            0,
            0,
            seq,
            0,
        )));

        let mut expansions: usize = 0;
        let mut depth_limited = false;

        while let Some(Reverse((_, _, _, _, index))) = open.pop() {
            let (g, depth) = (nodes[index].g, nodes[index].depth);

            // Устаревшая запись: к этому состоянию позже нашли путь дешевле.
            if best_g
                .get(&nodes[index].state)
                .is_some_and(|&best| g > best)
            {
                continue;
            }

            if nodes[index].state.matches(&goal.desired) {
                return Ok(Self::build_plan(&nodes, index, actions, g));
            }

            if depth >= self.max_depth {
                depth_limited = true;
                continue;
            }
            if expansions >= self.max_expansions {
                return Err(PlanError::SearchExhausted);
            }
            expansions += 1;

            for &action_index in &order {
                let action = &actions[action_index];
                if !action.is_applicable(&nodes[index].state) {
                    continue;
                }
                let next_state = action.apply(&nodes[index].state);
                let next_g = g.saturating_add(action.cost);
                if best_g.get(&next_state).is_some_and(|&best| best <= next_g) {
                    continue;
                }
                best_g.insert(next_state.clone(), next_g);

                let f =
                    u64::from(next_g).saturating_add(next_state.unsatisfied_count(&goal.desired));
                nodes.push(Node {
                    state: next_state,
                    g: next_g,
                    depth: depth + 1,
                    parent: Some(index),
                    action: Some(action_index),
                });
                seq += 1;
                open.push(Reverse((
                    f,
                    next_g,
                    rank[action_index],
                    seq,
                    nodes.len() - 1,
                )));
            }
        }

        if depth_limited {
            Err(PlanError::SearchExhausted)
        } else {
            Err(PlanError::NoPlanFound)
        }
    }

    /// Собирает план, идя от целевого узла к корню по ссылкам на родителей.
    fn build_plan(nodes: &[Node], goal_index: usize, actions: &[Action], total_cost: u32) -> Plan {
        let mut chain: Vec<usize> = Vec::new();
        let mut cursor = goal_index;
        while let Some(action_index) = nodes[cursor].action {
            chain.push(action_index);
            match nodes[cursor].parent {
                Some(parent) => cursor = parent,
                None => break,
            }
        }
        chain.reverse();
        Plan {
            actions: chain.into_iter().map(|i| actions[i].clone()).collect(),
            total_cost,
        }
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Мозг NPC-агента: набор целей и действий и лимиты планировщика.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBrain {
    /// Цели агента.
    pub goals: Vec<Goal>,
    /// Действия, доступные агенту.
    pub actions: Vec<Action>,
    /// Максимальная глубина поиска (число действий в плане).
    pub max_depth: usize,
    /// Максимальное число раскрытых узлов на одну цель.
    pub max_expansions: usize,
}

impl AgentBrain {
    /// Создаёт мозг с лимитами планировщика по умолчанию.
    pub fn new(goals: Vec<Goal>, actions: Vec<Action>) -> Self {
        Self {
            goals,
            actions,
            max_depth: DEFAULT_MAX_DEPTH,
            max_expansions: DEFAULT_MAX_EXPANSIONS,
        }
    }

    /// Строит план для самой приоритетной цели, для которой план существует.
    ///
    /// Цели перебираются по убыванию приоритета, затем по возрастанию имени.
    /// Уже достигнутая цель имеет пустой план и выбирается так же, как любая
    /// другая: отбирать недостигнутые цели должен вызывающий код.
    ///
    /// Если план не нашёлся ни для одной цели, возвращается
    /// [`PlanError::SearchExhausted`], когда хотя бы на одной цели поиск упёрся
    /// в лимит, иначе [`PlanError::NoPlanFound`] (в том числе при пустом
    /// списке целей).
    pub fn plan(&self, current: &WorldState) -> Result<Plan, PlanError> {
        let planner = Planner::new(self.max_depth, self.max_expansions);

        let mut order: Vec<usize> = (0..self.goals.len()).collect();
        order.sort_by(|&a, &b| {
            self.goals[b]
                .priority
                .cmp(&self.goals[a].priority)
                .then_with(|| self.goals[a].name.cmp(&self.goals[b].name))
                .then(a.cmp(&b))
        });

        let mut exhausted = false;
        for index in order {
            match planner.plan(current, &self.goals[index], &self.actions) {
                Ok(plan) => return Ok(plan),
                Err(PlanError::SearchExhausted) => exhausted = true,
                Err(PlanError::NoPlanFound) => {}
            }
        }
        if exhausted {
            Err(PlanError::SearchExhausted)
        } else {
            Err(PlanError::NoPlanFound)
        }
    }
}

/// Факт «агент находится в системе `system`»: `at_system_<id>` (1 — да).
pub fn fact_at_system(system: SystemId) -> Fact {
    Fact(format!("at_system_{}", system.0))
}

/// Факт «число кредитов у агента»: `credits`.
pub fn fact_credits() -> Fact {
    Fact("credits".to_string())
}

/// Факт «количество предмета `item` у агента»: `has_item_<id>`.
pub fn fact_has_item(item: ItemId) -> Fact {
    Fact(format!("has_item_{}", item.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(name: &str) -> Fact {
        Fact::new(name)
    }

    fn state(pairs: &[(&str, i64)]) -> WorldState {
        let mut ws = WorldState::default();
        for (name, value) in pairs {
            ws.set(fact(name), *value);
        }
        ws
    }

    fn action(name: &str, pre: Vec<Condition>, eff: Vec<Effect>, cost: u32) -> Action {
        Action {
            name: name.to_string(),
            preconditions: pre,
            effects: eff,
            cost,
        }
    }

    fn goal(name: &str, desired: &[(&str, i64)], priority: u32) -> Goal {
        Goal {
            name: name.to_string(),
            desired: state(desired),
            priority,
        }
    }

    fn names(plan: &Plan) -> Vec<&str> {
        plan.actions.iter().map(|a| a.name.as_str()).collect()
    }

    // Базовые

    #[test]
    fn world_state_get_missing_is_zero() {
        assert_eq!(WorldState::default().get(&fact("credits")), 0);
    }

    #[test]
    fn world_state_set_and_get() {
        let mut ws = WorldState::default();
        ws.set(fact("credits"), 500);
        assert_eq!(ws.get(&fact("credits")), 500);
        ws.set(fact("credits"), -7);
        assert_eq!(ws.get(&fact("credits")), -7);
    }

    #[test]
    fn world_state_add_saturating() {
        let mut ws = state(&[("x", i64::MAX - 1)]);
        ws.add(&fact("x"), 10);
        assert_eq!(ws.get(&fact("x")), i64::MAX);
        let mut low = state(&[("y", i64::MIN + 1)]);
        low.add(&fact("y"), -10);
        assert_eq!(low.get(&fact("y")), i64::MIN);
    }

    #[test]
    fn world_state_matches_exact() {
        let ws = state(&[("a", 1), ("b", 2)]);
        assert!(ws.matches(&state(&[("a", 1), ("b", 2)])));
        assert!(!ws.matches(&state(&[("a", 1), ("b", 3)])));
    }

    #[test]
    fn world_state_matches_ignores_extra_facts() {
        let ws = state(&[("a", 1), ("b", 2), ("c", 3)]);
        assert!(ws.matches(&state(&[("a", 1)])));
        assert!(ws.matches(&WorldState::default()));
    }

    #[test]
    fn world_state_zero_is_same_as_missing() {
        let mut ws = state(&[("a", 5)]);
        ws.set(fact("a"), 0);
        assert_eq!(ws, WorldState::default());
        assert!(WorldState::default().matches(&state(&[("a", 0)])));
    }

    // Условия

    #[test]
    fn condition_eq_gte_lte_gt_lt() {
        let ws = state(&[("n", 5)]);
        let n = fact("n");
        assert!(Condition::Eq(n.clone(), 5).is_satisfied(&ws));
        assert!(!Condition::Eq(n.clone(), 4).is_satisfied(&ws));
        assert!(Condition::Gte(n.clone(), 5).is_satisfied(&ws));
        assert!(!Condition::Gte(n.clone(), 6).is_satisfied(&ws));
        assert!(Condition::Lte(n.clone(), 5).is_satisfied(&ws));
        assert!(!Condition::Lte(n.clone(), 4).is_satisfied(&ws));
        assert!(Condition::Gt(n.clone(), 4).is_satisfied(&ws));
        assert!(!Condition::Gt(n.clone(), 5).is_satisfied(&ws));
        assert!(Condition::Lt(n.clone(), 6).is_satisfied(&ws));
        assert!(!Condition::Lt(n, 5).is_satisfied(&ws));
        // Отсутствующий факт равен нулю.
        assert!(Condition::Eq(fact("missing"), 0).is_satisfied(&ws));
    }

    #[test]
    fn action_applicable_when_all_preconditions_met() {
        let a = action(
            "buy",
            vec![
                Condition::Gte(fact("credits"), 100),
                Condition::Eq(fact("docked"), 1),
            ],
            vec![],
            1,
        );
        assert!(a.is_applicable(&state(&[("credits", 100), ("docked", 1)])));
    }

    #[test]
    fn action_not_applicable_when_any_fails() {
        let a = action(
            "buy",
            vec![
                Condition::Gte(fact("credits"), 100),
                Condition::Eq(fact("docked"), 1),
            ],
            vec![],
            1,
        );
        assert!(!a.is_applicable(&state(&[("credits", 100)])));
        assert!(!a.is_applicable(&state(&[("credits", 99), ("docked", 1)])));
    }

    // Эффекты

    #[test]
    fn effect_set_add_sub_apply() {
        let mut ws = state(&[("n", 10)]);
        Effect::Set(fact("n"), 3).apply(&mut ws);
        assert_eq!(ws.get(&fact("n")), 3);
        Effect::Add(fact("n"), 4).apply(&mut ws);
        assert_eq!(ws.get(&fact("n")), 7);
        Effect::Sub(fact("n"), 2).apply(&mut ws);
        assert_eq!(ws.get(&fact("n")), 5);
        let mut edge = state(&[("m", i64::MIN + 1)]);
        Effect::Sub(fact("m"), 10).apply(&mut edge);
        assert_eq!(edge.get(&fact("m")), i64::MIN);
    }

    #[test]
    fn action_apply_does_not_mutate_input() {
        let a = action("gain", vec![], vec![Effect::Add(fact("n"), 1)], 1);
        let before = state(&[("n", 1)]);
        let after = a.apply(&before);
        assert_eq!(before.get(&fact("n")), 1);
        assert_eq!(after.get(&fact("n")), 2);
    }

    // Планирование

    #[test]
    fn single_action_achieves_goal() {
        let actions = vec![action(
            "fly",
            vec![],
            vec![Effect::Set(fact("there"), 1)],
            5,
        )];
        let plan = Planner::with_defaults()
            .plan(
                &WorldState::default(),
                &goal("g", &[("there", 1)], 1),
                &actions,
            )
            .unwrap();
        assert_eq!(names(&plan), vec!["fly"]);
        assert_eq!(plan.total_cost, 5);
    }

    #[test]
    fn two_action_chain_achieves_goal() {
        let actions = vec![
            action(
                "sell",
                vec![Condition::Eq(fact("has_cargo"), 1)],
                vec![
                    Effect::Set(fact("has_cargo"), 0),
                    Effect::Add(fact("credits"), 100),
                ],
                1,
            ),
            action("load", vec![], vec![Effect::Set(fact("has_cargo"), 1)], 2),
        ];
        let target = goal("rich", &[("credits", 100)], 1);
        let plan = Planner::with_defaults()
            .plan(&WorldState::default(), &target, &actions)
            .unwrap();
        assert_eq!(names(&plan), vec!["load", "sell"]);
        assert_eq!(plan.total_cost, 3);
        assert!(plan.apply(&WorldState::default()).matches(&target.desired));
    }

    #[test]
    fn no_plan_when_unreachable() {
        let actions = vec![action(
            "needs_key",
            vec![Condition::Eq(fact("key"), 1)],
            vec![Effect::Set(fact("open"), 1)],
            1,
        )];
        let result = Planner::with_defaults().plan(
            &WorldState::default(),
            &goal("g", &[("open", 1)], 1),
            &actions,
        );
        assert_eq!(result, Err(PlanError::NoPlanFound));
    }

    #[test]
    fn higher_cost_plan_avoided() {
        let actions = vec![
            action("direct", vec![], vec![Effect::Set(fact("done"), 1)], 10),
            action("prepare", vec![], vec![Effect::Set(fact("ready"), 1)], 1),
            action(
                "finish",
                vec![Condition::Eq(fact("ready"), 1)],
                vec![Effect::Set(fact("done"), 1)],
                1,
            ),
        ];
        let plan = Planner::with_defaults()
            .plan(
                &WorldState::default(),
                &goal("g", &[("done", 1)], 1),
                &actions,
            )
            .unwrap();
        assert_eq!(names(&plan), vec!["prepare", "finish"]);
        assert_eq!(plan.total_cost, 2);
    }

    #[test]
    fn cheap_direct_action_beats_expensive_chain() {
        let actions = vec![
            action("prepare", vec![], vec![Effect::Set(fact("ready"), 1)], 5),
            action(
                "finish",
                vec![Condition::Eq(fact("ready"), 1)],
                vec![Effect::Set(fact("done"), 1)],
                5,
            ),
            action("direct", vec![], vec![Effect::Set(fact("done"), 1)], 3),
        ];
        let plan = Planner::with_defaults()
            .plan(
                &WorldState::default(),
                &goal("g", &[("done", 1)], 1),
                &actions,
            )
            .unwrap();
        assert_eq!(names(&plan), vec!["direct"]);
    }

    #[test]
    fn already_satisfied_goal_gives_empty_plan() {
        let plan = Planner::with_defaults()
            .plan(&state(&[("done", 1)]), &goal("g", &[("done", 1)], 1), &[])
            .unwrap();
        assert!(plan.actions.is_empty());
        assert_eq!(plan.total_cost, 0);
    }

    #[test]
    fn zero_cost_actions_are_planned() {
        let actions = vec![
            action("a", vec![], vec![Effect::Set(fact("x"), 1)], 0),
            action(
                "b",
                vec![Condition::Eq(fact("x"), 1)],
                vec![Effect::Set(fact("y"), 1)],
                0,
            ),
        ];
        let plan = Planner::with_defaults()
            .plan(&WorldState::default(), &goal("g", &[("y", 1)], 1), &actions)
            .unwrap();
        assert_eq!(names(&plan), vec!["a", "b"]);
        assert_eq!(plan.total_cost, 0);
    }

    #[test]
    fn plan_cost_saturates() {
        let actions = vec![
            action("a", vec![], vec![Effect::Set(fact("x"), 1)], u32::MAX),
            action(
                "b",
                vec![Condition::Eq(fact("x"), 1)],
                vec![Effect::Set(fact("y"), 1)],
                u32::MAX,
            ),
        ];
        let plan = Planner::with_defaults()
            .plan(&WorldState::default(), &goal("g", &[("y", 1)], 1), &actions)
            .unwrap();
        assert_eq!(plan.total_cost, u32::MAX);
    }

    // Детерминизм

    #[test]
    fn tie_break_is_deterministic() {
        let a = action("go_a", vec![], vec![Effect::Set(fact("done"), 1)], 4);
        let b = action("go_b", vec![], vec![Effect::Set(fact("done"), 1)], 4);
        let target = goal("g", &[("done", 1)], 1);
        let planner = Planner::with_defaults();
        for actions in [vec![a.clone(), b.clone()], vec![b, a]] {
            let plan = planner
                .plan(&WorldState::default(), &target, &actions)
                .unwrap();
            assert_eq!(names(&plan), vec!["go_a"]);
        }
    }

    #[test]
    fn identical_input_gives_identical_plans() {
        let actions = vec![
            action("m", vec![], vec![Effect::Add(fact("n"), 1)], 1),
            action("k", vec![], vec![Effect::Add(fact("n"), 1)], 1),
            action("z", vec![], vec![Effect::Add(fact("n"), 2)], 2),
        ];
        let target = goal("g", &[("n", 4)], 1);
        let planner = Planner::with_defaults();
        let first = planner
            .plan(&WorldState::default(), &target, &actions)
            .unwrap();
        for _ in 0..10 {
            let again = planner
                .plan(&WorldState::default(), &target, &actions)
                .unwrap();
            assert_eq!(first, again);
        }
    }

    #[test]
    fn search_exhausted_when_max_depth_reached() {
        let actions = vec![
            action("s1", vec![], vec![Effect::Set(fact("a"), 1)], 1),
            action(
                "s2",
                vec![Condition::Eq(fact("a"), 1)],
                vec![Effect::Set(fact("b"), 1)],
                1,
            ),
            action(
                "s3",
                vec![Condition::Eq(fact("b"), 1)],
                vec![Effect::Set(fact("c"), 1)],
                1,
            ),
        ];
        let target = goal("g", &[("c", 1)], 1);
        assert_eq!(
            Planner::new(2, 10_000).plan(&WorldState::default(), &target, &actions),
            Err(PlanError::SearchExhausted)
        );
        let plan = Planner::new(3, 10_000)
            .plan(&WorldState::default(), &target, &actions)
            .unwrap();
        assert_eq!(names(&plan), vec!["s1", "s2", "s3"]);
    }

    #[test]
    fn search_exhausted_when_max_expansions_reached() {
        let actions = vec![action("tick", vec![], vec![Effect::Add(fact("n"), 1)], 1)];
        let target = goal("g", &[("n", 1000)], 1);
        let result = Planner::new(5000, 50).plan(&WorldState::default(), &target, &actions);
        assert_eq!(result, Err(PlanError::SearchExhausted));
    }

    #[test]
    fn plan_found_with_exact_expansion_budget() {
        let actions = vec![action("tick", vec![], vec![Effect::Add(fact("n"), 1)], 1)];
        let target = goal("g", &[("n", 3)], 1);
        // Три раскрытия: из состояний n=0, n=1, n=2.
        assert!(
            Planner::new(10, 3)
                .plan(&WorldState::default(), &target, &actions)
                .is_ok()
        );
        assert_eq!(
            Planner::new(10, 2).plan(&WorldState::default(), &target, &actions),
            Err(PlanError::SearchExhausted)
        );
    }

    // AgentBrain

    #[test]
    fn brain_picks_highest_priority_achievable_goal() {
        let actions = vec![
            action("mine", vec![], vec![Effect::Set(fact("ore"), 1)], 3),
            action("trade", vec![], vec![Effect::Set(fact("profit"), 1)], 2),
        ];
        let goals = vec![
            goal("unreachable_top", &[("impossible", 1)], 100),
            goal("profit", &[("profit", 1)], 50),
            goal("ore", &[("ore", 1)], 10),
        ];
        let brain = AgentBrain::new(goals, actions);
        let plan = brain.plan(&WorldState::default()).unwrap();
        assert_eq!(names(&plan), vec!["trade"]);
    }

    #[test]
    fn brain_breaks_priority_tie_by_goal_name() {
        let actions = vec![
            action("mine", vec![], vec![Effect::Set(fact("ore"), 1)], 1),
            action("trade", vec![], vec![Effect::Set(fact("profit"), 1)], 1),
        ];
        let goals = vec![
            goal("b_profit", &[("profit", 1)], 5),
            goal("a_ore", &[("ore", 1)], 5),
        ];
        let plan = AgentBrain::new(goals, actions)
            .plan(&WorldState::default())
            .unwrap();
        assert_eq!(names(&plan), vec!["mine"]);
    }

    #[test]
    fn brain_returns_no_plan_when_no_goal_achievable() {
        let actions = vec![action("mine", vec![], vec![Effect::Set(fact("ore"), 1)], 1)];
        let goals = vec![goal("g", &[("impossible", 1)], 1)];
        let brain = AgentBrain::new(goals, actions);
        assert_eq!(
            brain.plan(&WorldState::default()),
            Err(PlanError::NoPlanFound)
        );
        assert_eq!(
            AgentBrain::new(vec![], vec![]).plan(&WorldState::default()),
            Err(PlanError::NoPlanFound)
        );
    }

    #[test]
    fn brain_reports_exhausted_search_when_limit_hit() {
        let actions = vec![action("tick", vec![], vec![Effect::Add(fact("n"), 1)], 1)];
        let mut brain = AgentBrain::new(vec![goal("g", &[("n", 1000)], 1)], actions);
        brain.max_expansions = 20;
        assert_eq!(
            brain.plan(&WorldState::default()),
            Err(PlanError::SearchExhausted)
        );
    }

    // Помощники и ошибки

    #[test]
    fn fact_helpers_build_expected_names() {
        assert_eq!(fact_at_system(SystemId(3)), fact("at_system_3"));
        assert_eq!(fact_credits(), fact("credits"));
        assert_eq!(fact_has_item(ItemId(7)), fact("has_item_7"));
    }

    #[test]
    fn plan_error_display_messages() {
        assert_eq!(
            PlanError::NoPlanFound.to_string(),
            "no plan achieves the goal"
        );
        assert_eq!(
            PlanError::SearchExhausted.to_string(),
            "plan search limit exceeded"
        );
    }

    #[test]
    fn trade_scenario_with_entity_facts() {
        // Доставить груз в другую систему: загрузиться, долететь, продать.
        let here = fact_at_system(SystemId(1));
        let there = fact_at_system(SystemId(2));
        let cargo = fact_has_item(ItemId(10));
        let actions = vec![
            action(
                "buy_ore",
                vec![Condition::Eq(here.clone(), 1)],
                vec![Effect::Set(cargo.clone(), 1)],
                2,
            ),
            action(
                "fly_1_to_2",
                vec![Condition::Eq(here.clone(), 1)],
                vec![Effect::Set(here.clone(), 0), Effect::Set(there.clone(), 1)],
                5,
            ),
            action(
                "sell_ore",
                vec![
                    Condition::Eq(there.clone(), 1),
                    Condition::Gte(cargo.clone(), 1),
                ],
                vec![
                    Effect::Sub(cargo.clone(), 1),
                    Effect::Add(fact_credits(), 300),
                ],
                1,
            ),
        ];
        let start = state(&[("at_system_1", 1)]);
        let target = goal("earn", &[("credits", 300)], 1);
        let plan = Planner::with_defaults()
            .plan(&start, &target, &actions)
            .unwrap();
        assert_eq!(names(&plan), vec!["buy_ore", "fly_1_to_2", "sell_ore"]);
        assert_eq!(plan.total_cost, 8);
        assert!(plan.apply(&start).matches(&target.desired));
    }
}
