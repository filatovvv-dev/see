//! Персонаж: мозг, нейросеть, импланты и базы знаний.
//!
//! Развитие персонажа трёхслойное:
//! - **мозг** хранит изученные базы знаний навсегда;
//! - **нейросеть** — заменяемый аппарат, смена которого не затрагивает мозг;
//! - **импланты** компенсируют низкий IQ или малый объём памяти.
//!
//! Все коллекции здесь детерминированы: используются только `BTreeMap`,
//! `BTreeSet` и `Vec` (см. AGENTS.md, раздел «Главное правило детерминизма»).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

/// Идентификатор нейросети.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NeuralNetId(pub u64);

/// Идентификатор базы знаний.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KnowledgeBaseId(pub u64);

/// Идентификатор фракции.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FactionId(pub u64);

/// Вид нейросети.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeuralNetKind {
    /// Базовая: бесплатная, работает везде, без бонусов.
    Basic,
    /// Специализированная: после второго носителя-человека становится токсичной,
    /// зато у животного работает именно на втором цикле.
    Specialized,
    /// Индивидуальная: одноразовая, после снятия выбрасывается.
    Individual,
}

/// Ошибка операции с нейросетью.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuralNetError {
    /// Нейросеть исчерпала число установок для человека.
    NotInstallableForHuman,
    /// Нейросеть подходит только животному, человеку её ставить нельзя.
    InstallableOnlyForAnimal,
    /// Счётчик установок достиг максимума.
    Overflow,
}

impl fmt::Display for NeuralNetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NeuralNetError::NotInstallableForHuman => {
                write!(f, "neural net is not installable for human")
            }
            NeuralNetError::InstallableOnlyForAnimal => {
                write!(f, "neural net is installable only for animal")
            }
            NeuralNetError::Overflow => write!(f, "neural net install counter overflow"),
        }
    }
}

impl std::error::Error for NeuralNetError {}

/// Ошибка изучения базы знаний.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyError {
    /// Эффективный IQ персонажа ниже требуемого.
    InsufficientIq,
    /// Эффективный объём памяти персонажа ниже требуемого.
    InsufficientMemory,
}

impl fmt::Display for StudyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StudyError::InsufficientIq => {
                write!(f, "character iq is too low to study this knowledge base")
            }
            StudyError::InsufficientMemory => {
                write!(
                    f,
                    "character memory is too low to study this knowledge base"
                )
            }
        }
    }
}

impl std::error::Error for StudyError {}

/// Нейросеть: заменяемый аппарат, дающий бонусы носителю.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuralNet {
    /// Идентификатор нейросети.
    pub id: NeuralNetId,
    /// Вид нейросети.
    pub kind: NeuralNetKind,
    /// Сколько раз нейросеть уже устанавливалась носителям.
    pub install_count: u8,
}

impl NeuralNet {
    /// Создаёт базовую нейросеть (`install_count = 1`).
    pub fn new_basic(id: NeuralNetId) -> Self {
        Self {
            id,
            kind: NeuralNetKind::Basic,
            install_count: 1,
        }
    }

    /// Создаёт специализированную нейросеть (`install_count = 0`).
    pub fn new_specialized(id: NeuralNetId) -> Self {
        Self {
            id,
            kind: NeuralNetKind::Specialized,
            install_count: 0,
        }
    }

    /// Создаёт индивидуальную нейросеть (`install_count = 0`).
    pub fn new_individual(id: NeuralNetId) -> Self {
        Self {
            id,
            kind: NeuralNetKind::Individual,
            install_count: 0,
        }
    }

    /// Возвращает `true`, если нейросеть можно установить человеку.
    ///
    /// Базовая подходит всегда, специализированная — пока установок меньше двух,
    /// индивидуальная — пока она ещё не устанавливалась.
    pub fn is_installable_for_human(&self) -> bool {
        match self.kind {
            NeuralNetKind::Basic => true,
            NeuralNetKind::Specialized => self.install_count < 2,
            NeuralNetKind::Individual => self.install_count < 1,
        }
    }

