# In-memory репозитории в юнит-тестах executor и queue service

Статус: **план на согласование**. Пункт бэклога #13: «Проверить executor и queue service - можно ли переделать так,
чтобы юнит тесты использовали in-memory реализации (сейчас они на sqlite через test_state_with)».

Дата: 2026-08-23

## Вердикт

**Да, возможно, без блокеров.** Все in-memory реализации уже существуют
(`src/db/inmemory_*.rs`, 11 штук) и их контрактное совпадение с sqlite гарантируют
паритет-сьюты в `src/db/parity.rs` (для queue: priority/dequeue, CAS outcomes,
pagination, purge+timeout — против обеих реализаций).

## Контекст (текущее состояние)

- `UniAppState<Q, R, U, P, S, A, Se, C, K, L, M>` полностью генерик над трейтами
  репозиториев (`src/state.rs:46`); `AppState` — alias со всеми Sqlite-типами (`src/state.rs:119`).
- `AppStateBuilder` жёстко создаёт Sqlite*-репозитории из пула внутри `build()`
  (`src/state.rs:173`): сервисная сборка (строки 195–255) не переиспользуется.
- Фикстуры `test_state()` / `test_state_with(pool, queue_repo)` всегда строят sqlite-state
  (`src/test_fixtures.rs:25,31`). `with_queue_repo` принимает только `Arc<SqliteQueueRepository>`.
- Тесты в скоупе задачи:
  - `actions/executor.rs` — 7 тестов, все через `setup*()` → `test_state_with`;
    нужен `QueueService`, `UserService`, опционально `TwitchAuthService<C>` + `TwitchConfig`.
  - `queue/service.rs` — 10 тестов через `test_state()` / `test_state_with`
    (последний — только чтобы держать прямой хэндл `queue_repo` для `mark_timed_out`).
- Вне скоупа: `api/admin.rs:449` тоже использует `test_state_with` — не трогаем в этой задаче.

## Почему это безопасно

- Атомарность параллельного dequeue (`dequeue_next_parallel_only_one_spin`,
  `complete_parallel_only_one_success`) сохраняется: `InMemoryQueueRepository`
  делает проверку «нет Spinning» + взятие первой записи под одним `Mutex`-локом
  (`src/db/inmemory_queue.rs:65`), как sqlite — под транзакцией.
- Тест `dequeue_next_retries_error_entry` зовёт `queue_repo.mark_timed_out()` напрямую
  по трейту — работает с in-memory без изменений.
- Паритет-сьюты уже фиксируют семантику Error→Pending приоритета, CAS, пагинации и таймаутов.
- FK-каскады (sqlite-only, вне паритета) сервисными тестами executor/queue не используются.

## План

### Шаг 1. Вынести сборку сервисов в генерик-функцию (prod-рефакторинг)

В `src/state.rs` добавить генерик-функцию/метод, принимающую готовые `Arc`-репозитории
и config-store, и собирающую все сервисы `UniAppState` (сейчас строки 195–255 `build()`):

```rust
pub struct UniStateRepos<Q, R, U, P, S, A, Se, K, L, M> { /* Arc<...> на каждый репо */ }

pub async fn assemble_uni_state<C: PlatformCredentialRepository>(
    repos: UniStateRepos<...>,
    credentials: Arc<C>,
    config: Arc<AppConfigStore-like<K>>,
) -> Result<UniAppState<...>, RepositoryError>
```

`AppStateBuilder::build()` становится тонкой обёрткой: создаёт Sqlite*-репозитории из пула
(плюс существующий hack `DELETE FROM roulette_slots/rarities` при `seeded=false`) и зовёт
ассемблер. Логика не дублируется — единое место сборки (совпадает с духом бэклога #2).

### Шаг 2. In-memory фикстура в test_fixtures.rs

Добавить:

```rust
pub type InMemoryAppState = UniAppState<
    InMemoryQueueRepository, InMemoryRarityRepository, InMemoryUserRepository,
    InMemoryPlatformRepository, InMemoryRouletteSlotRepository, InMemoryAdminRepository,
    InMemorySessionRepository, InMemoryPlatformCredentialRepository,
    InMemoryConfigRepository, InMemoryRuleRepository, InMemoryActionRepository,
>;

/// Пустой slate без сидов (in-memory и так пустой), sqlite не нужен вовсе.
pub async fn test_state_inmemory() -> (InMemoryAppState, Arc<InMemoryQueueRepository>);
```

- Собирается через ассемблер из шага 1 с `InMemory*::new()` + `InMemoryConfigRepository`.
- `StaticConfig::test_config()` / `RuntimeConfig::test_runtime("test-key")` — как сейчас.
- Возвращает также `Arc<InMemoryQueueRepository>` для тестов с прямым доступом к репо
  (замена паттерна `test_state_with(pool, Some(queue_repo))`).
- `spawn_logging_handler(ingress.subscribe())` — как в проде.

### Шаг 3. Перевести тесты executor

`actions/executor.rs`:

- `TestExecutor` alias → `ActionExecutor<InMemory...>` типы.
- `setup_with_twitch()` / `setup_without_twitch()` → `test_state_inmemory()`;
  `TwitchAuthService` строится на `InMemoryPlatformCredentialRepository` из стейта
  (он уже лежит в `state.credentials`).
- Сами тесты не меняются — они ходят только через сервисы.

### Шаг 4. Перевести тесты queue service

`queue/service.rs`:

- `test_state()` → `test_state_inmemory()` (9 тестов).
- `dequeue_next_retries_error_entry` → берёт `queue_repo` из кортежа фикстуры вместо
  ручного `SqliteQueueRepository::new(pool)`.
- `setup_slots` не меняется (rarities/slots создаются явно через сервисы).

### Шаг 5. Проверка

```sh
cargo check --package backend
cargo nextest run --package backend -E 'test(actions::executor) or test(queue::service)'
cargo nextest run --package backend -E 'test(parity)'
cargo clippy --all-targets
cargo fmt --check
```

## Риски / нюансы

- **Дрейф ассемблера**: если шаг 1 сделать небрежно (скопировать код вместо выноса),
  появится вторая точка правды. Митигация — ассемблер один, builder его вызывает;
  прод-тесты (`main`-path) остаются на sqlite и словят любую поломку.
- **Семантика seeded**: у sqlite `seeded=true` тянет сиды из миграций, in-memory пустой
  по умолчанию. Для скоупа задачи это неважно (тесты создают данные явно), но фикстура
  должна называться/документироваться как «пустой slate».
- **Не расширяем на всё сразу**: `api/admin.rs` и прочие интеграционные тесты осознанно
  остаются на sqlite — там важен реальный SQL/FK. Отдельный пункт бэклога при желании.

## Оценка

1.5–3 ч: шаг 1 ~1 ч, шаги 2–4 ~0.5–1 ч, шаг 5 ~0.5 ч.
