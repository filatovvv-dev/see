//! Персонаж: мозг, нейросеть, импланты и базы знаний.
//!
//! Развитие персонажа трёхслойное:
//! - **мозг** хранит изученные базы знаний навсегда;
//! - **нейросеть** — заменяемый аппарат, смена которого не затрагивает мозг;
//! - **импланты** компенсируют низкий IQ или малый объём памяти.
//!
//! Поверх этого модуль хранит социальное состояние разумного: гражданство
//! (с отметкой о первом гражданстве на нейросети), правовой статус
//! (свободный или раб) и отлучение от **новых** баз знаний
//! (см. `docs/lore-bible.md`, разделы 2.5, 3.10, 5 и 6).
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

/// Гражданство разумного.
///
/// Отметка о первом гражданстве хранится на нейросети навсегда: проверка
/// отлучения идёт по ней (см. `docs/lore-bible.md`, раздел 5).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Citizenship(pub String);

/// Идентификатор государства.
///
/// Это не [`FactionId`]: государства и фракции — разные сущности
/// (см. `docs/lore-bible.md`, разделы 1 и 3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StateId(pub String);

/// Производитель нейросетей и баз знаний («Нейро Инк.», «Квазар» и т. д.).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Manufacturer(pub String);

/// Государства, где рабство — источник и раб не может стать свободным.
pub const SLAVERY_SOURCE: &[&str] = &["Коткор"];

/// Государства, где рабство легально, но срок не больше 30 лет.
pub const SLAVERY_LEGAL_LIMITED: &[&str] = &["Соединённые Второе и Седьмое Королевства", "Лейтуар"];

/// Государства, где рабство легально, но запрещено обращать в рабов.
pub const SLAVERY_LEGAL_NO_ENSLAVE: &[&str] = &[
    "Русское Царство",
    "Свободные Миры",
    "Сокобри",
    "Мерказ",
    "Таррпа",
    "Траска",
];

/// Правовой статус разумного.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LegalStatus {
    /// Свободный.
    Free,
    /// Раб. Срок и ограничения — по государству (см. `docs/lore-bible.md`, раздел 6).
    Slave {
        /// Государство, в котором разумный является рабом.
        state: StateId,
        /// Игровой год, до которого длится рабство. `None` — бессрочно (Коткор).
        until_year: Option<u32>,
    },
}

/// Запись об отлучении от новых баз знаний.
///
/// Отлучение «Союзной корпорации» не автоматическое: оно возможно, только
/// если её корабли сами атакуют «Нейро Инк.» или «Квазар». Эта логика
/// относится к уровню фракций и контрактов и здесь не реализована.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Excommunication {
    /// Компания, которая отлучила («Нейро Инк.» или «Квазар»).
    pub by: Manufacturer,
    /// Игровой год отлучения. Базы, выпущенные после него, недоступны.
    pub since_year: u32,
    /// Причина (для лора и интерфейса).
    pub reason: String,
}

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
    /// IQ разумного ниже минимального порога установки нейросети.
    BelowMinIq,
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
            NeuralNetError::BelowMinIq => {
                write!(f, "character iq is below the neural net minimum iq")
            }
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
    /// Персонаж отлучён, а база выпущена после отлучения.
    Excommunicated,
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
            StudyError::Excommunicated => {
                write!(
                    f,
                    "excommunicated: knowledge base was released after the excommunication"
                )
            }
        }
    }
}

impl std::error::Error for StudyError {}

/// Ошибка смены гражданства.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitizenshipError {
    /// Смена гражданства запрещена государством.
    ChangeForbidden,
    /// Первое гражданство уже зафиксировано на нейросети и не может быть изменено.
    FirstCitizenshipLocked,
}

impl fmt::Display for CitizenshipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CitizenshipError::ChangeForbidden => {
                write!(f, "citizenship change is forbidden by the state")
            }
            CitizenshipError::FirstCitizenshipLocked => {
                write!(
                    f,
                    "first citizenship is already recorded and cannot be changed"
                )
            }
        }
    }
}

