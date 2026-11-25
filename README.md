# SEE — Space And Everyone Else

SEE — однопользовательская оффлайн космическая sandbox-RPG для Windows:
симулятор экономики и политики, в котором живут разумные.

**Стек:** Rust (workspace, Edition 2024), Bevy 0.15 и `bevy_ecs`, `serde`,
`serde_json`, `thiserror`, `tracing`, `rand` 0.8 и `rand_chacha`, `tokio`,
`reqwest`.

## Статус

- **MVP `sim_core` — готов:** 8 модулей, 166 тестов.
- `narrative`, `client`, `shared` — заглушки.
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
    │                            character, factions, contracts, ai_agents
    ├── shared/                  общие типы и данные (заглушка)
    ├── narrative/               текстовый слой, LLM-адаптер (заглушки)
    └── client/                  клиент на Bevy (заглушка)
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
| **Всего** | | **166** | |

### Другие крейты

| Крейт | Статус |
|---|---|
| `narrative` | 🚧 Заглушки: пять пустых модулей (кэш, LLM-адаптер, сборка промптов, шаблонный генератор, валидатор) |
| `client` | 🚧 Заглушка: `main.rs` печатает «SEE client stub», рендер не начат |
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

- ✅ MVP `sim_core` (8 модулей, 166 тестов).
- ⬜ `narrative` (LLM-адаптер).
- ⬜ `client` (рендер, UI).
- ⬜ Оркестратор `World`, связывающий модули.
- ⬜ Балансный PR (реакторы, классы оружия, масса корпуса, истребитель).
- ⬜ Сохранения и загрузки.
- ⬜ `Cargo.lock` в коммит (техдолг перед релизом).

## Лицензия

TBD.

---

_Последнее обновление: 2026-11-25._
