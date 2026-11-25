# Критерии приёмки MVP

> Чек-листы: `- [x]` — критерий выполнен, `- [ ]` — отложено.
> Числа тестов — на момент составления документа (проверено командой
> `grep -c '#\[test\]' crates/sim_core/src/*.rs`): всего **166**.

## 1. Общие критерии MVP

- [x] Все модули `sim_core` компилируются.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` проходит
  без замечаний.
- [x] `cargo fmt --all -- --check` проходит.
- [x] Все тесты зелёные: `cargo test --workspace` (166 тестов).
- [x] Детерминизм: в коде `sim_core` нет `HashMap`/`HashSet`
  (RandomState), `thread_rng`, `SystemTime::now` (проверка поиском
  по `crates/sim_core/src`: совпадение только в поясняющем комментарии
  `time.rs`).
- [x] Документация: `lore-bible.md`, `economy-model.md`,
  `acceptance-criteria.md` заполнены, `README.md` актуален.

## 2. `time.rs`

- [x] `GameTime` (счётчик тиков), `TICKS_PER_GAME_HOUR = 60`.
- [x] Перевод тиков в игровые часы (`to_hours`).
- [x] Тесты: не меньше 8 (сейчас 8).

## 3. `entities.rs`

- [x] `AgentId`, `ShipId`, `ItemId`, `SystemId` — обёртки над `u64`.
- [x] `Cargo` — `BTreeMap` с закрытым полем, доступ только через
  методы; `CargoError`.
- [x] `Agent`, `Ship`, `MarketOrder`.
- [x] Тесты: не меньше 11 (сейчас 11).

## 4. `market.rs`

- [x] `Market`, `Trade`, `MarketError`.
- [x] `match_orders` (аукцион): покупатели по убыванию цены,
  продавцы по возрастанию, при равенстве — по номеру ордера;
  цена сделки — цена продавца; частичное исполнение.
- [x] `best_bid`/`best_ask`, `record_price`/`last_price`.
- [x] Тесты: не меньше 12 (сейчас 12).

## 5. `production.rs`

- [x] `Recipe`, `CompletedBatch`, `ProductionQueue`
  (`enqueue`, `tick`, `pending_jobs`), `apply_batch`.
- [x] Арифметика с насыщением, без паники при переполнении.
- [x] Тесты: не меньше 10 (сейчас 10).

## 6. `character.rs`

- [x] `Character`, `Brain`, `NeuralNet` (Basic/Specialized/Individual),
  `Implant`, `KnowledgeBase`, `FactionId`.
- [x] `Citizenship`, `StateId`, `Manufacturer`, `LegalStatus`,
  `Excommunication`.
- [x] Ошибки: `StudyError`, `NeuralNetError`, `CitizenshipError`,
  `LegalStatusError`, `ExcommunicationError`.
- [x] `memory_pb: u64` (не `u32`): несколько имплантов «+1 ЙБ» не
  переполняют значение.
- [x] Первое гражданство записывается на нейросети и остаётся навсегда.
- [x] Отлучение блокирует только новые базы знаний (выпущенные после
  года отлучения); старые базы изучаются.
- [x] Коткор: раб не может стать свободным (`CannotBeFreed`).
- [x] Тесты: не меньше 50 (сейчас 51; было 23).

## 7. `factions.rs`

- [x] `Faction`, `FactionKind`, `FactionRegistry`, `RegistryError`.
- [x] `WAR_RELATION = -10` (границы отношений `-10..=10`).
- [x] Отношения симметричны: `set_relation(a, b)` меняет и `(b, a)`.
- [x] Тесты: не меньше 19 (сейчас 19).

## 8. `contracts.rs`

- [x] `Contract`, `ContractKind` (Courier / Combat / Production),
  `ContractState` (Available, Accepted, Completed, Failed, Expired),
  `ContractRegistry`, `ContractError`.
- [x] Конечный автомат: терминальные состояния не меняются.
- [x] Тесты: не меньше 24 (сейчас 24).

## 9. `ai_agents.rs`

- [x] `Fact`, `WorldState`, `Condition`, `Effect`, `Action`, `Goal`,
  `Plan`, `PlanError`, `Planner`, `AgentBrain`.
- [x] A* (поиск вперёд) с детерминированным выбором среди равных по
  стоимости планов: не зависит от порядка действий во входных данных.
- [x] Лимиты `max_depth` и `max_expansions`; при их исчерпании —
  `SearchExhausted`, отдельно от `NoPlanFound`.
- [x] Тесты: не меньше 31 (сейчас 31).

## 10. Документация

- [x] `lore-bible.md`: разделы 1–8 — государства, динамика мира,
  прочие фракции (включая нейросетевые компании), нейминг, гражданство
  и «дикие», рабство, Древние, общество и разумные.
- [x] `economy-model.md`: разделы 1–10 — кредит, банки, социальная
  структура, три бесплатные вещи, цены (включая компоненты кораблей и
  реальные ориентиры), производственные цепочки (включая техническую
  документацию), рабство, параметры балансировки, открытые вопросы,
  связь с кодом.
- [x] `acceptance-criteria.md`: этот файл.
- [x] `README.md`: актуален.

## 11. Что не входит в MVP (отложено)

- [ ] `narrative` (LLM-адаптер): trait и заглушка, интеграционные
  тесты отложены.
- [ ] `client` (рендер, UI): заглушка.
- [ ] Оркестратор `World`, связывающий модули.
- [ ] Балансный PR: реакторы для гиперприводов по классам, классы
  оружия, масса корпуса, класс «истребитель/перехватчик»
  (см. `economy-model.md`, раздел 9).
- [ ] `Cargo.lock` в коммите — техдолг перед релизом (сейчас файл в
  `.gitignore`).
- [ ] `economy-model.md`, раздел 10: строки про `NeuralNet.min_iq`,
  `Character.citizenship`, `LegalStatus` и отметку об освобождении от
  отлучения помечены как не реализованные, хотя уже реализованы в
  `character.rs`; обновить.