impl std::error::Error for CitizenshipError {}

/// Ошибка смены правового статуса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegalStatusError {
    /// В этом государстве раб не может стать свободным.
    CannotBeFreed,
    /// Разумный не является рабом.
    NotSlave,
    /// Срок рабства ещё не истёк (или рабство бессрочно).
    SlaveryNotExpired,
}

impl fmt::Display for LegalStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LegalStatusError::CannotBeFreed => {
                write!(f, "a slave cannot be freed in this state")
            }
            LegalStatusError::NotSlave => write!(f, "character is not a slave"),
            LegalStatusError::SlaveryNotExpired => write!(f, "slavery term has not expired"),
        }
    }
}

impl std::error::Error for LegalStatusError {}

/// Ошибка операции с отлучением.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcommunicationError {
    /// Разумный уже отлучён.
    AlreadyExcommunicated,
    /// Разумный не отлучён.
    NotExcommunicated,
}

impl fmt::Display for ExcommunicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExcommunicationError::AlreadyExcommunicated => write!(f, "already excommunicated"),
            ExcommunicationError::NotExcommunicated => write!(f, "not excommunicated"),
        }
    }
}

impl std::error::Error for ExcommunicationError {}

/// Нейросеть: заменяемый аппарат, дающий бонусы носителю.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuralNet {
    /// Идентификатор нейросети.
    pub id: NeuralNetId,
    /// Вид нейросети.
    pub kind: NeuralNetKind,
    /// Сколько раз нейросеть уже устанавливалась носителям.
    pub install_count: u8,
    /// Минимальный IQ разумного для установки. Ниже порога сеть не устанавливается.
    pub min_iq: u32,
    /// Отметка о первом гражданстве владельца. Записывается один раз и
    /// остаётся навсегда (в том числе при смене нейросети и гражданства).
    pub first_citizenship: Option<Citizenship>,
}

impl NeuralNet {
    /// Создаёт базовую нейросеть (`install_count = 1`).
    pub fn new_basic(id: NeuralNetId) -> Self {
        Self {
            id,
            kind: NeuralNetKind::Basic,
            install_count: 1,
            min_iq: 0,
            first_citizenship: None,
        }
    }

    /// Создаёт специализированную нейросеть (`install_count = 0`).
    pub fn new_specialized(id: NeuralNetId) -> Self {
        Self {
            id,
            kind: NeuralNetKind::Specialized,
            install_count: 0,
            min_iq: 0,
            first_citizenship: None,
        }
    }

    /// Создаёт индивидуальную нейросеть (`install_count = 0`).
    pub fn new_individual(id: NeuralNetId) -> Self {
        Self {
            id,
            kind: NeuralNetKind::Individual,
            install_count: 0,
            min_iq: 0,
            first_citizenship: None,
        }
    }

    /// Возвращает нейросеть с заданным минимальным порогом IQ для установки.
    pub fn with_min_iq(mut self, min_iq: u32) -> Self {
        self.min_iq = min_iq;
        self
    }

    /// Возвращает `true`, если IQ разумного не ниже порога `min_iq`.
    pub fn can_install(&self, character_iq: u32) -> bool {
        character_iq >= self.min_iq
    }