    /// Возвращает `true`, если нейросеть можно установить животному.
    ///
    /// Базовая животным не нужна. Специализированная работает у животного
    /// ровно после двух установок. Индивидуальную технически поставить можно.
    pub fn is_installable_for_animal(&self) -> bool {
        match self.kind {
            NeuralNetKind::Basic => false,
            NeuralNetKind::Specialized => self.install_count == 2,
            NeuralNetKind::Individual => true,
        }
    }

    /// Увеличивает счётчик установок на единицу.
    ///
    /// Проверку применимости (`is_installable_*`) вызывающий код делает сам.
    /// Возвращает [`NeuralNetError::Overflow`], если счётчик уже равен `u8::MAX`.
    pub fn install(&mut self) -> Result<(), NeuralNetError> {
        self.install_count = self
            .install_count
            .checked_add(1)
            .ok_or(NeuralNetError::Overflow)?;
        Ok(())
    }
}

/// Имплант: дополнительное железо с узкими бонусами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Implant {
    /// Бонус к IQ (может быть отрицательным).
    pub iq_bonus: i32,
    /// Бонус к объёму памяти в петабайтах (может быть отрицательным).
    pub memory_bonus_pb: i32,
}

/// База знаний, которую можно изучить.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// Идентификатор базы знаний.
    pub id: KnowledgeBaseId,
    /// Минимальный эффективный IQ для изучения.
    pub required_iq: u32,
    /// Минимальный эффективный объём памяти (в петабайтах) для изучения.
    pub required_memory_pb: u32,
    /// Время изучения в тиках симуляции.
    pub study_time_ticks: u64,
}

/// Мозг персонажа: базовые параметры и изученные базы знаний.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brain {
    /// Базовый IQ.
    pub iq: u32,
    /// Базовый объём памяти в петабайтах.
    pub memory_capacity_pb: u32,
    /// Изученные базы знаний. Хранятся в мозге навсегда.
    pub known_bases: BTreeSet<KnowledgeBaseId>,
}

impl Brain {
    /// Создаёт мозг без изученных баз знаний.
    pub fn new(iq: u32, memory_pb: u32) -> Self {
        Self {
            iq,
            memory_capacity_pb: memory_pb,
            known_bases: BTreeSet::new(),
        }
    }

    /// Возвращает `true`, если база знаний `id` уже изучена.
    pub fn has_base(&self, id: KnowledgeBaseId) -> bool {
        self.known_bases.contains(&id)
    }
}

/// Приводит знаковую сумму к `u32`, обрезая значение до диапазона `0..=u32::MAX`.
fn clamp_to_u32(value: i64) -> u32 {
    u32::try_from(value.clamp(0, i64::from(u32::MAX))).unwrap_or(u32::MAX)
}

/// Персонаж: мозг, установленная нейросеть, импланты и репутация.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Character {
    /// Мозг персонажа.
    pub brain: Brain,
    /// Установленная нейросеть, если есть.
    pub installed_neural_net: Option<NeuralNet>,
    /// Установленные импланты.
    pub implants: Vec<Implant>,
    /// Репутация по фракциям. Пока только хранится, логики работы с ней нет.
    pub reputation: BTreeMap<FactionId, i32>,
}

impl Character {
    /// Создаёт персонажа без нейросети, имплантов и репутации.
    pub fn new(brain: Brain) -> Self {
        Self {
            brain,
            installed_neural_net: None,
            implants: Vec::new(),
            reputation: BTreeMap::new(),
        }
    }

    /// Эффективный IQ: базовый плюс бонусы имплантов, в диапазоне `0..=u32::MAX`.
    pub fn effective_iq(&self) -> u32 {
        let total = self
            .implants
            .iter()
            .fold(i64::from(self.brain.iq), |acc, implant| {
                acc.saturating_add(i64::from(implant.iq_bonus))
            });
        clamp_to_u32(total)
    }

    /// Эффективный объём памяти в петабайтах: базовый плюс бонусы имплантов,
    /// в диапазоне `0..=u32::MAX`.
    pub fn effective_memory_pb(&self) -> u32 {
        let total = self
            .implants
            .iter()
            .fold(i64::from(self.brain.memory_capacity_pb), |acc, implant| {
                acc.saturating_add(i64::from(implant.memory_bonus_pb))
            });
        clamp_to_u32(total)
    }

