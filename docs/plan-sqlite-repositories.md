# SQLite-репозитории на sqlx вместо in-memory

Статус: **план согласован**, к выполнению. Пункт бэклога: «Сохранение правил, действий и остальных данных на диск (sqlite)».

Дата: 2026-08-22

## Контекст (текущее состояние)

- 11 трейтов репозиториев (`src/*/repository.rs`), все реализации в `src/db/inmemory_*.rs`:
  `QueueRepository`, `RarityRepository`, `UserRepository`, `PlatformRepository`,
  `RouletteSlotRepository`, `AdminRepository`, `SessionRepository`,
  `PlatformCredentialRepository`, `ConfigRepository`, `RuleRepository`, `ActionRepository`.
- `AppState` — конкретный alias поверх in-memory типов (`src/state.rs:111`);
  `AppStateBuilder` жёстко прибит к ним (поля `config`, `credentials_repo`, `queue_repo`).
- `main.rs` тоже прибит: тип `credentials_repo` (:42), сигнатура замыкания `build_ingress` (:77),
  фоновые таски принимают `Arc<ConfigStore<InMemoryConfigRepository>>` (:185, :198).
- sqlx 0.9.0 уже в deps (`sqlite+chrono+runtime-tokio`), нигде не используется.
- ID сейчас генерируются AtomicU32 с 1; конструкторы id-типов `pub(crate)` — sqlite-код внутри крейта сможет их строить.
- `RepositoryError` имеет варианты `Conflict(String)` / `Database(String)`.
- Сиды: platforms(3), rarities(4, фикс. id), roulette_slots(4, кириллица, веса 50/20/5/1) — сейчас в
  `InMemory*::new_seeded()`, включаются флагом `seeded` в builder'е.

### Семантика, которую нужно воспроизвести точно

- queue:
  - `peek_next` / `dequeue_next_with_slot`: приоритет Error → Pending по порядку id;
    dequeue атомарен («нет Spinning» + взятие первой);
  - `update_status_if` — CAS по ожидаемому статусу;
  - keyset-пагинация (`id > cursor`, опц. фильтр статуса, LIMIT, ORDER BY id ASC);
  - `mark_timed_out`: Spinning c updated_at < cutoff → Error; `purge_completed_cancelled` → кол-во удалённых.
- sessions: `take_ticket` — деструктивное чтение (удалить и вернуть).
- users: `link_platform` — глобальная уникальность `(platform_id, platform_user_id)` → `Conflict`;
  попутно трогает `users.updated_at`.
- rules/actions update: сохраняют `created_at`, ставят `updated_at = now`.

## Решения

- Миграции: `sqlx::migrate!`, **пошагово** — одна минимальная миграция на каждый репозиторий/шаг,
  в порядке реализации. Никакого монолитного init-файла: каждая миграция маленькая,
  применяется независимо, её видно в `_sqlx_migrations`. Сиды едут в своих миграциях.
  Нужны sqlx-фичи `"migrate"`, `"macros"`.
- Запросы: **compile-time проверенные `query!` / `query_as!`** для всех статических запросов.
  Офлайн-метаданные `.sqlx` коммитятся в репо; sqlx-cli установлен, синхронизация автоматизируется
  скриптом. Динамический SQL (queue `list` с опциональными фильтрами статуса/курсора) — runtime API,
  остальное — макросы. Корректность SQL дополнительно дублируется паритет-тестами.
- Воркфлоу метаданных:
  - **одна БД для всего**: `data/server.db` из `.env` (`DATABASE_URL=sqlite:data/server.db?mode=rwc`)
    — и рантайм, и компиляция макросов, и prepare. Макросы sqlx сами читают dotenv и ходят
    в живую БД; свежий клон без `.env` автоматически уходит в офлайн на закоммиченном `.sqlx`;
  - запуск: **`just sqlx-prepare`** (рецепт в корневом justfile → `apps/backend/scripts/sqlx-prepare.ps1`):
    absolute-path `data/server.db` → `sqlx database create` → `sqlx migrate run`
    → `cargo sqlx prepare -- --all-targets`;
  - правило: поменял запрос или миграцию → запусти рецепт → закоммить обновлённый `.sqlx`;
  - обычная сборка без `DATABASE_URL` сама падает обратно на `.sqlx` (offline mode), CI ничего
    поднимать не должен — там только deploy-пайплайны, prepare на CI не нужен.