    /// Записывает первое гражданство владельца, если оно ещё не записано.
    ///
    /// Повторная запись того же гражданства допустима. Попытка записать
    /// другое возвращает [`CitizenshipError::FirstCitizenshipLocked`].
    pub fn record_first_citizenship(
        &mut self,
        citizenship: Citizenship,
    ) -> Result<(), CitizenshipError> {
        match &self.first_citizenship {
            None => {
                self.first_citizenship = Some(citizenship);
                Ok(())
            }
            Some(existing) if *existing == citizenship => Ok(()),
            Some(_) => Err(CitizenshipError::FirstCitizenshipLocked),
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

    /// Устанавливает нейросеть разумному с IQ `character_iq` и гражданством
    /// `citizenship`.
    ///
    /// Сначала проверяет порог `min_iq` ([`NeuralNetError::BelowMinIq`]), затем
    /// увеличивает счётчик установок (см. [`NeuralNet::install`]) и записывает
    /// первое гражданство, если его ещё нет. При ошибке нейросеть не меняется.
    pub fn install_for(
        &mut self,
        character_iq: u32,
        citizenship: Citizenship,
    ) -> Result<(), NeuralNetError> {
        if !self.can_install(character_iq) {
            return Err(NeuralNetError::BelowMinIq);
        }
        self.install()?;
        if self.first_citizenship.is_none() {
            self.first_citizenship = Some(citizenship);
        }
        Ok(())
    }
}

/// Имплант: дополнительное железо с узкими бонусами.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Implant {
    /// Бонус к IQ (может быть отрицательным).
    pub iq_bonus: i32,
    /// Бонус к объёму памяти в петабайтах (может быть отрицательным).
    pub memory_bonus_pb: i64,
}

/// База знаний, которую можно изучить.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeBase {
    /// Идентификатор базы знаний.
    pub id: KnowledgeBaseId,
    /// Минимальный эффективный IQ для изучения.
    pub required_iq: u32,
    /// Минимальный эффективный объём памяти (в петабайтах) для изучения.
    pub required_memory_pb: u64,
    /// Время изучения в тиках симуляции.
    pub study_time_ticks: u64,
    /// Игровой год выпуска базы. Нужен для проверки отлучения: база,
    /// выпущенная после отлучения, отлучённому недоступна.
    pub release_year: u32,
}

/// Мозг персонажа: базовые параметры и изученные базы знаний.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brain {
    /// Базовый IQ.
    pub iq: u32,
    /// Базовый объём памяти в петабайтах.
    pub memory_capacity_pb: u64,
    /// Изученные базы знаний. Хранятся в мозге навсегда.
    pub known_bases: BTreeSet<KnowledgeBaseId>,
}

