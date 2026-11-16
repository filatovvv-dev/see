//! Базовые сущности игрового мира SEE.
//!
//! Содержит newtype-идентификаторы, агентов, корабли, груз и рыночные ордера.
//! Все коллекции здесь детерминированы: используется только `BTreeMap`
//! (см. AGENTS.md, раздел «Главное правило детерминизма»).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Идентификатор агента (персонажа/фракции-владельца).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AgentId(pub u64);

/// Идентификатор корабля.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShipId(pub u64);

/// Идентификатор предмета/товара.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemId(pub u64);

/// Идентификатор звёздной системы (локации).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SystemId(pub u64);

/// Ошибка операции с грузом.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoError {
    /// Недостаточно предметов для списания.
    InsufficientQuantity {
        /// Сколько предметов фактически есть в трюме.
        have: u64,
        /// Сколько предметов пытались списать.
        want: u64,
    },
}

impl fmt::Display for CargoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CargoError::InsufficientQuantity { have, want } => {
                write!(f, "insufficient cargo quantity: have {have}, want {want}")
            }
        }
    }
}

impl std::error::Error for CargoError {}

/// Грузовой трюм корабля: карта `предмет -> количество`.
///
/// Внутреннее состояние приватно, чтобы нельзя было нарушить инварианты
/// (например, оставить запись с нулевым количеством) в обход методов.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cargo {
    items: BTreeMap<ItemId, u64>,
}

impl Cargo {
    /// Создаёт пустой трюм.
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
        }
    }

    /// Добавляет `qty` предметов `item` в трюм.
    ///
    /// Нулевое количество является операцией без эффекта.
    pub fn add(&mut self, item: ItemId, qty: u64) {
        if qty == 0 {
            return;
        }
        *self.items.entry(item).or_insert(0) += qty;
    }

    /// Списывает `qty` предметов `item` из трюма.
    ///
    /// Возвращает [`CargoError::InsufficientQuantity`], если предметов меньше,
    /// чем запрошено. При полном списании запись удаляется.
    pub fn remove(&mut self, item: ItemId, qty: u64) -> Result<(), CargoError> {
        let have = self.get(item);
        if have < qty {
            return Err(CargoError::InsufficientQuantity { have, want: qty });
        }
        if have == qty {
            self.items.remove(&item);
        } else {
            // `have > qty`, значит запись точно существует.
            if let Some(entry) = self.items.get_mut(&item) {
                *entry -= qty;
            }
        }
        Ok(())
    }

    /// Возвращает количество предметов `item` в трюме (0, если их нет).
    pub fn get(&self, item: ItemId) -> u64 {
        self.items.get(&item).copied().unwrap_or(0)
    }

    /// Возвращает суммарное количество всех предметов в трюме.
    pub fn total(&self) -> u64 {
        self.items.values().sum()
    }

    /// Возвращает `true`, если трюм пуст.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Агент — субъект экономики мира (персонаж, фракция, NPC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// Уникальный идентификатор агента.
    pub id: AgentId,
    /// Количество кредитов на счету.
    pub credits: u64,
    /// Текущая локация агента.
    pub location: SystemId,
}

/// Корабль — собственность агента с грузовым трюмом.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ship {
    /// Уникальный идентификатор корабля.
    pub id: ShipId,
    /// Агент-владелец корабля.
    pub owner: AgentId,
    /// Грузовой трюм.
    pub cargo: Cargo,
}