    /// Возвращает `true`, если эффективных IQ и памяти хватает для базы `kb`.
    pub fn can_study(&self, kb: &KnowledgeBase) -> bool {
        self.effective_iq() >= kb.required_iq && self.effective_memory_pb() >= kb.required_memory_pb
    }

    /// Изучает базу знаний `kb`.
    ///
    /// Если база уже изучена, операция идемпотентна и возвращает `Ok(())`:
    /// знания в мозге не теряются, даже если требования больше не выполняются.
    /// Иначе при нехватке IQ возвращает [`StudyError::InsufficientIq`], при
    /// нехватке памяти — [`StudyError::InsufficientMemory`] (IQ проверяется первым).
    pub fn study(&mut self, kb: &KnowledgeBase) -> Result<(), StudyError> {
        if self.brain.has_base(kb.id) {
            return Ok(());
        }
        if self.effective_iq() < kb.required_iq {
            return Err(StudyError::InsufficientIq);
        }
        if self.effective_memory_pb() < kb.required_memory_pb {
            return Err(StudyError::InsufficientMemory);
        }
        self.brain.known_bases.insert(kb.id);
        Ok(())
    }

    /// Устанавливает нейросеть человеку, заменяя прежнюю.
    ///
    /// Если нейросеть не подходит человеку, возвращает:
    /// - [`NeuralNetError::InstallableOnlyForAnimal`] для специализированной сети,
    ///   которая уже токсична для человека, но работает у животного;
    /// - [`NeuralNetError::NotInstallableForHuman`] во всех остальных случаях
    ///   (исчерпано число установок).
    ///
    /// При ошибке состояние персонажа не меняется. Изученные базы знаний
    /// (`brain.known_bases`) никогда не затрагиваются. Прежняя нейросеть
    /// отбрасывается: операции снятия появятся позже.
    pub fn install_neural_net(&mut self, mut nn: NeuralNet) -> Result<(), NeuralNetError> {
        if !nn.is_installable_for_human() {
            return Err(
                if nn.kind == NeuralNetKind::Specialized && nn.is_installable_for_animal() {
                    NeuralNetError::InstallableOnlyForAnimal
                } else {
                    NeuralNetError::NotInstallableForHuman
                },
            );
        }
        nn.install()?;
        self.installed_neural_net = Some(nn);
        Ok(())
    }

    /// Добавляет имплант персонажу.
    pub fn add_implant(&mut self, implant: Implant) {
        self.implants.push(implant);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kb(id: u64, required_iq: u32, required_memory_pb: u32) -> KnowledgeBase {
        KnowledgeBase {
            id: KnowledgeBaseId(id),
            required_iq,
            required_memory_pb,
            study_time_ticks: 10,
        }
    }

    fn implant(iq_bonus: i32, memory_bonus_pb: i32) -> Implant {
        Implant {
            iq_bonus,
            memory_bonus_pb,
        }
    }

    #[test]
    fn specialized_is_installable_for_human_before_second_install() {
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1));
        assert_eq!(nn.install_count, 0);
        assert!(nn.is_installable_for_human());
        assert!(!nn.is_installable_for_animal());