impl Brain {
    /// Создаёт мозг без изученных баз знаний.
    pub fn new(iq: u32, memory_pb: u64) -> Self {
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

/// Приводит знаковую сумму к `u64`, обрезая значение до диапазона `0..=u64::MAX`.
fn clamp_to_u64(value: i128) -> u64 {
    u64::try_from(value.clamp(0, i128::from(u64::MAX))).unwrap_or(u64::MAX)
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
    /// Текущее гражданство. `None` — у «дикого» до выбора гражданства.
    pub citizenship: Option<Citizenship>,
    /// Правовой статус.
    pub legal_status: LegalStatus,
    /// Отлучение от новых баз знаний, если есть.
    pub excommunication: Option<Excommunication>,
    /// Флаг: разумный был отлучён, но потом «освобождён» (дипломатия или
    /// компенсация). Сбрасывается при новом отлучении.
    pub released_after_excommunication: bool,
}

impl Character {
    /// Создаёт свободного персонажа без нейросети, имплантов, репутации,
    /// гражданства и отлучения.
    pub fn new(brain: Brain) -> Self {
        Self {
            brain,
            installed_neural_net: None,
            implants: Vec::new(),
            reputation: BTreeMap::new(),
            citizenship: None,
            legal_status: LegalStatus::Free,
            excommunication: None,
            released_after_excommunication: false,
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
    /// в диапазоне `0..=u64::MAX`.
    pub fn effective_memory_pb(&self) -> u64 {
        let total = self
            .implants
            .iter()
            .fold(i128::from(self.brain.memory_capacity_pb), |acc, implant| {
                acc.saturating_add(i128::from(implant.memory_bonus_pb))
            });
        clamp_to_u64(total)
    }

    /// Проверяет, можно ли изучить базу `kb`, не меняя состояние.
    ///
    /// Порядок проверок: IQ ([`StudyError::InsufficientIq`]), память
    /// ([`StudyError::InsufficientMemory`]), отлучение
    /// ([`StudyError::Excommunicated`]). Отлучение блокирует только **новые**
    /// базы: выпущенные не позже года отлучения (`release_year <= since_year`)
    /// изучаются как обычно.
    pub fn check_study(&self, kb: &KnowledgeBase) -> Result<(), StudyError> {
        if self.effective_iq() < kb.required_iq {
            return Err(StudyError::InsufficientIq);
        }
        if self.effective_memory_pb() < kb.required_memory_pb {
            return Err(StudyError::InsufficientMemory);
        }
        if let Some(ex) = &self.excommunication
            && kb.release_year > ex.since_year
        {
            return Err(StudyError::Excommunicated);
        }
        Ok(())
    }

    /// Возвращает `true`, если базу `kb` можно изучить (см. [`Character::check_study`]).
    pub fn can_study(&self, kb: &KnowledgeBase) -> bool {
        self.check_study(kb).is_ok()
    }

    /// Изучает базу знаний `kb`.
    ///
    /// Если база уже изучена, операция идемпотентна и возвращает `Ok(())`:
    /// знания в мозге не теряются, даже если требования больше не выполняются.
    /// Иначе проверки идут в порядке [`Character::check_study`]: IQ, память,
    /// отлучение.
    pub fn study(&mut self, kb: &KnowledgeBase) -> Result<(), StudyError> {
        if self.brain.has_base(kb.id) {
            return Ok(());
        }
        self.check_study(kb)?;
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
    /// Если эффективный IQ ниже `min_iq` нейросети, возвращает
    /// [`NeuralNetError::BelowMinIq`] (проверяется после применимости человеку).
    ///
    /// Отметка о первом гражданстве сохраняется навсегда: если у новой сети её
    /// нет, она берётся у прежней сети, а при её отсутствии — из текущего
    /// гражданства персонажа (если оно есть).
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
        if !nn.can_install(self.effective_iq()) {
            return Err(NeuralNetError::BelowMinIq);
        }
        nn.install()?;
        if nn.first_citizenship.is_none() {
            nn.first_citizenship = self
                .installed_neural_net
                .as_ref()
                .and_then(|old| old.first_citizenship.clone())
                .or_else(|| self.citizenship.clone());
        }
        self.installed_neural_net = Some(nn);
        Ok(())
    }

    /// Возвращает отметку о первом гражданстве с установленной нейросети.
    pub fn first_citizenship(&self) -> Option<&Citizenship> {
        self.installed_neural_net
            .as_ref()
            .and_then(|nn| nn.first_citizenship.as_ref())
    }

    /// Меняет текущее гражданство разумного.
    ///
    /// Отметка о первом гражданстве на нейросети не меняется. Согласие
    /// государства проверяет вызывающий код (оркестратор): если оно не
    /// получено, нужно вернуть [`CitizenshipError::ChangeForbidden`] до вызова.
    pub fn change_citizenship(&mut self, new: Citizenship) -> Result<(), CitizenshipError> {
        self.citizenship = Some(new);
        Ok(())
    }

    /// Освобождает раба по законам государства `state`.
    ///
    /// В государствах из [`SLAVERY_SOURCE`] (Коткор) раб не может стать
    /// свободным никогда: [`LegalStatusError::CannotBeFreed`]. В остальных
    /// статус становится [`LegalStatus::Free`]; для уже свободного разумного
    /// операция ничего не меняет.
    pub fn free(&mut self, state: &StateId) -> Result<(), LegalStatusError> {
        if SLAVERY_SOURCE.contains(&state.0.as_str()) {
            return Err(LegalStatusError::CannotBeFreed);
        }
        self.legal_status = LegalStatus::Free;
        Ok(())
    }

    /// Проверяет, истёк ли срок рабства в год `current_year`.
    ///
    /// Если срок истёк (`current_year >= until_year`), разумный становится
    /// свободным. Для свободного — [`LegalStatusError::NotSlave`], для раба
    /// с неистёкшим или бессрочным рабством — [`LegalStatusError::SlaveryNotExpired`].
    pub fn check_slavery_expired(&mut self, current_year: u32) -> Result<(), LegalStatusError> {
        match &self.legal_status {
            LegalStatus::Free => Err(LegalStatusError::NotSlave),
            LegalStatus::Slave {
                until_year: Some(until),
                ..
            } if current_year >= *until => {
                self.legal_status = LegalStatus::Free;
                Ok(())
            }
            LegalStatus::Slave { .. } => Err(LegalStatusError::SlaveryNotExpired),
        }
    }

    /// Отлучает разумного от новых баз знаний компанией `by` с года `since_year`.
    ///
    /// Возвращает [`ExcommunicationError::AlreadyExcommunicated`], если
    /// отлучение уже действует. Сбрасывает флаг `released_after_excommunication`.
    pub fn excommunicate(
        &mut self,
        by: Manufacturer,
        since_year: u32,
        reason: String,
    ) -> Result<(), ExcommunicationError> {
        if self.excommunication.is_some() {
            return Err(ExcommunicationError::AlreadyExcommunicated);
        }
        self.excommunication = Some(Excommunication {
            by,
            since_year,
            reason,
        });
        self.released_after_excommunication = false;
        Ok(())
    }

    /// Снимает отлучение (дипломатия или компенсация).
    ///
    /// Новые базы снова доступны, `released_after_excommunication` становится
    /// `true`. Если разумный не отлучён — [`ExcommunicationError::NotExcommunicated`].
    pub fn release_from_excommunication(&mut self) -> Result<(), ExcommunicationError> {
        if self.excommunication.take().is_none() {
            return Err(ExcommunicationError::NotExcommunicated);
        }
        self.released_after_excommunication = true;
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

    fn kb(id: u64, required_iq: u32, required_memory_pb: u64) -> KnowledgeBase {
        KnowledgeBase {
            id: KnowledgeBaseId(id),
            required_iq,
            required_memory_pb,
            study_time_ticks: 10,
            release_year: 0,
        }
    }

    fn implant(iq_bonus: i32, memory_bonus_pb: i64) -> Implant {
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
        let mut ch = Character::new(Brain::new(u32::MAX, u64::MAX));
        ch.add_implant(implant(i32::MAX, i64::MAX));
        assert_eq!(ch.effective_iq(), u32::MAX);
        assert_eq!(ch.effective_memory_pb(), u64::MAX);
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

    // ---- Гражданство, правовой статус, отлучение, память u64 ----

    fn cit(name: &str) -> Citizenship {
        Citizenship(name.to_string())
    }

    fn state(name: &str) -> StateId {
        StateId(name.to_string())
    }

    fn maker(name: &str) -> Manufacturer {
        Manufacturer(name.to_string())
    }

    fn kb_released(id: u64, release_year: u32) -> KnowledgeBase {
        KnowledgeBase {
            release_year,
            ..kb(id, 0, 0)
        }
    }

    fn slave_of(name: &str, until_year: Option<u32>) -> Character {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.legal_status = LegalStatus::Slave {
            state: state(name),
            until_year,
        };
        ch
    }

    // Гражданство

    #[test]
    fn first_citizenship_set_on_install() {
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1));
        assert_eq!(nn.first_citizenship, None);
        nn.install_for(100, cit("Мерказ")).unwrap();
        assert_eq!(nn.first_citizenship, Some(cit("Мерказ")));
        assert_eq!(nn.install_count, 1);

        let mut ch = Character::new(Brain::new(100, 100));
        ch.citizenship = Some(cit("Сокобри"));
        ch.install_neural_net(NeuralNet::new_basic(NeuralNetId(2)))
            .unwrap();
        assert_eq!(ch.first_citizenship(), Some(&cit("Сокобри")));
    }

    #[test]
    fn change_citizenship_updates_current_but_not_first() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.citizenship = Some(cit("Мерказ"));
        ch.install_neural_net(NeuralNet::new_basic(NeuralNetId(1)))
            .unwrap();

        ch.change_citizenship(cit("Лейтуар")).unwrap();
        assert_eq!(ch.citizenship, Some(cit("Лейтуар")));
        assert_eq!(ch.first_citizenship(), Some(&cit("Мерказ")));
    }

    #[test]
    fn first_citizenship_locked() {
        let mut nn = NeuralNet::new_basic(NeuralNetId(1));
        nn.record_first_citizenship(cit("Мерказ")).unwrap();
        assert_eq!(
            nn.record_first_citizenship(cit("Лейтуар")),
            Err(CitizenshipError::FirstCitizenshipLocked)
        );
        assert_eq!(nn.first_citizenship, Some(cit("Мерказ")));
        assert_eq!(nn.record_first_citizenship(cit("Мерказ")), Ok(()));
    }

    #[test]
    fn install_for_keeps_existing_first_citizenship() {
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1));
        nn.record_first_citizenship(cit("Мерказ")).unwrap();
        nn.install_for(100, cit("Лейтуар")).unwrap();
        assert_eq!(nn.first_citizenship, Some(cit("Мерказ")));
    }

    #[test]
    fn first_citizenship_survives_neural_net_replacement() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.citizenship = Some(cit("Мерказ"));
        ch.install_neural_net(NeuralNet::new_basic(NeuralNetId(1)))
            .unwrap();
        ch.change_citizenship(cit("Лейтуар")).unwrap();

        ch.install_neural_net(NeuralNet::new_specialized(NeuralNetId(2)))
            .unwrap();
        assert_eq!(ch.first_citizenship(), Some(&cit("Мерказ")));
        assert_eq!(ch.citizenship, Some(cit("Лейтуар")));
    }

