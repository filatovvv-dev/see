# SEE — Space And Everyone Else

SEE — однопользовательская оффлайн космическая sandbox-RPG для Windows:
симулятор экономики и политики, в котором живут разумные.

**Стек:** Rust (workspace, Edition 2024), Bevy 0.15 и `bevy_ecs`, `serde`,
`serde_json`, `thiserror`, `tracing`, `rand` 0.8 и `rand_chacha`, `tokio`,
`reqwest`.

## Статус

- **MVP `sim_core` — готов:** 9 модулей, 200 тестов (всего по workspace: 230 — `sim_core` 200, `narrative` 8, `client` 22).
- `narrative` и `client` — каркасы, `shared` — заглушка.
- `docs/` — заполнены.

## Ключевые правила

- **Детерминизм:** `sim_core` не использует `thread_rng`, `SystemTime`,
  `HashMap`/`HashSet` с RandomState. Только `BTreeMap`/`BTreeSet` и
  seeded RNG.
- **Лор:** субъекты мира — «разумные», люди (humans) — один из видов.
  Три слоя развития: мозг (знания — навсегда), нейросеть (заменяемая),
  импланты (компенсация).
- **Правила для ИИ-программистов:** `AGENTS.md`.

## Структура репозитория

```
.
├── AGENTS.md                    правила для ИИ-программистов
├── Cargo.toml                   workspace
├── .agents/skills/              профильные инструкции агентов
├── .github/workflows/           CI (ci.yml, quality-gate.yml)
├── docs/
│   ├── lore-bible.md            лор
│   ├── economy-model.md         экономическая модель
│   └── acceptance-criteria.md   критерии приёмки MVP
└── crates/
    ├── sim_core/                ядро симуляции (без рендера и ввода-вывода)
    │   └── src/                 time, entities, market, production,
    │                            character, factions, contracts, ai_agents,
    │                            world
    ├── shared/                  общие типы и данные (заглушка)
    ├── narrative/               текстовый слой, LLM-адаптер (заглушки)
    └── client/                  клиент на Bevy (каркас)
```

## Модули `sim_core`

| Модуль | Статус | Тесты | Что есть |
|---|---|---|---|
| `time` | ✅ | 8 | Игровое время, тики, `TICKS_PER_GAME_HOUR = 60` |
| `entities` | ✅ | 11 | ID-типы, `Cargo`, `Agent`, `Ship`, `MarketOrder` |
| `market` | ✅ | 12 | Стакан ордеров, матчинг, лучшие цены, история цен |
| `production` | ✅ | 10 | Рецепты, очередь производства |
| `character` | ✅ | 51 | Мозг, нейросети, импланты, базы знаний, гражданство, правовой статус, отлучение |
| `factions` | ✅ | 19 | Фракции, симметричные отношения, войны |
| `contracts` | ✅ | 24 | Контракты (курьерские, боевые, производственные), конечный автомат |
| `ai_agents` | ✅ | 31 | GOAP-планировщик на A*, мозг NPC-агента |
| `world` | ✅ | 34 | Оркестратор: тик-цикл, события, AI-шаг, сохранение/загрузка (JSON) |
| **Всего** | | **200** | |

### Другие крейты

| Крейт | Статус |
|---|---|
| `narrative` | 🟡 Каркас (PR #29, 8 тестов): `LlmClient`, `MockLlmClient`, `NarrativeEngine`, `build_prompt`; кэш, шаблонный генератор и валидатор — заглушки |
| `client` | 🟡 22 теста: каркас + `narrative` (mock), спрайты агентов, save/load (F5/F9), UI с кириллицей; рендер кораблей и планет не начат |
| `shared` | 🚧 Заглушка: общие типы и загрузчики данных не реализованы |

## Документация

- [`AGENTS.md`](AGENTS.md) — правила для ИИ-программистов.
- [`docs/lore-bible.md`](docs/lore-bible.md) — лор: государства, фракции,
  нейросетевые компании, гражданство, рабство, Древние.
- [`docs/economy-model.md`](docs/economy-model.md) — экономическая модель:
  кредит, налоги, цены, производственные цепочки.
- [`docs/acceptance-criteria.md`](docs/acceptance-criteria.md) —
  критерии приёмки MVP.

## Сборка и тесты

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

На Linux для сборки `client` нужны системные библиотеки Bevy (см.
`.github/workflows/ci.yml`).

## Дорожная карта

- ✅ MVP `sim_core` (9 модулей, 200 тестов).
- 🟡 `narrative` (каркас: PR #29, 8 тестов; реальный LLM-клиент — ⬜).
- 🟡 `client` (каркас + `narrative` (mock), PR #37, 22 теста; рендер кораблей и планет, реальный HTTP-клиент — ⬜).
- ✅ Оркестратор `World` (PR #27).
- ✅ Балансный PR (PR #24).
- ✅ Сохранения и загрузки (PR #33, F5/F9 в клиенте — PR #34).
- ⬜ `Cargo.lock` в коммит (техдолг перед релизом).

## Лицензия

TBD.

---

_Последнее обновление: 2026-11-26._