- Старые юнит-тесты остаются на in-memory. Для sqlite пишем общие generic паритет-сьюты,
  гоняемые против обеих реализаций.
- Тестовые БД — temp-файлы (`std::env::temp_dir()` + uuid v7) + прогон MIGRATOR.
  `:memory:` не используем: пул даёт отдельную БД на каждый коннект.

---

## Шаг 0a. Нормализация переводов строк — отдельный атомарный коммит

Делается **до** всех остальных шагов и живёт в собственном коммите без единой функциональной правки,
иначе дифф sqlite-работы утонет в построчных изменениях ~282 файлов:

1. Корневой `.gitattributes`: `* text=auto eol=lf` — LF для всех текстовых файлов.
   `text=auto`, а не голый `text` — бинарники эвристика не тронет.
2. `git add --renormalize .` → сейчас нормализуются 282 файла.
3. Закоммитить только это; после коммита миграции sqlx хешируются стабильно.

---

## Шаг 0. Инфраструктура

1. **Удалить мёртвую пару миграций** `20260723041117_create_roulette_slots.{up,down}.sql`
   (лежит в `migrations/`, никогда не использовалась, битая кодировка, схема расходится с доменом).
   Критично: migrator выполнит её на свежей БД — создаст свои `rarities`/`roulette_slots`
   с mojibake-сидами, и `0005_rarities.sql` упадёт на «table already exists».
2. `apps/backend/Cargo.toml`: sqlx features += `"migrate"`, `"macros"`.
3. `scripts/sqlx-prepare.ps1` + рецепт `sqlx-prepare` в корневом justfile
   (см. «Воркфлоу метаданных»). ✅
4. `src/db/sqlite/mod.rs`:
   - `static MIGRATOR: Migrator = sqlx::migrate!();`
   - `pub async fn connect(path) -> SqlitePool` — create_if_missing, journal_mode=WAL,
     foreign_keys=ON, busy_timeout ~5s; перед открытием файла — `create_dir_all` родительского
     каталога (`data/` может не существовать);
   - хелпер `map_err(sqlx::Error) -> RepositoryError` (unique violation → `Conflict`, остальное → `Database`).
5. Тестовый хелпер создания temp-БД с миграциями.

Общие соглашения по схеме: даты TEXT RFC3339 (chrono через sqlx), булевы INTEGER,
JSON-колонки TEXT. id генерируемые — `INTEGER PRIMARY KEY AUTOINCREMENT`; фиксированные PK
(platforms, rarities, admins, sessions, login_tickets, runtime_config, platform_credentials) — без
автоинкремента. Колонка триггера в rules названа `trigger_kind` (TRIGGER — зарезервированное слово).
Таймстампы (`created_at`/`updated_at`) — только там, где они уже есть в доменных структурах
(queue_entries, users, actions, rules); остальным таблицам заранее не добавляем —
понадобится, сделаем отдельную миграцию (`ALTER TABLE ADD COLUMN` в SQLite дешёв).

## Шаги 1–7. Репозитории + их миграции

Порядок — от простого к сложному; каждый шаг: миграция → структура с `SqlitePool` →
trait impl → маппинг строк → тесты. Компилируется и проверяется независимо.

1. **Config** (`db/sqlite/config.rs`)
   - `migrations/0001_runtime_config.sql`: `runtime_config(id PK CHECK(id=1), payload JSON)`.
   - load/save JSON-блоба. Самый простой, на нём отрабатываем общий паттерн.
2. **Admin** (`db/sqlite/admin.rs`)
   - `0002_admins.sql`: `admins(twitch_id TEXT PK, display_name NULL, is_root INT, created_at TEXT)`.
3. **PlatformCredential + Platform** (`db/sqlite/platform.rs`, `db/sqlite/platform_credential.rs`)
   - `0003_platforms_and_credentials.sql`:
     `platforms(id PK, name UNIQUE)` + seed twitch/youtube/vk_video_live;
     `platform_credentials(platform_id PK, credential TEXT NOT NULL)`.
     platforms — read-only из сида.
4. **Session** (`db/sqlite/session.rs`)
   - `0004_sessions_and_tickets.sql`:
     `sessions(token TEXT PK, twitch_user_id, twitch_user_name NULL, created_at, expires_at)` и
     `login_tickets(ticket TEXT PK, ...те же поля)`.
   - `take_ticket` = `DELETE ... RETURNING`; purge = DELETE + rows_affected.