    #[test]
    fn wild_character_has_no_first_citizenship_without_citizenship() {
        let mut ch = Character::new(Brain::new(100, 100));
        assert_eq!(ch.citizenship, None);
        ch.install_neural_net(NeuralNet::new_basic(NeuralNetId(1)))
            .unwrap();
        assert_eq!(ch.first_citizenship(), None);
    }

    // Правовой статус

    #[test]
    fn free_in_kotkor_impossible() {
        let mut ch = slave_of("Коткор", None);
        assert_eq!(
            ch.free(&state("Коткор")),
            Err(LegalStatusError::CannotBeFreed)
        );
        assert!(matches!(ch.legal_status, LegalStatus::Slave { .. }));
    }

    #[test]
    fn free_in_limited_state_ok() {
        let mut ch = slave_of("Соединённые Второе и Седьмое Королевства", Some(30));
        assert_eq!(
            ch.free(&state("Соединённые Второе и Седьмое Королевства")),
            Ok(())
        );
        assert_eq!(ch.legal_status, LegalStatus::Free);
    }

    #[test]
    fn slavery_expires_after_term() {
        let mut ch = slave_of("Лейтуар", Some(30));
        assert_eq!(ch.check_slavery_expired(30), Ok(()));
        assert_eq!(ch.legal_status, LegalStatus::Free);
    }

