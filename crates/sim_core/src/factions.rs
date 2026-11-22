//! Фракции и межфракционные отношения.
//!
//! Отношения измеряются шкалой от [`MIN_RELATION`] (война) до [`MAX_RELATION`]
//! (союз), значение `0` означает нейтралитет. Репутация персонажа здесь не
//! хранится: она лежит в `Character.reputation`.
//!
//! Все коллекции здесь детерминированы: используется только `BTreeMap`
//! (см. AGENTS.md, раздел «Главное правило детерминизма»).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::character::FactionId;

/// Минимальное значение отношений (война).
pub const MIN_RELATION: i32 = -10;
/// Максимальное значение отношений (союз).
pub const MAX_RELATION: i32 = 10;
/// Значение отношений, при котором фракции находятся в состоянии войны.
pub const WAR_RELATION: i32 = -10;

/// Вид фракции.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactionKind {
    /// Крупное государство.
    State,
    /// Корпорация.
    Corporation,
    /// Независимое движение.
    Movement,
}

/// Фракция с набором отношений к другим фракциям.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faction {
    /// Идентификатор фракции.
    pub id: FactionId,
    /// Название фракции.
    pub name: String,
    /// Вид фракции.
    pub kind: FactionKind,
    relations: BTreeMap<FactionId, i32>,
}

impl Faction {
    /// Создаёт фракцию без заданных отношений (со всеми нейтралитет).
    pub fn new(id: FactionId, name: impl Into<String>, kind: FactionKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            relations: BTreeMap::new(),
        }
    }

    /// Возвращает отношение к фракции `other` или `0`, если оно не задано.
    pub fn relation_with(&self, other: FactionId) -> i32 {
        self.relations.get(&other).copied().unwrap_or(0)
    }

    /// Устанавливает одностороннее отношение к `other`, обрезая значение
    /// до диапазона `MIN_RELATION..=MAX_RELATION`.
    ///
    /// Метод публичен для внутреннего использования реестром и не предназначен
    /// для прямого вызова игровым кодом: он не обновляет вторую фракцию.
    /// Игровой код должен вызывать [`FactionRegistry::set_relation`].
    pub fn set_relation(&mut self, other: FactionId, value: i32) {
        self.relations
            .insert(other, value.clamp(MIN_RELATION, MAX_RELATION));
    }

    /// Возвращает `true`, если фракция находится в состоянии войны с `other`.
    pub fn is_at_war_with(&self, other: FactionId) -> bool {
        self.relation_with(other) <= WAR_RELATION
    }

    /// Возвращает `true`, если фракция в союзе с `other`.
    pub fn is_allied_with(&self, other: FactionId) -> bool {
        self.relation_with(other) >= MAX_RELATION
    }
}