        nn.install().unwrap();
        assert_eq!(nn.install_count, 1);
        assert!(nn.is_installable_for_human());
        assert!(!nn.is_installable_for_animal());
    }

    #[test]
    fn specialized_after_two_installs_is_for_animal_only() {
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1));
        nn.install().unwrap();
        nn.install().unwrap();
        assert_eq!(nn.install_count, 2);
        assert!(!nn.is_installable_for_human());
        assert!(nn.is_installable_for_animal());
    }

    #[test]
    fn individual_after_one_install_is_not_installable_for_human() {
        let mut nn = NeuralNet::new_individual(NeuralNetId(2));
        assert!(nn.is_installable_for_human());
        nn.install().unwrap();
        assert_eq!(nn.install_count, 1);
        assert!(!nn.is_installable_for_human());
        assert!(nn.is_installable_for_animal());
    }

    #[test]
    fn basic_is_always_installable_for_human() {
        let mut nn = NeuralNet::new_basic(NeuralNetId(3));
        assert_eq!(nn.install_count, 1);
        assert!(nn.is_installable_for_human());
        assert!(!nn.is_installable_for_animal());
        for _ in 0..5 {
            nn.install().unwrap();
            assert!(nn.is_installable_for_human());
        }
    }

    #[test]
    fn install_at_u8_max_returns_overflow() {
        let mut nn = NeuralNet::new_basic(NeuralNetId(1));
        nn.install_count = u8::MAX;
        assert_eq!(nn.install(), Err(NeuralNetError::Overflow));
        assert_eq!(nn.install_count, u8::MAX);
    }

    #[test]
    fn low_iq_cannot_study() {
        let mut ch = Character::new(Brain::new(80, 100));
        let base = kb(1, 100, 10);
        assert!(!ch.can_study(&base));
        assert_eq!(ch.study(&base), Err(StudyError::InsufficientIq));
        assert!(!ch.brain.has_base(base.id));
    }

    #[test]
    fn implant_bonus_makes_study_possible() {
        let mut ch = Character::new(Brain::new(80, 100));
        let base = kb(1, 100, 10);
        ch.add_implant(implant(30, 0));
        assert_eq!(ch.effective_iq(), 110);
        assert!(ch.can_study(&base));
        assert_eq!(ch.study(&base), Ok(()));
        assert!(ch.brain.has_base(base.id));
    }

    #[test]
    fn low_memory_cannot_study() {
        let mut ch = Character::new(Brain::new(100, 10));
        let base = kb(1, 100, 20);
        assert_eq!(ch.study(&base), Err(StudyError::InsufficientMemory));
        assert!(!ch.brain.has_base(base.id));
    }

    #[test]
    fn study_is_idempotent() {
        let mut ch = Character::new(Brain::new(100, 100));
        let base = kb(1, 50, 50);
        assert_eq!(ch.study(&base), Ok(()));
        assert_eq!(ch.study(&base), Ok(()));
        assert_eq!(ch.brain.known_bases.len(), 1);
    }

    #[test]
    fn known_base_stays_studied_even_if_requirements_drop() {
        let mut ch = Character::new(Brain::new(100, 100));
        let base = kb(1, 100, 100);
        ch.study(&base).unwrap();
        // Негативный имплант опускает эффективный IQ ниже требований.
        ch.add_implant(implant(-50, 0));
        assert!(!ch.can_study(&base));
        assert_eq!(ch.study(&base), Ok(()));
        assert!(ch.brain.has_base(base.id));
    }

    #[test]
    fn replacing_neural_net_keeps_known_bases() {
        let mut ch = Character::new(Brain::new(150, 150));
        let (first, second) = (kb(1, 100, 100), kb(2, 120, 120));
        ch.study(&first).unwrap();
        ch.study(&second).unwrap();
        let before = ch.brain.known_bases.clone();
        assert_eq!(before.len(), 2);

        ch.install_neural_net(NeuralNet::new_specialized(NeuralNetId(10)))
            .unwrap();
        assert_eq!(
            ch.installed_neural_net.as_ref().map(|nn| nn.id),
            Some(NeuralNetId(10))
        );

        // Заменяем нейросеть: старая отбрасывается, мозг не меняется.
        ch.install_neural_net(NeuralNet::new_individual(NeuralNetId(11)))
            .unwrap();
        assert_eq!(
            ch.installed_neural_net.as_ref().map(|nn| nn.id),
            Some(NeuralNetId(11))
        );

        assert_eq!(ch.brain.known_bases, before);
        assert!(ch.brain.has_base(first.id));
        assert!(ch.brain.has_base(second.id));
        assert_eq!(ch.brain.known_bases.len(), 2);
    }

    #[test]
    fn install_spent_individual_is_not_installable_for_human() {
        let mut ch = Character::new(Brain::new(100, 100));
        let mut nn = NeuralNet::new_individual(NeuralNetId(1));
        nn.install().unwrap();
        assert_eq!(
            ch.install_neural_net(nn),
            Err(NeuralNetError::NotInstallableForHuman)
        );
        assert!(ch.installed_neural_net.is_none());
    }

    #[test]
    fn install_toxic_specialized_is_animal_only_error() {
        let mut ch = Character::new(Brain::new(100, 100));
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1));
        nn.install().unwrap();
        nn.install().unwrap();
        assert_eq!(
            ch.install_neural_net(nn),
            Err(NeuralNetError::InstallableOnlyForAnimal)
        );
        assert!(ch.installed_neural_net.is_none());
    }

    #[test]
    fn install_over_exhausted_specialized_is_not_installable_for_human() {
        let mut ch = Character::new(Brain::new(100, 100));
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1));
        nn.install_count = 3;
        assert_eq!(
            ch.install_neural_net(nn),
            Err(NeuralNetError::NotInstallableForHuman)
        );
    }

    #[test]
    fn failed_install_keeps_previous_neural_net() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.install_neural_net(NeuralNet::new_basic(NeuralNetId(1)))
            .unwrap();
        let mut spent = NeuralNet::new_individual(NeuralNetId(2));
        spent.install().unwrap();
        assert!(ch.install_neural_net(spent).is_err());
        assert_eq!(
            ch.installed_neural_net.as_ref().map(|nn| nn.id),
            Some(NeuralNetId(1))
        );
    }

    #[test]
    fn install_neural_net_increments_install_count() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.install_neural_net(NeuralNet::new_specialized(NeuralNetId(1)))
            .unwrap();
        assert_eq!(
            ch.installed_neural_net.as_ref().map(|nn| nn.install_count),
            Some(1)
        );
    }

    #[test]
    fn install_neural_net_propagates_overflow() {
        let mut ch = Character::new(Brain::new(100, 100));
        let mut nn = NeuralNet::new_basic(NeuralNetId(1));
        nn.install_count = u8::MAX;
        assert_eq!(ch.install_neural_net(nn), Err(NeuralNetError::Overflow));
        assert!(ch.installed_neural_net.is_none());
    }

    #[test]
    fn effective_iq_sums_several_implants() {
        let mut ch = Character::new(Brain::new(100, 50));
        ch.add_implant(implant(10, 5));
        ch.add_implant(implant(20, 15));
        ch.add_implant(implant(-5, -10));
        assert_eq!(ch.effective_iq(), 125);
        assert_eq!(ch.effective_memory_pb(), 60);
    }

    #[test]
    fn negative_implants_clamp_to_zero() {
        let mut ch = Character::new(Brain::new(10, 10));
        ch.add_implant(implant(-1000, -1000));
        assert_eq!(ch.effective_iq(), 0);
        assert_eq!(ch.effective_memory_pb(), 0);
    }

    #[test]
    fn effective_values_clamp_at_u32_max() {
        let mut ch = Character::new(Brain::new(u32::MAX, u32::MAX));
        ch.add_implant(implant(i32::MAX, i32::MAX));
        assert_eq!(ch.effective_iq(), u32::MAX);
        assert_eq!(ch.effective_memory_pb(), u32::MAX);
    }

    #[test]
    fn new_character_is_empty() {
        let ch = Character::new(Brain::new(100, 50));
        assert!(ch.installed_neural_net.is_none());
        assert!(ch.implants.is_empty());
        assert!(ch.reputation.is_empty());
        assert!(ch.brain.known_bases.is_empty());
    }

    #[test]
    fn error_display_messages() {
        assert_eq!(
            NeuralNetError::NotInstallableForHuman.to_string(),
            "neural net is not installable for human"
        );
        assert_eq!(
            NeuralNetError::InstallableOnlyForAnimal.to_string(),
            "neural net is installable only for animal"
        );
        assert_eq!(
            NeuralNetError::Overflow.to_string(),
            "neural net install counter overflow"
        );
        assert_eq!(
            StudyError::InsufficientIq.to_string(),
            "character iq is too low to study this knowledge base"
        );
        assert_eq!(
            StudyError::InsufficientMemory.to_string(),
            "character memory is too low to study this knowledge base"
        );
    }

    #[test]
    fn known_bases_iterate_in_id_order() {
        let mut ch = Character::new(Brain::new(100, 100));
        for id in [30, 10, 20] {
            ch.study(&kb(id, 1, 1)).unwrap();
        }
        let ids: Vec<u64> = ch.brain.known_bases.iter().map(|b| b.0).collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }
}
