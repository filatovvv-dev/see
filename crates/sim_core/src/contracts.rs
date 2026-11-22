//! Контракты: данные и конечный автомат состояний.
//!
//! Модуль не проверяет наличие предметов, не генерирует контракты и не
//! начисляет награды: это ответственность оркестратора и ИИ-агентов.
//! После создания контракт иммутабелен, кроме состояния, которое меняется
//! только через методы. Время дискретное (тики, `u64`), дедлайн абсолютный.
//!
//! Все коллекции здесь детерминированы: используется только `BTreeMap`
//! (см. AGENTS.md, раздел «Главное правило детерминизма»).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::character::FactionId;
use crate::entities::{ItemId, SystemId};

/// Вид контракта и его условия.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractKind {
    /// Курьерский: доставить груз из одной системы в другую.
    Courier {
        /// Система отправления.
        from: SystemId,
        /// Система назначения.
        to: SystemId,
        /// Груз: `предмет -> количество`.
        cargo: BTreeMap<ItemId, u64>,
    },
    /// Боевой: нанести урон указанной фракции.
    Combat {
        /// Фракция-цель.
        target_faction: FactionId,
        /// Система, в которой нужно действовать (если ограничено).
        target_system: Option<SystemId>,
    },
    /// Производственный: произвести указанные предметы.
    Production {
        /// Требуемые предметы: `предмет -> количество`.
        required: BTreeMap<ItemId, u64>,
    },
}

/// Состояние контракта.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractState {
    /// Доступен для принятия.
    Available,
    /// Принят исполнителем.
    Accepted,
    /// Успешно выполнен (терминальное).
    Completed,
    /// Провален (терминальное).
    Failed,
    /// Истёк по дедлайну (терминальное).
    Expired,
}

impl ContractState {
    /// Возвращает `true` для терминальных состояний: `Completed`, `Failed`, `Expired`.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ContractState::Completed | ContractState::Failed | ContractState::Expired
        )
    }
}

/// Ошибка операции с контрактом или реестром контрактов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractError {
    /// Недопустимый переход между состояниями.
    InvalidTransition {
        /// Текущее состояние.
        from: ContractState,
        /// Состояние, в которое пытались перейти.
        to: ContractState,
    },
    /// Дедлайн уже наступил, принять контракт нельзя.
    DeadlinePassed {
        /// Тик дедлайна.
        deadline: u64,
        /// Тик, в который выполнялась операция.
        at: u64,
    },
    /// Дедлайн ещё не наступил, истечь контракт не может.
    NotYetExpired {
        /// Тик дедлайна.
        deadline: u64,
        /// Тик, в который выполнялась операция.
        at: u64,
    },
    /// Контракт с таким идентификатором не найден.
    ContractNotFound(u64),
    /// Контракт с таким идентификатором уже зарегистрирован.
    AlreadyRegistered(u64),
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::InvalidTransition { from, to } => {
                write!(f, "invalid contract transition: {from:?} -> {to:?}")
            }
            ContractError::DeadlinePassed { deadline, at } => {
                write!(f, "deadline passed: deadline {deadline}, at {at}")
            }
            ContractError::NotYetExpired { deadline, at } => {
                write!(f, "not yet expired: deadline {deadline}, at {at}")
            }
            ContractError::ContractNotFound(id) => write!(f, "contract {id} not found"),
            ContractError::AlreadyRegistered(id) => {
                write!(f, "contract {id} is already registered")
            }
        }
    }
}

impl std::error::Error for ContractError {}