/// Ошибка операции с реестром фракций.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    /// Фракция с таким идентификатором уже зарегистрирована.
    FactionAlreadyExists(FactionId),
    /// Фракция с таким идентификатором не зарегистрирована.
    FactionNotFound(FactionId),
    /// Попытка задать отношение фракции с самой собой.
    SameFaction(FactionId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::FactionAlreadyExists(id) => {
                write!(f, "faction {id:?} is already registered")
            }
            RegistryError::FactionNotFound(id) => {
                write!(f, "faction {id:?} is not registered")
            }
            RegistryError::SameFaction(id) => {
                write!(f, "cannot set relation of faction {id:?} with itself")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// Реестр фракций: единая точка изменения и чтения межфракционных отношений.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionRegistry {
    factions: BTreeMap<FactionId, Faction>,
}

impl FactionRegistry {
    /// Создаёт пустой реестр.
    pub fn new() -> Self {
        Self {
            factions: BTreeMap::new(),
        }
    }

    /// Регистрирует фракцию.
    ///
    /// Возвращает [`RegistryError::FactionAlreadyExists`], если идентификатор занят.
    pub fn register(&mut self, faction: Faction) -> Result<(), RegistryError> {
        if self.factions.contains_key(&faction.id) {
            return Err(RegistryError::FactionAlreadyExists(faction.id));
        }
        self.factions.insert(faction.id, faction);
        Ok(())
    }

    /// Возвращает фракцию по идентификатору.
    pub fn get(&self, id: FactionId) -> Option<&Faction> {
        self.factions.get(&id)
    }

    /// Возвращает изменяемую ссылку на фракцию по идентификатору.
    pub fn get_mut(&mut self, id: FactionId) -> Option<&mut Faction> {
        self.factions.get_mut(&id)
    }

    /// Возвращает количество зарегистрированных фракций.
    pub fn len(&self) -> usize {
        self.factions.len()
    }

    /// Возвращает `true`, если в реестре нет фракций.
    pub fn is_empty(&self) -> bool {
        self.factions.is_empty()
    }

    /// Итерирует фракции в порядке возрастания [`FactionId`].
    pub fn iter(&self) -> impl Iterator<Item = &Faction> {
        self.factions.values()
    }

    /// Симметрично устанавливает отношения между `a` и `b`.
    ///
    /// Значение обрезается до `MIN_RELATION..=MAX_RELATION` и записывается в обе
    /// стороны: после вызова `a.relation_with(b) == b.relation_with(a)`.
    ///
    /// Ошибки (состояние реестра при ошибке не меняется):
    /// - [`RegistryError::SameFaction`], если `a == b`;
    /// - [`RegistryError::FactionNotFound`], если `a` или `b` не зарегистрированы
    ///   (если нет обеих, сообщается об `a`).
    ///
    /// Асимметричные отношения (когда A видит B иначе, чем B видит A) в MVP не
    /// поддерживаются. API спроектирован так, что добавить их позже можно через
    /// отдельный метод `set_one_way_relation` без ломки существующего кода.
    pub fn set_relation(
        &mut self,
        a: FactionId,
        b: FactionId,
        value: i32,
    ) -> Result<(), RegistryError> {
        if a == b {
            return Err(RegistryError::SameFaction(a));
        }
        self.ensure_registered(a)?;
        self.ensure_registered(b)?;

        if let Some(faction) = self.factions.get_mut(&a) {
            faction.set_relation(b, value);
        }
        if let Some(faction) = self.factions.get_mut(&b) {
            faction.set_relation(a, value);
        }
        Ok(())
    }

    /// Возвращает отношение `a` к `b`; обе фракции должны быть зарегистрированы.
    pub fn relation_between(&self, a: FactionId, b: FactionId) -> Result<i32, RegistryError> {
        let faction_a = self
            .factions
            .get(&a)
            .ok_or(RegistryError::FactionNotFound(a))?;
        self.ensure_registered(b)?;
        Ok(faction_a.relation_with(b))
    }

    /// Возвращает `true`, если `a` и `b` находятся в состоянии войны.
    pub fn are_at_war(&self, a: FactionId, b: FactionId) -> Result<bool, RegistryError> {
        Ok(self.relation_between(a, b)? <= WAR_RELATION)
    }

    /// Объявляет войну: устанавливает отношения `WAR_RELATION` в обе стороны.
    pub fn declare_war(&mut self, a: FactionId, b: FactionId) -> Result<(), RegistryError> {
        self.set_relation(a, b, WAR_RELATION)
    }

    /// Заключает мир: устанавливает нейтральные отношения (`0`) в обе стороны.
    pub fn make_peace(&mut self, a: FactionId, b: FactionId) -> Result<(), RegistryError> {
        self.set_relation(a, b, 0)
    }

    fn ensure_registered(&self, id: FactionId) -> Result<(), RegistryError> {
        if self.factions.contains_key(&id) {
            Ok(())
        } else {
            Err(RegistryError::FactionNotFound(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: FactionId = FactionId(1);
    const B: FactionId = FactionId(2);
    const C: FactionId = FactionId(3);

    fn faction(id: FactionId, name: &str, kind: FactionKind) -> Faction {
        Faction::new(id, name, kind)
    }

    fn registry_ab() -> FactionRegistry {
        let mut registry = FactionRegistry::new();
        registry
            .register(faction(A, "Альфа", FactionKind::State))
            .unwrap();
        registry
            .register(faction(B, "Бета", FactionKind::Corporation))
            .unwrap();
        registry
    }

    #[test]
    fn register_new_faction_succeeds() {
        let mut registry = FactionRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(
            registry.register(faction(A, "Альфа", FactionKind::State)),
            Ok(())
        );
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn register_duplicate_id_fails_and_keeps_original() {
        let mut registry = FactionRegistry::new();
        registry
            .register(faction(A, "Альфа", FactionKind::State))
            .unwrap();
        let result = registry.register(faction(A, "Другая", FactionKind::Movement));
        assert_eq!(result, Err(RegistryError::FactionAlreadyExists(A)));
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(A).map(|f| f.name.as_str()), Some("Альфа"));
    }

    #[test]
    fn get_and_get_mut_return_correct_references() {
        let mut registry = registry_ab();
        assert_eq!(registry.get(A).map(|f| f.id), Some(A));
        assert_eq!(
            registry.get(B).map(|f| f.kind),
            Some(FactionKind::Corporation)
        );
        assert!(registry.get(C).is_none());

        if let Some(f) = registry.get_mut(A) {
            f.name = "Новое имя".to_string();
        }
        assert_eq!(registry.get(A).map(|f| f.name.as_str()), Some("Новое имя"));
        assert!(registry.get_mut(C).is_none());
    }

    #[test]
    fn set_relation_is_symmetric() {
        let mut registry = registry_ab();
        registry.set_relation(A, B, 5).unwrap();

        let a = registry.get(A).unwrap();
        let b = registry.get(B).unwrap();
        assert_eq!(a.relation_with(B), 5);
        assert_eq!(b.relation_with(A), 5);
        assert_eq!(a.relation_with(B), b.relation_with(A));
        assert_eq!(registry.relation_between(A, B), Ok(5));
        assert_eq!(registry.relation_between(B, A), Ok(5));
    }

    #[test]
    fn set_relation_symmetric_in_reverse_argument_order() {
        let mut registry = registry_ab();
        registry.set_relation(B, A, -3).unwrap();
        assert_eq!(registry.get(A).unwrap().relation_with(B), -3);
        assert_eq!(registry.get(B).unwrap().relation_with(A), -3);
    }

    #[test]
    fn set_relation_clamps_values() {
        let mut registry = registry_ab();
        registry.set_relation(A, B, -100).unwrap();
        assert_eq!(registry.relation_between(A, B), Ok(-10));
        assert_eq!(registry.relation_between(B, A), Ok(-10));

        registry.set_relation(A, B, 100).unwrap();
        assert_eq!(registry.relation_between(A, B), Ok(10));
        assert_eq!(registry.relation_between(B, A), Ok(10));

        registry.set_relation(A, B, i32::MIN).unwrap();
        assert_eq!(registry.relation_between(A, B), Ok(MIN_RELATION));
        registry.set_relation(A, B, i32::MAX).unwrap();
        assert_eq!(registry.relation_between(A, B), Ok(MAX_RELATION));
    }

    #[test]
    fn set_relation_with_same_faction_fails() {
        let mut registry = registry_ab();
        assert_eq!(
            registry.set_relation(A, A, 5),
            Err(RegistryError::SameFaction(A))
        );
        // Ошибка SameFaction приоритетнее проверки существования.
        assert_eq!(
            registry.set_relation(C, C, 5),
            Err(RegistryError::SameFaction(C))
        );
    }

    #[test]
    fn set_relation_with_missing_faction_fails() {
        let mut registry = registry_ab();
        assert_eq!(
            registry.set_relation(A, C, 5),
            Err(RegistryError::FactionNotFound(C))
        );
        assert_eq!(
            registry.set_relation(C, A, 5),
            Err(RegistryError::FactionNotFound(C))
        );
        // Если нет обеих, сообщается о первой (a).
        assert_eq!(
            registry.set_relation(FactionId(8), FactionId(9), 5),
            Err(RegistryError::FactionNotFound(FactionId(8)))
        );
    }

    #[test]
    fn failed_set_relation_does_not_change_state() {
        let mut registry = registry_ab();
        let before = registry.clone();
        assert!(registry.set_relation(A, C, 7).is_err());
        assert!(registry.set_relation(A, A, 7).is_err());
        assert_eq!(registry, before);
    }

    #[test]
    fn declare_war_sets_minimum_and_are_at_war() {
        let mut registry = registry_ab();
        registry.declare_war(A, B).unwrap();
        assert_eq!(registry.relation_between(A, B), Ok(-10));
        assert_eq!(registry.relation_between(B, A), Ok(-10));
        assert_eq!(registry.are_at_war(A, B), Ok(true));
        assert_eq!(registry.are_at_war(B, A), Ok(true));
        assert!(registry.get(A).unwrap().is_at_war_with(B));
    }

    #[test]
    fn make_peace_sets_neutral_and_ends_war() {
        let mut registry = registry_ab();
        registry.declare_war(A, B).unwrap();
        registry.make_peace(A, B).unwrap();
        assert_eq!(registry.relation_between(A, B), Ok(0));
        assert_eq!(registry.relation_between(B, A), Ok(0));
        assert_eq!(registry.are_at_war(A, B), Ok(false));
    }

    #[test]
    fn war_and_peace_require_registered_distinct_factions() {
        let mut registry = registry_ab();
        assert_eq!(
            registry.declare_war(A, C),
            Err(RegistryError::FactionNotFound(C))
        );
        assert_eq!(
            registry.make_peace(B, B),
            Err(RegistryError::SameFaction(B))
        );
        assert_eq!(
            registry.are_at_war(A, C),
            Err(RegistryError::FactionNotFound(C))
        );
        assert_eq!(
            registry.relation_between(C, A),
            Err(RegistryError::FactionNotFound(C))
        );
    }

    #[test]
    fn is_allied_with_true_only_at_max() {
        let mut registry = registry_ab();
        registry.set_relation(A, B, 10).unwrap();
        assert!(registry.get(A).unwrap().is_allied_with(B));
        assert!(registry.get(B).unwrap().is_allied_with(A));

        registry.set_relation(A, B, 9).unwrap();
        assert!(!registry.get(A).unwrap().is_allied_with(B));
    }

    #[test]
    fn unset_relation_is_neutral() {
        let registry = registry_ab();
        let a = registry.get(A).unwrap();
        assert_eq!(a.relation_with(B), 0);
        assert_eq!(a.relation_with(C), 0);
        assert!(!a.is_at_war_with(B));
        assert!(!a.is_allied_with(B));
        assert_eq!(registry.relation_between(A, B), Ok(0));
        assert_eq!(registry.are_at_war(A, B), Ok(false));
    }

    #[test]
    fn faction_level_set_relation_clamps_but_is_one_way() {
        let mut a = faction(A, "Альфа", FactionKind::Movement);
        a.set_relation(B, 99);
        assert_eq!(a.relation_with(B), MAX_RELATION);
        a.set_relation(B, -99);
        assert_eq!(a.relation_with(B), MIN_RELATION);
        assert!(a.is_at_war_with(B));
    }

    #[test]
    fn iter_returns_factions_in_id_order() {
        let mut registry = FactionRegistry::new();
        for id in [30u64, 10, 20] {
            registry
                .register(faction(FactionId(id), "Ф", FactionKind::State))
                .unwrap();
        }
        let ids: Vec<u64> = registry.iter().map(|f| f.id.0).collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }

    #[test]
    fn registry_error_display_messages() {
        assert_eq!(
            RegistryError::FactionAlreadyExists(A).to_string(),
            "faction FactionId(1) is already registered"
        );
        assert_eq!(
            RegistryError::FactionNotFound(B).to_string(),
            "faction FactionId(2) is not registered"
        );
        assert_eq!(
            RegistryError::SameFaction(C).to_string(),
            "cannot set relation of faction FactionId(3) with itself"
        );
    }

    #[test]
    fn faction_new_accepts_str_and_string() {
        let from_str = Faction::new(A, "Движ Первых", FactionKind::Movement);
        let from_string = Faction::new(A, String::from("Движ Первых"), FactionKind::Movement);
        assert_eq!(from_str, from_string);
        assert_eq!(from_str.name, "Движ Первых");
    }

    #[test]
    fn registry_state_is_reproducible() {
        let build = || {
            let mut registry = registry_ab();
            registry
                .register(faction(C, "Гамма", FactionKind::Movement))
                .unwrap();
            registry.set_relation(A, B, 4).unwrap();
            registry.declare_war(B, C).unwrap();
            registry.set_relation(A, C, -250).unwrap();
            registry
        };
        assert_eq!(build(), build());
    }
}