/// Ордер на рынке: покупка или продажа предмета по фиксированной цене.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketOrder {
    /// Уникальный идентификатор ордера.
    pub order_id: u64,
    /// Предмет, которым торгуют.
    pub item: ItemId,
    /// Цена за единицу в кредитах.
    pub price: u64,
    /// Количество единиц в ордере.
    pub quantity: u64,
    /// `true` — ордер на покупку, `false` — на продажу.
    pub is_buy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_comparable_and_hashable() {
        assert_eq!(AgentId(1), AgentId(1));
        assert_ne!(AgentId(1), AgentId(2));
        assert!(ShipId(1) < ShipId(2));
        assert!(ItemId(5) > ItemId(4));
        assert!(SystemId(0) <= SystemId(0));

        let mut set = std::collections::BTreeSet::new();
        set.insert(AgentId(1));
        set.insert(AgentId(2));
        set.insert(AgentId(1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn cargo_add_and_get() {
        let mut cargo = Cargo::new();
        cargo.add(ItemId(10), 5);
        assert_eq!(cargo.get(ItemId(10)), 5);
        cargo.add(ItemId(10), 7);
        assert_eq!(cargo.get(ItemId(10)), 12);
        assert_eq!(cargo.get(ItemId(99)), 0);
    }

    #[test]
    fn cargo_add_zero_is_noop() {
        let mut cargo = Cargo::new();
        cargo.add(ItemId(1), 0);
        assert!(cargo.is_empty());
        assert_eq!(cargo.total(), 0);
    }

    #[test]
    fn cargo_remove_success() {
        let mut cargo = Cargo::new();
        cargo.add(ItemId(3), 10);
        assert_eq!(cargo.remove(ItemId(3), 4), Ok(()));
        assert_eq!(cargo.get(ItemId(3)), 6);
        assert_eq!(cargo.remove(ItemId(3), 6), Ok(()));
        assert!(cargo.is_empty());
    }

    #[test]
    fn cargo_remove_insufficient_returns_error() {
        let mut cargo = Cargo::new();
        cargo.add(ItemId(7), 3);
        let err = cargo.remove(ItemId(7), 5).expect_err("should fail");
        assert_eq!(err, CargoError::InsufficientQuantity { have: 3, want: 5 });
        // Состояние не изменилось после неудачной операции.
        assert_eq!(cargo.get(ItemId(7)), 3);
    }

    #[test]
    fn cargo_remove_from_missing_item_errors() {
        let mut cargo = Cargo::new();
        let err = cargo.remove(ItemId(42), 1).expect_err("should fail");
        assert_eq!(err, CargoError::InsufficientQuantity { have: 0, want: 1 });
    }

    #[test]
    fn cargo_error_display() {
        let err = CargoError::InsufficientQuantity { have: 2, want: 9 };
        assert_eq!(
            format!("{err}"),
            "insufficient cargo quantity: have 2, want 9"
        );
    }

    #[test]
    fn cargo_total_and_default() {
        let mut cargo = Cargo::default();
        cargo.add(ItemId(1), 2);
        cargo.add(ItemId(2), 3);
        assert_eq!(cargo.total(), 5);
        assert!(!cargo.is_empty());
    }

    #[test]
    fn agent_and_ship_construction_compiles_and_works() {
        let agent = Agent {
            id: AgentId(1),
            credits: 1000,
            location: SystemId(7),
        };
        let mut ship = Ship {
            id: ShipId(1),
            owner: agent.id,
            cargo: Cargo::new(),
        };
        ship.cargo.add(ItemId(5), 100);
        assert_eq!(ship.owner, AgentId(1));
        assert_eq!(ship.cargo.get(ItemId(5)), 100);
        assert_eq!(agent.credits, 1000);
    }

    #[test]
    fn market_order_fields() {
        let order = MarketOrder {
            order_id: 12,
            item: ItemId(3),
            price: 50,
            quantity: 4,
            is_buy: true,
        };
        assert_eq!(order.order_id, 12);
        assert!(order.is_buy);
        assert_eq!(order.price * order.quantity, 200);
    }

    #[test]
    fn cargo_deterministic_iteration_order() {
        let mut cargo = Cargo::new();
        cargo.add(ItemId(30), 1);
        cargo.add(ItemId(10), 1);
        cargo.add(ItemId(20), 1);
        let ids: Vec<u64> = cargo.items.keys().map(|id| id.0).collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }
}