/// Контракт: условия, награда, дедлайн и текущее состояние.
///
/// Все поля, кроме состояния, задаются при создании и не меняются.
/// Состояние меняется только методами [`accept`](Contract::accept),
/// [`complete`](Contract::complete), [`fail`](Contract::fail) и
/// [`expire`](Contract::expire). При ошибке перехода состояние не меняется.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contract {
    /// Уникальный идентификатор контракта.
    pub id: u64,
    /// Фракция-заказчик.
    pub issuer: FactionId,
    /// Вид контракта и его условия.
    pub kind: ContractKind,
    /// Награда в кредитах (начисляет оркестратор).
    pub reward_credits: u64,
    /// Награда в репутации (начисляет оркестратор).
    pub reward_reputation: i32,
    /// Абсолютный тик дедлайна.
    pub deadline_tick: u64,
    state: ContractState,
}

impl Contract {
    /// Создаёт контракт в состоянии [`ContractState::Available`].
    pub fn new(
        id: u64,
        issuer: FactionId,
        kind: ContractKind,
        reward_credits: u64,
        reward_reputation: i32,
        deadline_tick: u64,
    ) -> Self {
        Self {
            id,
            issuer,
            kind,
            reward_credits,
            reward_reputation,
            deadline_tick,
            state: ContractState::Available,
        }
    }

    /// Возвращает текущее состояние контракта.
    pub fn state(&self) -> ContractState {
        self.state
    }

    /// Принимает контракт в тик `at_tick`.
    ///
    /// Допустимо только из `Available` (иначе [`ContractError::InvalidTransition`])
    /// и только до дедлайна: при `at_tick >= deadline_tick` возвращает
    /// [`ContractError::DeadlinePassed`].
    pub fn accept(&mut self, at_tick: u64) -> Result<(), ContractError> {
        if self.state != ContractState::Available {
            return Err(ContractError::InvalidTransition {
                from: self.state,
                to: ContractState::Accepted,
            });
        }
        if at_tick >= self.deadline_tick {
            return Err(ContractError::DeadlinePassed {
                deadline: self.deadline_tick,
                at: at_tick,
            });
        }
        self.state = ContractState::Accepted;
        Ok(())
    }

    /// Помечает принятый контракт выполненным.
    ///
    /// Допустимо только из `Accepted`, иначе [`ContractError::InvalidTransition`].
    pub fn complete(&mut self) -> Result<(), ContractError> {
        self.transition_from_accepted(ContractState::Completed)
    }

    /// Помечает принятый контракт проваленным.
    ///
    /// Допустимо только из `Accepted`, иначе [`ContractError::InvalidTransition`].
    pub fn fail(&mut self) -> Result<(), ContractError> {
        self.transition_from_accepted(ContractState::Failed)
    }

    /// Истекает контракт в тик `at_tick`.
    ///
    /// Допустимо из `Available` и `Accepted`. Из терминальных состояний возвращает
    /// [`ContractError::InvalidTransition`], а при `at_tick < deadline_tick` —
    /// [`ContractError::NotYetExpired`].
    pub fn expire(&mut self, at_tick: u64) -> Result<(), ContractError> {
        if self.state.is_terminal() {
            return Err(ContractError::InvalidTransition {
                from: self.state,
                to: ContractState::Expired,
            });
        }
        if at_tick < self.deadline_tick {
            return Err(ContractError::NotYetExpired {
                deadline: self.deadline_tick,
                at: at_tick,
            });
        }
        self.state = ContractState::Expired;
        Ok(())
    }

    fn transition_from_accepted(&mut self, to: ContractState) -> Result<(), ContractError> {
        if self.state != ContractState::Accepted {
            return Err(ContractError::InvalidTransition {
                from: self.state,
                to,
            });
        }
        self.state = to;
        Ok(())
    }
}

/// Реестр контрактов, упорядоченный по идентификатору.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractRegistry {
    contracts: BTreeMap<u64, Contract>,
}