    #[test]
    fn slavery_not_expired_before_term() {
        let mut ch = slave_of("Лейтуар", Some(30));
        assert_eq!(
            ch.check_slavery_expired(10),
            Err(LegalStatusError::SlaveryNotExpired)
        );
        assert_eq!(
            ch.legal_status,
            LegalStatus::Slave {
                state: state("Лейтуар"),
                until_year: Some(30),
            }
        );
    }

    #[test]
    fn perpetual_slavery_never_expires() {
        let mut ch = slave_of("Коткор", None);
        assert_eq!(
            ch.check_slavery_expired(u32::MAX),
            Err(LegalStatusError::SlaveryNotExpired)
        );
    }

    #[test]
    fn free_character_has_no_slavery_to_expire() {
        let mut ch = Character::new(Brain::new(100, 100));
        assert_eq!(
            ch.check_slavery_expired(100),
            Err(LegalStatusError::NotSlave)
        );
    }

    #[test]
    fn slavery_policy_constants_match_lore() {
        assert_eq!(SLAVERY_SOURCE, &["Коткор"]);
        assert!(SLAVERY_LEGAL_LIMITED.contains(&"Лейтуар"));
        assert!(SLAVERY_LEGAL_NO_ENSLAVE.contains(&"Таррпа"));
        assert!(!SLAVERY_LEGAL_NO_ENSLAVE.contains(&"Коткор"));
    }

