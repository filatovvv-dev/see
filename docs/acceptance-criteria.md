# Критерии приёмки MVP

> Чек-листы: `- [x]` — критерий выполнен, `- [ ]` — отложено.
> Числа тестов — на момент составления документа (проверено командой
> `grep -c '#\[test\]' crates/sim_core/src/*.rs`): всего **192**.

## 1. Общие критерии MVP

- [x] Все модули `sim_core` компилируются.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` проходит
  без замечаний.
- [x] `cargo fmt --all -- --check` проходит.
- [x] Все тесты зелёные: `cargo test --workspace` (192 теста).
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

## 10. `world.rs`

- [x] **Оркестратор `World`** (`crates/sim_core/src/world.rs`): `World`,
  `WorldEvent`, `AgentMind`, `WorldError`. Тик-цикл: время →
  производство → рынок → контракты → AI. Один тик = одно действие из
  плана. Детерминированный (`ChaCha8Rng` seeded, `BTreeMap`).
- [x] Тесты: не меньше 26 (сейчас 26; выполнено в PR #27).

## 11. Документация

- [x] `lore-bible.md`: разделы 1–8 — «Государства», «Динамика мира:
  появление новых государств», «Прочие фракции» (включая 3.10
  «Нейросетевые компании» и 3.11 «Прочие корпорации»: 8 корпораций —
  фарма, верфи, агро, электроника, искины), «Нейминг: правила и
  запреты»,
  «Гражданство и «дикие»», «Рабство», «Древние», «Общество и
  разумные».
- [x] `economy-model.md`: разделы 1–10 — «Кредит», «Банки и движение
  капитала», «Социальная структура», «Что бесплатно (три вещи)»,
  «Цены» (включая 5.6 «Компоненты корабля» и 5.7 «Реальные ориентиры
  (калибровка)»), «Производственные цепочки (MVP)» (включая 6.5
  «Техническая документация»), «Рабство как экономический фактор»,
  «Параметры балансировки», «Открытые вопросы», «Связь с кодом».
- [x] `acceptance-criteria.md`: этот файл.
- [x] `README.md`: актуален.

## 12. Что не входит в MVP (отложено)

- [ ] `narrative` (LLM-адаптер): trait и заглушка, интеграционные
  тесты отложены.
- [ ] `client` (рендер, UI): заглушка.
- [x] Балансный PR: реакторы для гиперприводов по классам, классы
  оружия, масса корпуса, класс «истребитель/перехватчик»
  (выполнено в PR #24; см. `economy-model.md`, 5.6.1.1, 5.4, 5.6.6,
  5.3). Форсаж реактора и совместимость классов оружия с классами
  кораблей остаются открытыми вопросами (`economy-model.md`, раздел 9).
- [ ] `Cargo.lock` в коммите — техдолг перед релизом (сейчас файл в
  `.gitignore`).
- [ ] **`unused_dependencies` в stub-крейтах.** `client`, `narrative`,
  `shared` и `sim_core` генерируют предупреждения `unused_dependencies`
  (cargo-lint `cargo::unused_dependencies`, не rustc). На CI (stable,
  rustc 1.100.0) шаг `cargo clippy --workspace --all-targets` даёт 10
  предупреждений (`client` 3, `narrative` 2, `shared` 3, `sim_core` 2),
  шаг `cargo test --workspace --exclude client` — 8 (`narrative` 2,
  `shared` 3, `sim_core` 3, включая `rand`). Ожидаемо до post-MVP, когда
  крейты начнут использовать зависимости. CI зелёный: предупреждения не
  валят сборку. Решение: либо `[workspace.lints.cargo]
  unused_dependencies = "allow"` в `Cargo.toml` (плюс `[lints]
  workspace = true` в каждом крейте; отдельный PR, CODEOWNERS), либо
  оставить как есть до post-MVP. Локальный cargo 1.97 этих
  предупреждений не показывает, настройка `allow` не проверялась.