impl ContractRegistry {
    /// Создаёт пустой реестр.
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
        }
    }

    /// Регистрирует контракт.
    ///
    /// Возвращает [`ContractError::AlreadyRegistered`], если идентификатор занят.
    pub fn register(&mut self, contract: Contract) -> Result<(), ContractError> {
        if self.contracts.contains_key(&contract.id) {
            return Err(ContractError::AlreadyRegistered(contract.id));
        }
        self.contracts.insert(contract.id, contract);
        Ok(())
    }

    /// Возвращает контракт по идентификатору.
    pub fn get(&self, id: u64) -> Option<&Contract> {
        self.contracts.get(&id)
    }

    /// Возвращает изменяемую ссылку на контракт по идентификатору.
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Contract> {
        self.contracts.get_mut(&id)
    }

    /// Возвращает количество контрактов в реестре.
    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    /// Возвращает `true`, если реестр пуст.
    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }

    /// Итерирует контракты в порядке возрастания идентификатора.
    pub fn iter(&self) -> impl Iterator<Item = &Contract> {
        self.contracts.values()
    }

    /// Возвращает доступные (`Available`) контракты фракции `issuer`
    /// в порядке возрастания идентификатора.
    pub fn available_for_faction(&self, issuer: FactionId) -> Vec<&Contract> {
        self.contracts
            .values()
            .filter(|c| c.issuer == issuer && c.state() == ContractState::Available)
            .collect()
    }

    /// Истекает все нетерминальные контракты с `deadline_tick <= at_tick`.
    ///
    /// Обход идёт в порядке возрастания идентификатора. Возвращает количество
    /// контрактов, которые перешли в `Expired`.
    pub fn expire_overdue(&mut self, at_tick: u64) -> usize {
        let mut expired = 0;
        for contract in self.contracts.values_mut() {
            if !contract.state().is_terminal()
                && contract.deadline_tick <= at_tick
                && contract.expire(at_tick).is_ok()
            {
                expired += 1;
            }
        }
        expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACTION_A: FactionId = FactionId(1);
    const FACTION_B: FactionId = FactionId(2);

    fn production_kind() -> ContractKind {
        ContractKind::Production {
            required: BTreeMap::from([(ItemId(1), 10)]),
        }
    }

    fn contract(id: u64, issuer: FactionId, deadline_tick: u64) -> Contract {
        Contract::new(id, issuer, production_kind(), 100, 5, deadline_tick)
    }

    fn accepted(id: u64, deadline_tick: u64) -> Contract {
        let mut c = contract(id, FACTION_A, deadline_tick);
        c.accept(0).unwrap();
        c
    }

    #[test]
    fn new_contract_is_available() {
        let c = contract(1, FACTION_A, 100);
        assert_eq!(c.state(), ContractState::Available);
        assert_eq!(c.id, 1);
        assert_eq!(c.issuer, FACTION_A);
        assert_eq!(c.reward_credits, 100);
        assert_eq!(c.reward_reputation, 5);
        assert_eq!(c.deadline_tick, 100);
    }

    #[test]
    fn accept_from_available_succeeds() {
        let mut c = contract(1, FACTION_A, 100);
        assert_eq!(c.accept(50), Ok(()));
        assert_eq!(c.state(), ContractState::Accepted);
    }

    #[test]
    fn accept_from_accepted_is_invalid_transition() {
        let mut c = accepted(1, 100);
        assert_eq!(
            c.accept(10),
            Err(ContractError::InvalidTransition {
                from: ContractState::Accepted,
                to: ContractState::Accepted,
            })
        );
        assert_eq!(c.state(), ContractState::Accepted);
    }

    #[test]
    fn accept_at_or_after_deadline_fails_and_keeps_state() {
        let mut c = contract(1, FACTION_A, 100);
        assert_eq!(
            c.accept(100),
            Err(ContractError::DeadlinePassed {
                deadline: 100,
                at: 100,
            })
        );
        assert_eq!(c.state(), ContractState::Available);
        assert_eq!(
            c.accept(150),
            Err(ContractError::DeadlinePassed {
                deadline: 100,
                at: 150,
            })
        );
        assert_eq!(c.state(), ContractState::Available);
        // Последний допустимый тик — на единицу раньше дедлайна.
        assert_eq!(c.accept(99), Ok(()));
    }

    #[test]
    fn accept_from_terminal_states_is_invalid_transition() {
        for terminal in [
            ContractState::Completed,
            ContractState::Failed,
            ContractState::Expired,
        ] {
            let mut c = accepted(1, 100);
            match terminal {
                ContractState::Completed => c.complete().unwrap(),
                ContractState::Failed => c.fail().unwrap(),
                _ => c.expire(100).unwrap(),
            }
            assert_eq!(
                c.accept(0),
                Err(ContractError::InvalidTransition {
                    from: terminal,
                    to: ContractState::Accepted,
                })
            );
            assert_eq!(c.state(), terminal);
        }
    }

    #[test]
    fn complete_from_accepted_succeeds() {
        let mut c = accepted(1, 100);
        assert_eq!(c.complete(), Ok(()));
        assert_eq!(c.state(), ContractState::Completed);
    }

    #[test]
    fn complete_from_available_is_invalid_transition() {
        let mut c = contract(1, FACTION_A, 100);
        assert_eq!(
            c.complete(),
            Err(ContractError::InvalidTransition {
                from: ContractState::Available,
                to: ContractState::Completed,
            })
        );
        assert_eq!(c.state(), ContractState::Available);
    }

    #[test]
    fn complete_and_fail_from_terminal_are_invalid() {
        let mut c = accepted(1, 100);
        c.complete().unwrap();
        assert_eq!(
            c.complete(),
            Err(ContractError::InvalidTransition {
                from: ContractState::Completed,
                to: ContractState::Completed,
            })
        );
        assert_eq!(
            c.fail(),
            Err(ContractError::InvalidTransition {
                from: ContractState::Completed,
                to: ContractState::Failed,
            })
        );
        assert_eq!(c.state(), ContractState::Completed);
    }

    #[test]
    fn fail_from_accepted_succeeds() {
        let mut c = accepted(1, 100);
        assert_eq!(c.fail(), Ok(()));
        assert_eq!(c.state(), ContractState::Failed);
    }

    #[test]
    fn fail_from_available_is_invalid_transition() {
        let mut c = contract(1, FACTION_A, 100);
        assert_eq!(
            c.fail(),
            Err(ContractError::InvalidTransition {
                from: ContractState::Available,
                to: ContractState::Failed,
            })
        );
        assert_eq!(c.state(), ContractState::Available);
    }

    #[test]
    fn expire_before_deadline_fails_and_keeps_state() {
        let mut c = accepted(1, 100);
        assert_eq!(
            c.expire(99),
            Err(ContractError::NotYetExpired {
                deadline: 100,
                at: 99,
            })
        );
        assert_eq!(c.state(), ContractState::Accepted);
    }

    #[test]
    fn expire_at_or_after_deadline_from_accepted_succeeds() {
        let mut at_deadline = accepted(1, 100);
        assert_eq!(at_deadline.expire(100), Ok(()));
        assert_eq!(at_deadline.state(), ContractState::Expired);

        let mut after_deadline = accepted(2, 100);
        assert_eq!(after_deadline.expire(500), Ok(()));
        assert_eq!(after_deadline.state(), ContractState::Expired);
    }

    #[test]
    fn expire_from_available_succeeds_after_deadline() {
        let mut c = contract(1, FACTION_A, 100);
        assert_eq!(c.expire(100), Ok(()));
        assert_eq!(c.state(), ContractState::Expired);
    }

    #[test]
    fn expire_from_terminal_is_invalid_transition() {
        let mut completed = accepted(1, 100);
        completed.complete().unwrap();
        assert_eq!(
            completed.expire(1000),
            Err(ContractError::InvalidTransition {
                from: ContractState::Completed,
                to: ContractState::Expired,
            })
        );
        assert_eq!(completed.state(), ContractState::Completed);

        let mut failed = accepted(2, 100);
        failed.fail().unwrap();
        assert!(failed.expire(1000).is_err());
        assert_eq!(failed.state(), ContractState::Failed);

        let mut expired = contract(3, FACTION_A, 10);
        expired.expire(10).unwrap();
        assert_eq!(
            expired.expire(20),
            Err(ContractError::InvalidTransition {
                from: ContractState::Expired,
                to: ContractState::Expired,
            })
        );
    }

    #[test]
    fn is_terminal_is_correct_for_all_states() {
        assert!(!ContractState::Available.is_terminal());
        assert!(!ContractState::Accepted.is_terminal());
        assert!(ContractState::Completed.is_terminal());
        assert!(ContractState::Failed.is_terminal());
        assert!(ContractState::Expired.is_terminal());
    }

    #[test]
    fn register_duplicate_id_fails_and_keeps_original() {
        let mut registry = ContractRegistry::new();
        assert!(registry.is_empty());
        registry.register(contract(1, FACTION_A, 100)).unwrap();
        assert_eq!(
            registry.register(contract(1, FACTION_B, 200)),
            Err(ContractError::AlreadyRegistered(1))
        );
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(1).map(|c| c.issuer), Some(FACTION_A));
    }

    #[test]
    fn get_and_get_mut_work_and_mutation_goes_through_methods() {
        let mut registry = ContractRegistry::new();
        registry.register(contract(1, FACTION_A, 100)).unwrap();
        assert!(registry.get(2).is_none());
        assert!(registry.get_mut(2).is_none());

        if let Some(c) = registry.get_mut(1) {
            c.accept(5).unwrap();
        }
        assert_eq!(
            registry.get(1).map(|c| c.state()),
            Some(ContractState::Accepted)
        );
    }

    #[test]
    fn iter_returns_contracts_in_id_order() {
        let mut registry = ContractRegistry::new();
        for id in [30, 10, 20] {
            registry.register(contract(id, FACTION_A, 100)).unwrap();
        }
        let ids: Vec<u64> = registry.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }

    #[test]
    fn available_for_faction_filters_by_issuer_and_state_in_id_order() {
        let mut registry = ContractRegistry::new();
        // Намеренно регистрируем не по порядку id.
        registry.register(contract(5, FACTION_A, 100)).unwrap();
        registry.register(contract(2, FACTION_B, 100)).unwrap();
        registry.register(contract(3, FACTION_A, 100)).unwrap();
        registry.register(contract(1, FACTION_A, 100)).unwrap();
        registry.register(contract(4, FACTION_A, 100)).unwrap();

        // Принятый и выполненный контракты фракции A выпадают из выборки.
        registry.get_mut(3).unwrap().accept(0).unwrap();
        let done = registry.get_mut(4).unwrap();
        done.accept(0).unwrap();
        done.complete().unwrap();

        let ids: Vec<u64> = registry
            .available_for_faction(FACTION_A)
            .iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, vec![1, 5]);

        let ids_b: Vec<u64> = registry
            .available_for_faction(FACTION_B)
            .iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids_b, vec![2]);
        assert!(registry.available_for_faction(FactionId(99)).is_empty());
    }

    #[test]
    fn expire_overdue_expires_only_overdue_and_counts() {
        let mut registry = ContractRegistry::new();
        registry.register(contract(1, FACTION_A, 50)).unwrap();
        registry.register(contract(2, FACTION_A, 100)).unwrap();
        registry.register(contract(3, FACTION_A, 150)).unwrap();
        registry.get_mut(2).unwrap().accept(0).unwrap();

        // Контракты 1 (Available) и 2 (Accepted) просрочены, 3 — нет.
        assert_eq!(registry.expire_overdue(100), 2);
        assert_eq!(
            registry.get(1).map(|c| c.state()),
            Some(ContractState::Expired)
        );
        assert_eq!(
            registry.get(2).map(|c| c.state()),
            Some(ContractState::Expired)
        );
        assert_eq!(
            registry.get(3).map(|c| c.state()),
            Some(ContractState::Available)
        );

        // Повторный вызов не считает уже истёкшие.
        assert_eq!(registry.expire_overdue(100), 0);
        assert_eq!(registry.expire_overdue(150), 1);
    }

    #[test]
    fn expire_overdue_skips_terminal_contracts() {
        let mut registry = ContractRegistry::new();
        registry.register(contract(1, FACTION_A, 10)).unwrap();
        registry.register(contract(2, FACTION_A, 10)).unwrap();
        registry.register(contract(3, FACTION_A, 10)).unwrap();
        for id in [1, 2] {
            registry.get_mut(id).unwrap().accept(0).unwrap();
        }
        registry.get_mut(1).unwrap().complete().unwrap();
        registry.get_mut(2).unwrap().fail().unwrap();

        assert_eq!(registry.expire_overdue(1000), 1);
        assert_eq!(
            registry.get(1).map(|c| c.state()),
            Some(ContractState::Completed)
        );
        assert_eq!(
            registry.get(2).map(|c| c.state()),
            Some(ContractState::Failed)
        );
        assert_eq!(
            registry.get(3).map(|c| c.state()),
            Some(ContractState::Expired)
        );
    }

    #[test]
    fn expire_overdue_is_deterministic() {
        let build = || {
            let mut registry = ContractRegistry::new();
            for id in [7u64, 3, 9, 1, 5] {
                registry.register(contract(id, FACTION_A, id * 10)).unwrap();
            }
            registry
        };
        let (mut first, mut second) = (build(), build());
        // Дедлайны 10, 30, 50, 70, 90: на тике 55 просрочены контракты 1, 3 и 5.
        assert_eq!(first.expire_overdue(55), 3);
        assert_eq!(second.expire_overdue(55), 3);
        assert_eq!(first, second);

        let expired: Vec<u64> = first
            .iter()
            .filter(|c| c.state() == ContractState::Expired)
            .map(|c| c.id)
            .collect();
        assert_eq!(expired, vec![1, 3, 5]);
    }

    #[test]
    fn contract_error_display_contains_numbers() {
        assert_eq!(
            ContractError::DeadlinePassed {
                deadline: 100,
                at: 150,
            }
            .to_string(),
            "deadline passed: deadline 100, at 150"
        );
        assert_eq!(
            ContractError::NotYetExpired {
                deadline: 200,
                at: 120,
            }
            .to_string(),
            "not yet expired: deadline 200, at 120"
        );
        assert_eq!(
            ContractError::ContractNotFound(42).to_string(),
            "contract 42 not found"
        );
        assert_eq!(
            ContractError::AlreadyRegistered(7).to_string(),
            "contract 7 is already registered"
        );
        assert_eq!(
            ContractError::InvalidTransition {
                from: ContractState::Available,
                to: ContractState::Completed,
            }
            .to_string(),
            "invalid contract transition: Available -> Completed"
        );
    }

    #[test]
    fn all_contract_kinds_can_be_constructed() {
        let courier = ContractKind::Courier {
            from: SystemId(1),
            to: SystemId(2),
            cargo: BTreeMap::from([(ItemId(3), 4), (ItemId(1), 2)]),
        };
        let combat = ContractKind::Combat {
            target_faction: FACTION_B,
            target_system: Some(SystemId(9)),
        };
        let combat_anywhere = ContractKind::Combat {
            target_faction: FACTION_B,
            target_system: None,
        };
        for (id, kind) in [(1, courier), (2, combat), (3, combat_anywhere)] {
            let c = Contract::new(id, FACTION_A, kind.clone(), 0, 0, 10);
            assert_eq!(c.kind, kind);
            assert_eq!(c.state(), ContractState::Available);
        }
    }
}