    // Отлучение

    #[test]
    fn excommunicate_sets_flag() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.excommunicate(maker("Нейро Инк."), 50, "атака на конвой".to_string())
            .unwrap();
        let ex = ch.excommunication.as_ref().unwrap();
        assert_eq!(ex.by, maker("Нейро Инк."));
        assert_eq!(ex.since_year, 50);
        assert_eq!(ex.reason, "атака на конвой");
        assert!(!ch.released_after_excommunication);
    }

    #[test]
    fn already_excommunicated() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.excommunicate(maker("Нейро Инк."), 50, String::new())
            .unwrap();
        assert_eq!(
            ch.excommunicate(maker("Квазар"), 60, String::new()),
            Err(ExcommunicationError::AlreadyExcommunicated)
        );
        assert_eq!(ch.excommunication.as_ref().unwrap().since_year, 50);
    }

    #[test]
    fn cannot_study_new_base_after_excommunication() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.excommunicate(maker("Нейро Инк."), 50, String::new())
            .unwrap();
        let new_base = kb_released(1, 51);
        assert!(!ch.can_study(&new_base));
        assert_eq!(ch.study(&new_base), Err(StudyError::Excommunicated));
        assert!(!ch.brain.has_base(new_base.id));
    }

    #[test]
    fn can_study_old_base_after_excommunication() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.excommunicate(maker("Нейро Инк."), 50, String::new())
            .unwrap();
        let older = kb_released(1, 10);
        let same_year = kb_released(2, 50);
        assert_eq!(ch.study(&older), Ok(()));
        assert_eq!(ch.study(&same_year), Ok(()));
        assert!(ch.brain.has_base(older.id));
        assert!(ch.brain.has_base(same_year.id));
    }

    #[test]
    fn release_from_excommunication_allows_new_bases() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.excommunicate(maker("Нейро Инк."), 50, String::new())
            .unwrap();
        let new_base = kb_released(1, 51);
        assert_eq!(ch.study(&new_base), Err(StudyError::Excommunicated));

        assert_eq!(ch.release_from_excommunication(), Ok(()));
        assert!(ch.released_after_excommunication);
        assert_eq!(ch.excommunication, None);
        assert_eq!(ch.study(&new_base), Ok(()));
    }

    #[test]
    fn release_without_excommunication_is_error() {
        let mut ch = Character::new(Brain::new(100, 100));
        assert_eq!(
            ch.release_from_excommunication(),
            Err(ExcommunicationError::NotExcommunicated)
        );
        assert!(!ch.released_after_excommunication);
    }

    #[test]
    fn new_excommunication_after_release_resets_flag() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.excommunicate(maker("Нейро Инк."), 50, String::new())
            .unwrap();
        ch.release_from_excommunication().unwrap();
        ch.excommunicate(maker("Квазар"), 70, String::new())
            .unwrap();
        assert!(!ch.released_after_excommunication);
        assert_eq!(
            ch.study(&kb_released(1, 71)),
            Err(StudyError::Excommunicated)
        );
    }

    #[test]
    fn excommunication_keeps_known_bases() {
        let mut ch = Character::new(Brain::new(100, 100));
        let base = kb_released(1, 10);
        ch.study(&base).unwrap();
        ch.excommunicate(maker("Нейро Инк."), 5, String::new())
            .unwrap();
        assert!(ch.brain.has_base(base.id));
        assert_eq!(ch.study(&base), Ok(()));
    }

    #[test]
    fn iq_and_memory_are_checked_before_excommunication() {
        let mut ch = Character::new(Brain::new(80, 100));
        ch.excommunicate(maker("Нейро Инк."), 5, String::new())
            .unwrap();
        let hard = KnowledgeBase {
            release_year: 10,
            ..kb(1, 100, 10)
        };
        assert_eq!(ch.study(&hard), Err(StudyError::InsufficientIq));
        let heavy = KnowledgeBase {
            release_year: 10,
            ..kb(2, 10, 1_000)
        };
        assert_eq!(ch.study(&heavy), Err(StudyError::InsufficientMemory));
    }

    // memory_pb: u64

    #[test]
    fn memory_pb_is_u64() {
        // 1 ЙБ = 1_000_000_000 ПБ; пять таких имплантов не помещаются в u32.
        let mut ch = Character::new(Brain::new(100, 10));
        for _ in 0..5 {
            ch.add_implant(implant(0, 1_000_000_000));
        }
        assert_eq!(ch.effective_memory_pb(), 5_000_000_010);
        let heavy = kb(1, 10, 5_000_000_000);
        assert_eq!(ch.study(&heavy), Ok(()));
    }

    #[test]
    fn memory_pb_overflow_was_possible_with_u32() {
        // Тест-документация: при u32 пять имплантов «+1 ЙБ» упёрлись бы в потолок.
        let total: u64 = 5 * 1_000_000_000;
        assert!(total > u64::from(u32::MAX));
        let mut ch = Character::new(Brain::new(100, 0));
        for _ in 0..5 {
            ch.add_implant(implant(0, 1_000_000_000));
        }
        assert_eq!(ch.effective_memory_pb(), total);
    }

    // Совместимость

    #[test]
    fn existing_install_still_works() {
        let mut ch = Character::new(Brain::new(100, 100));
        ch.install_neural_net(NeuralNet::new_basic(NeuralNetId(1)))
            .unwrap();
        assert_eq!(ch.installed_neural_net.as_ref().unwrap().install_count, 2);
        assert_eq!(ch.first_citizenship(), None);
    }

    #[test]
    fn min_iq_blocks_install_below_threshold() {
        let mut ch = Character::new(Brain::new(100, 100));
        let old = NeuralNet::new_basic(NeuralNetId(1));
        ch.install_neural_net(old.clone()).unwrap();
        let before = ch.clone();

        let elite = NeuralNet::new_specialized(NeuralNetId(2)).with_min_iq(120);
        assert_eq!(
            ch.install_neural_net(elite.clone()),
            Err(NeuralNetError::BelowMinIq)
        );
        assert_eq!(ch, before);

        ch.add_implant(implant(20, 0));
        assert_eq!(ch.install_neural_net(elite), Ok(()));
    }

    #[test]
    fn install_for_below_min_iq_keeps_net_unchanged() {
        let mut nn = NeuralNet::new_specialized(NeuralNetId(1)).with_min_iq(120);
        assert_eq!(
            nn.install_for(119, cit("Мерказ")),
            Err(NeuralNetError::BelowMinIq)
        );
        assert_eq!(nn.install_count, 0);
        assert_eq!(nn.first_citizenship, None);
        assert_eq!(nn.install_for(120, cit("Мерказ")), Ok(()));
    }

    #[test]
    fn new_error_display_messages() {
        assert_eq!(
            CitizenshipError::FirstCitizenshipLocked.to_string(),
            "first citizenship is already recorded and cannot be changed"
        );
        assert_eq!(
            LegalStatusError::CannotBeFreed.to_string(),
            "a slave cannot be freed in this state"
        );
        assert_eq!(
            ExcommunicationError::NotExcommunicated.to_string(),
            "not excommunicated"
        );
        assert_eq!(
            NeuralNetError::BelowMinIq.to_string(),
            "character iq is below the neural net minimum iq"
        );
    }
}