5. **Rarity + RouletteSlot** (`db/sqlite/rarity.rs`, `db/sqlite/roulette_slot.rs`)
   - `0005_rarities.sql`: `rarities(id PK, name, display_name, image, color)` + seed 4 дефолтов
     (как `new_seeded`, фикс. id 1–4).
   - `0006_roulette_slots.sql`: `roulette_slots(id AUTOINC, name, rarity_id FK→rarities,
     weight INTEGER, action TEXT)` + seed 4 из `SEEDED_SLOTS`.
   - save = INSERT + RETURNING id; update/delete по PK; weight u64↔i64 cast.
6. **User** (`db/sqlite/user.rs`)
   - `0007_users_and_platforms.sql`:
     `users(id AUTOINC, display_name, created_at, updated_at)`;
     `user_platforms(id AUTOINC, user_id FK CASCADE, platform_id FK→platforms,
     platform_user_id, platform_username, UNIQUE(platform_id, platform_user_id))`.
   - `link_platform` в транзакции (INSERT; unique violation → `Conflict`; UPDATE users.updated_at);
     `find_by_platform` JOIN-ом.
7. **Action + Rule** (`db/sqlite/action.rs`, `db/sqlite/rule.rs`)
   - `0008_actions.sql`: `actions(id AUTOINC, name, kind JSON, enabled INT, created_at, updated_at)`.
   - `0009_rules.sql`: `rules(id AUTOINC, name, enabled INT, trigger_kind TEXT, conditions JSON,
     action_id FK→actions, created_at, updated_at)`.
   - kind/conditions через `serde_json` (roundtrip уже покрыт тестами домена);
     update сохраняет created_at, updated_at=now (как in-memory).
8. **Queue** (`db/sqlite/queue.rs`) — самый сложный
   - `0010_queue_entries.sql`: `queue_entries(id AUTOINC, user_id, user_name, status TEXT,
     result_slot_id NULL, created_at, updated_at)`.
   - peek/dequeue: приоритет Error→Pending через
     `ORDER BY CASE status WHEN 'error' THEN 0 ELSE 1 END, id`;
   - `dequeue_next_with_slot`: **один атомарный стейтмент** (SQLite сериализует запись,
     гонка невозможна конструктивно):
     ```sql
     UPDATE queue_entries SET status='spinning', result_slot_id=?, updated_at=?
     WHERE id = (SELECT id FROM queue_entries
                 WHERE status IN ('error','pending')
                   AND NOT EXISTS (SELECT 1 FROM queue_entries WHERE status='spinning')
                 ORDER BY CASE status WHEN 'error' THEN 0 ELSE 1 END, id LIMIT 1)
     RETURNING ...;
     ```
     пустой результат → отдельный `EXISTS(spinning)` → AlreadyActive / Empty;
   - `update_status_if`: `UPDATE ... WHERE id=? AND status=?` → rows_affected →
     NotFound / StatusMismatch / Updated (+SELECT);
   - `mark_timed_out`: UPDATE spinning c updated_at<cutoff → RETURNING;
     `purge_completed_cancelled`: DELETE → rows_affected;
   - `count_by_status`: GROUP BY; list — keyset `WHERE id > ? [AND status=?] ORDER BY id LIMIT ?`.
   - инвариант приоритета Error→Pending задокументировать rustdoc-комментарием на трейте
     `QueueRepository` (`src/queue/repository.rs`) — это контракт для обеих реализаций.

## Почему dequeue/update_status_if не выносим в сервис

Эти два метода сложны не потому, что логика лежит не там, а потому что они обязаны быть
**атомарными** — check-then-act:

- `dequeue_next_with_slot`: «нет Spinning» + взять первую error/pending + перевести в Spinning —
  одна неделимая операция. Разнос на три вызова сервиса (`has_spinning()` → `first_pickable()` →
  `set_spinning()`) даёт гонку: два параллельных спина оба проходят проверку.
  Починить можно только транзакцией, живущей через несколько вызовов репо → слой
  `with_tx(|tx| ...)` во всех репозиториях: больше инфраструктуры, in-memory реализация сложнее.
- `update_status_if` — CAS: complete/cancel валиден только пока статус Spinning; get+set
  возвращает lost update'ы.

В SQLite сложность схлопывается в один стейтмент (см. шаг Queue) — тело метода ~15 строк.
Слой между service и repo уже существует: это типы `DequeueOutcome` / `StatusUpdateOutcome`
(`AlreadyActive`, `StatusMismatch`) — сервисный словарь над репо без исключений.
Слой транзакций вводим только при появлении реального кейса «атомарно через несколько репо».

## Приоритет Error→Pending: где документируем

Источник правды — **rustdoc на трейте** `src/queue/repository.rs`:
на `peek_next` и `dequeue_next_with_slot` док-комментарий с инвариантом
«сначала записи в Error (по возрастанию id), затем Pending (по возрастанию id)».
Контракт читают оба реализатора (in-memory/sqlite) рядом с их сигнатурами; паритет-сьюты
(peek-prefers-error, dequeue-prefers-error) фиксируют его исполнение с двух сторон.
План — не место для контрактов кода, поэтому в этом документе только ссылка на решение.

## Шаг 8. Паритет-тесты

Generic-сьюты (одни и те же `async fn suite_*<R: XxxRepository>`), прогон для InMemory и Sqlite:

- CRUD-roundtrip по каждому репо;
- семантика очереди: peek-prefers-error, AlreadyActive, CAS, mark_timed_out, purge, пагинация;
- деструктивный take_ticket;
- Conflict при link_platform;
- сохранение created_at при update (rules/actions/users);
- JSON-roundtrip kind/conditions.

## Шаг 9. Вёринг

1. `state.rs`: alias `AppState` → sqlite-типы. Builder прибит к sqlite-типам **без генериков** —
   принимает `pool: SqlitePool` и строит все репозитории над ним;
   `with_empty_repos` теперь удаляет сиды из БД (`DELETE FROM roulette_slots / rarities`) —
   сохраняет старое поведение тестов «пустые репозитории»;
   `with_queue_repo` → `Arc<SqliteQueueRepository>`; обновить `AppQueueService` / `AppSessionService`.
2. `main.rs`: **dotenv загрузить до создания пула** (сейчас `dotenvy::dotenv()` сидит внутри
   `StaticConfig::load()`, который вызывается позже — `DATABASE_URL` из `.env` не подхватится);
   pool создаётся первым делом; сигнатуры `build_ingress` и фоновых тасок — новые типы;
   `ConfigStore::load_or_seed(repo)` → на `SqliteConfigRepository`
   (ротация widget access key наконец переживает рестарт).
3. `test_fixtures.rs` переезжает на **temp-sqlite БД**: все хендлеры сидят на конкретном
   `State<AppState>`, при смене alias'а fixtures физически не могут остаться на in-memory.
   In-memory реализации остаются только в юнит-тестах репозиториев и паритет-сьютах.
4. Задокументировать env var `DATABASE_URL` (дефолт `./data/backend.db`, mode=rwc).

## Готовность (Definition of Done)

- `.sqlx` сгенерирован скриптом и закоммичен вместе с изменениями;
- `cargo nextest run --package backend` зелёный;
- `cargo clippy --all-targets` + `cargo fmt --check` чистые;
- после рестарта процесса правила / действия / слоты / rarity / админы / юзеры / сессии /
  конфиг / креденшеллы сохраняются.

## Риски / заметки

- sqlx-cli 0.9.0 == крейс 0.9.0 (мисматч версий даёт несовместимые метаданные);
- cwd при разворачивании `query!` — корень воркспейса: относительные пути БД в макросах
  не работают, скрипт передаёт абсолютный путь;
- `.env`/`.env.dev` gitignored → на CI и свежих клонах макросы идут офлайн через `.sqlx`;

- weight u64 ↔ i64: никаких `unwrap`/паник — конвертация через
  `i64::try_from(weight)` с ошибкой `RepositoryError::Conflict("weight out of range")`.
  Отрицательные отсекаются раньше: serde не парсит `-5` в u64 → API 400.
  Значения выше `i64::MAX` в in-memory легальны — там ограничения нет; паритет-сьют
  гоняется в представимом диапазоне, граница покрыта отдельным sqlite-тестом;
- `RETURNING` требует SQLite ≥ 3.35 (bundled libsqlite3-sys ок);
- кириллические сиды в миграциях — следить за кодировкой (UTF-8 без BOM);
- паритет-сьюты должны учитывать, что у sqlite сиды приходят из миграции
  (для чистых CRUD-тестов удалять сиды или использовать свои данные).
