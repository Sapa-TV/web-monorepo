# План: config → StaticConfig + RuntimeConfig (config repo, load/save, живое обновление)

## Цель

1. Разделить текущий монолитный `Config` (всё в `src/config/common.rs`) на **статический** (инфраструктура, файл/env) и **runtime** (настраиваемое поведение, живёт в repo).
2. Ввести `ConfigRepository` trait + in-memory реализацию (по конвенции остальных repo: `impl Future<RPITIT>`, `RepositoryError`), готовую к sqlite.
3. Загрузка/сохранение настроек из repo. Приоритет: **repo > config.toml (seed)**. Пустой repo → seed из файла + сгенерированный `access_key` → сохранить в repo.
4. **Живое обновление**: настройки из repo читаются на лету (без рестарта), в первую очередь `access_key` (ротация через root-эндпоинт). Ранее замороженные `Duration` в `QueueService`/`SessionService` перестают быть замороженными.
5. Убрать `refresh_token` из конфига полностью — credentials уже живут в `PlatformCredentialRepository` (единственный источник).
6. Убрать `access_key` из статического конфига — читается только из repo.

## Контекст (текущее состояние)

- `src/config/common.rs` — `Config` (Clone, Deserialize, `#[serde(default)]`, `load()` из config.toml + env):
  `roulette_timeout_secs`, `retention_secs`, `queue_cleanup_interval_secs`, `sessions_cleanup_interval_secs`, `queue_default_limit`, `port`, `access_key`, `cors_origins`, `twitch: Option<Arc<TwitchConfig>>`, `admin_twitch_id`, `session_ttl_secs`, `cookie_secure`. `load()` паникует, если `access_key` пуст. `cfg(test) test_config()`.
- `src/config/twitch.rs` — `TwitchConfig { client_id, client_secret, refresh_token, broadcaster_id, redirect_uri, csrf_ttl_secs }`, `build()` с валидацией (required + `csrf_ttl_secs > 0`), ручной `Deserialize`.
- `src/config.rs` — `pub mod common; pub mod twitch;` re-export `Config`, `TwitchConfig`.
- `src/error/config.rs` — `ConfigError { MissingField, InvalidCsrfTtl }`.
- Потребители `Config` (через `state.config`):
  - `state.rs` (builder: roulette_timeout/retention → `QueueService`, admin_twitch_id → seed, session_ttl → `SessionService`, twitch → `AdminAuthService`);
  - `main.rs` (twitch → ingress, cors_origins → CORS, port → bind, queue_cleanup_interval/sessions_cleanup_interval → background-таски);
  - `api/auth.rs:33`, `api/ws.rs:45` — `access_key` constant-time сравнение;
  - `api/admin.rs:118` — возвращает `access_key` (PAK);
  - `api/session.rs` — `cookie_secure` (cookie), `session_ttl_secs` (Max-Age);
  - `api/queue.rs:766` — `queue_default_limit`.
- Сервисы, заморозившие Duration: `QueueService.timeout`/`retention` (поля в struct, `timeout()`), `SessionService.session_ttl`.
- `PlatformCredentialRepository` (`src/platform.rs`) уже хранит refresh_token (twitch `PlatformId::TWITCH`); `ingress/twitch_auth.rs::current_refresh_token()` имеет **fallback на `config.refresh_token`** — его убрать.
- Конвенции: `#[non_exhaustive]`, `std::sync::nonpoison::{Mutex,RwLock}` (feature-gated в `lib.rs`), RPITIT в трейтах, `#[serde(default)]`, `thiserror` в `src/error/*`, `#[cfg(test)]` тесты под модулем, `cargo test`/`clippy`/`fmt`.

## Решения

- **Два типа конфига**: `StaticConfig` (инфраструктура, file/env) и `RuntimeConfig` (поведение + `access_key`, в repo).
- **`refresh_token` удаляется из `TwitchConfig`** — единственный источник `PlatformCredentialRepository`. `current_refresh_token()`: репо пусто → `PlatformError::Auth("… not configured")` (без fallback).
- **`access_key` только в repo**: не читается из env/файла. При пустом repo генерируется случайно (rand, как `nonce()` в `session/service.rs`), кладётся в `RuntimeConfig` и сохраняется. Смена — root-эндпоинт через `ConfigStore::rotate_access_key()`.
- **Файл как seed**: `StaticConfig::load()` читает file+env один раз и отдаёт `(StaticConfig, Option<RuntimeConfig>)` (runtime-часть = seed-поля из config.toml, `access_key` пустой). Если repo непуст — repo выигрывает, seed не применяется.
- **Живой сторедж `ConfigStore<R: ConfigRepository>`**: `static_cfg: Arc<StaticConfig>` + `current: RwLock<RuntimeConfig>` + `repo: Arc<R>`. Чтение — клонирование snapshot-полей на каждый запрос (`access_key()` и т.п.); запись — `validate` → `repo.save` → применить в память (только после успешного персиста).
- **`QueueService`/`SessionService`** получают `SharedSettings` (`Arc<RwLock<RuntimeConfig>>`) вместо замороженных `Duration`; читают текущие значения на каждый вызов (`mark_timed_out`, `purge_expired`, `issue_session`).
- **Background-таски очистки** читают интервал из `source()` каждую итерацию (sleep-loop вместо `time::interval`) → интервалы меняются на лету.
- `ConfigStore` — generic-параметр `K: ConfigRepository` в `UniAppState` (полностью по конвенции остальных repo; alias `AppState` биндит `InMemoryConfigRepository`).

## Разбивка настроек

### Статик (`StaticConfig`, config.toml/.env) — постоянные, инфраструктура

- `port`
- `cors_origins`
- `cookie_secure`
- `admin_twitch_id` (bootstrap seed)
- `twitch`: `client_id`, `client_secret`, `redirect_uri`, `broadcaster_id`, `csrf_ttl_secs`
  - ~~`refresh_token`~~ — удалить (живёт в `PlatformCredentialRepository`)

### В repo (`RuntimeConfig`, живые)

- `access_key` (seed: генерируется при первом старте; ротация через root-эндпоинт)
- `roulette_timeout_secs`
- `retention_secs`
- `queue_cleanup_interval_secs`
- `sessions_cleanup_interval_secs`
- `queue_default_limit`
- `session_ttl_secs`
- ~~`access_key` в статике~~ — нет, только repo

## Разбиение на части (порядок реализации)

> Каждая часть — отдельный коммит: компилируется, тесты зелёные, `clippy` чистый. Потребители старого `Config` не трогаются, пока не наступит Часть 3.

Реализацию делаем по частям последовательно, по факту завершения каждой — верификация (см. раздел «Верификация»).

### Часть 1. Фундамент: RuntimeConfig + ConfigRepository + InMemoryConfigRepository (без изменений потребителей)

Совершенно не влияет на существующий код — новые типы и тесты.

**Шаги:**

1. `src/error/config.rs` — расширить `ConfigError`: `InvalidAccessKey`, `InvalidValue { field: &'static str }`.
2. `src/config/runtime.rs` (новое) — `RuntimeConfig`:
   - поля: `access_key`, `roulette_timeout_secs`, `retention_secs`, `queue_cleanup_interval_secs`, `sessions_cleanup_interval_secs`, `queue_default_limit`, `session_ttl_secs`;
   - `Clone + Serialize + Deserialize + PartialEq + Debug`, `#[non_exhaustive]`;
   - `Default` = текущие дефолты из `Config::default()` (tuning), `access_key: String::new()`;
   - `validate(&self) -> Result<(), ConfigError>`: `access_key` непустой, все интервалы/ttl/limit > 0;
   - тесты: serde roundtrip, default→InvalidAccessKey, валид принят, нулевые значения отклонены.
3. `src/config/repository.rs` (новое) — trait:

   ```rust
   pub trait ConfigRepository: Send + Sync {
       fn load(&self)
           -> impl Future<Output = Result<Option<RuntimeConfig>, RepositoryError>> + Send;
       fn save(&self, config: &RuntimeConfig)
           -> impl Future<Output = Result<(), RepositoryError>> + Send;
   }
   ```

   `load()` возвращает `Option` — пустой repo = настроек нет → фолбэк на seed.

4. `src/db/inmemory_config.rs` (новое) — `InMemoryConfigRepository { config: nonpoison::Mutex<Option<RuntimeConfig>> }`, `new()`, `Default`; реализация `ConfigRepository` (`load` → клон под локом; `save` → `*lock = Some(config.clone())`); тесты: load пустой → `None`, roundtrip, перезапись.
5. `src/db.rs` — `pub mod inmemory_config;`.
6. `src/config.rs` — новый набор модулей/реэкспортов (см. Часть 2, шаг 5).

**Проверка:** `cargo test -p backend`, `clippy`, `fmt` — существующие тесты зелёные (ничего не сломано).

### Часть 2. Новые типы: StaticConfig + ConfigStore, удаление `refresh_token` (старый `Config` живёт)

Старый `Config` (файл `common.rs`) пока остаётся рабочим и используется потребителями — его миграция в Части 3. `RuntimeConfig`/`ConfigRepository` из Части 1 уже доступны.

**Шаги:**

1. `src/config/static.rs` (rename `common.rs`) + split seed:
   - `pub struct StaticConfig { port, cors_origins, cookie_secure, admin_twitch_id, twitch: Option<Arc<TwitchConfig>> }` (`Clone`, `Deserialize`, `#[serde(default)]`, `Default`);
   - `pub fn load() -> (StaticConfig, Option<RuntimeConfig>)`: читает config.toml + env как раньше (dotenvy + `::config::Config` builder); deserialize во внутренний `Raw` (все поля, включая runtime-seed), split → `StaticConfig` + `RuntimeConfig` (seed, `access_key` пустой); убрать `panic!` про ACCESS_KEY;
   - `cfg(test) test_config()` — только статика.
2. `src/config/twitch.rs` — убрать `refresh_token`:
   - удалить поле из struct, `build()`, `Raw`-deserialize, тест-фикстур (`twitch_json()` без `refresh_token`);
   - `ConfigError::MissingField { field }` — списки `required` не трогать;
   - `src/ingress/twitch_auth.rs::current_refresh_token()` — убрать fallback на `config.refresh_token` (репо пусто → `PlatformError::Auth("… not configured")`); тесты: удалить fallback-тест, добавить «пустой repo → error».
3. `src/config/store.rs` (новое) — `ConfigStore`:

   ```rust
   pub struct SharedSettings(Arc<nonpoison::RwLock<RuntimeConfig>>); // Clone

   pub struct ConfigStore<R: ConfigRepository> {
       static_cfg: Arc<StaticConfig>,
       current: Arc<nonpoison::RwLock<RuntimeConfig>>, // = SharedSettings
       repo: Arc<R>,
   }
   ```

   - `new(static_cfg, runtime, repo)`, геттер `source()` → `SharedSettings`;
   - акцессоры (клонируют snapshot-поле): `access_key()`, `queue_default_limit()`, `session_ttl_secs()`, `roulette_timeout_secs()`, `retention_secs()`, `queue_cleanup_interval_secs()`, `sessions_cleanup_interval_secs()`, `cookie_secure()`, `cors_origins()` (аналог старого trimmed-десериализатора), `port()`, `admin_twitch_id()`, `twitch()`;
   - `async fn update_runtime(&self, next: RuntimeConfig) -> Result<(), ConfigError>`: `validate()` → `repo.save(&next)` → `*current.write() = next`;
   - `async fn rotate_access_key(&self, key: &str) -> Result<(), ConfigError>`: клон current → заменить `access_key` → `validate` → `repo.save` → применить (частичное обновление);
   - тесты: инвалид не применяется и repo не тронут; `update_runtime` персистит и виден через `source()`; `rotate_access_key` меняет только key.

4. `src/config.rs` — `pub mod static; pub mod runtime; pub mod repository; pub mod store; pub mod twitch;` (модуль `common` временно остаётся), re-export `StaticConfig`, `RuntimeConfig`, `ConfigRepository`, `ConfigStore`, `SharedSettings`, `TwitchConfig`.
5. `src/config/common.rs` — НЕ удалять до Части 3 (старый `Config` ещё в работе).

**Проверка:** `cargo test -p backend`, `clippy`, `fmt` — старый код работает, новые типы покрыты тестами.

### Часть 3. Миграция потребителей на ConfigStore, живые настройки, удаление `Config`

Точка, где меняется вайринг и все `state.config.*`. Делать целиком (переходные половинчатые состояния не компилируются).

**Шаги:**

1. `src/state.rs`:
   - `UniAppState` — добавить generic `K: ConfigRepository`, поле `config: Arc<Config>` → `config: Arc<ConfigStore<K>>`; обновить bounds + `Clone`; `AppState` alias добавить `InMemoryConfigRepository`;
   - `AppStateBuilder::new(random, config_store: Arc<ConfigStore<InMemoryConfigRepository>>, credentials_repo)` (вместо `config: &Arc<Config>`);
   - `build()`: `admin_service.seed(store.admin_twitch_id()…)`; `AdminAuthService::new(store.twitch(), …)`; сервисы получают `store.source()` (SharedSettings).
2. `src/queue/service.rs` — живые настройки:
   - поле `timeout: Duration`/`retention: Duration` → `settings: SharedSettings`;
   - `mark_timed_out()`: cutoff из `settings.read().roulette_timeout_secs` на каждый вызов;
   - `purge_expired()`: то же для `retention_secs`;
   - `timeout()` — чтение `settings` вместо замороженного Duration.
3. `src/session/service.rs` — `issue_session()` использует `session_ttl_secs` из `settings`.
4. `src/api/*` — чтение из стореджа:
   - `api/auth.rs:33`, `api/ws.rs:45`: `state.config.access_key()` → `.as_bytes()` для `ct_eq`;
   - `api/session.rs`, `api/queue.rs:766`: `state.config.session_ttl_secs()` / `cookie_secure()` / `queue_default_limit()`.
5. `src/main.rs` — boot-флоу:

   ```rust
   let (static_cfg, file_seed) = StaticConfig::load();
   let repo = Arc::new(InMemoryConfigRepository::new());
   let runtime = if let Some(r) = repo.load().await? {
       r
   } else {
       let seed = file_seed.unwrap_or_default();
       seed.access_key = generate_access_key(); // rand, как nonce()
       repo.save(&seed).await?;
       seed
   };
   let store = Arc::new(ConfigStore::new(Arc::new(static_cfg), runtime, repo));
   ```

   Вайринг: CORS/port/twitch через сторедж; background-таски — sleep-loop с интервалом из `store` (каждый цикл).

6. `src/test_fixtures.rs` — `test_state()`: `ConfigStore<InMemoryConfigRepository>` с `test_runtime("test-key")`; обновить всех потребителей `Config::test_config()`.
7. Удалить `src/config/common.rs` и `pub mod common;` из `src/config.rs`.

**Проверка:** весь бэкенд компилируется, все существующие тесты зелёные (`/api/admin/pak` снова отдаёт `"test-key"`).

### Часть 4. Ротация `access_key` + чистка

**Шаги:**

1. `src/api/admin.rs` — `POST /api/admin/pak` (root-only, через `require_root`), без body → новый PAK генерируется случайно (`generate_secret()`, как при boot-seed) через `ConfigStore::rotate_access_key_generated()`; ответ — новый PAK. OpenAPI: `AdminApiDoc` (`paths`) + путь в `root_router`.
2. Тесты ротации: root меняет → новый работает, старый отклоняется в `require_auth`; non-root → forbidden; значение персистится в repo.
3. Чистка: `.env.example` — убрать `ACCESS_KEY` (и в комментариях документации при необходимости).
4. `backlog.md` — отметить выполненными пункты: разделение config, ACCESS_KEY→repo+root-эндпоинт, refresh_token в `PlatformCredentialRepository`.

## Тесты (план — добавить/обновить при реализации)

- `RuntimeConfig`: serde roundtrip; `validate()` (пустой `access_key`, нулевой ttl/интервал → ошибка).
- `InMemoryConfigRepository`: load пустой → `None`; roundtrip; перезапись.
- `ConfigStore`: `update_runtime` применяет + персистит; инвалид → repo не тронут; `rotate_access_key` меняет только key и виден через `source()`.
- Boot-seed: пустой repo → access_key сгенерирован + сохранён; непустой repo → repo выигрывает, seed не применяется.
- `twitch_auth`: репо — единственный источник (fallback удалён); пустой repo → `PlatformError::Auth("… not configured")`; удалить старый тест fallback.
- Ротация PAK через endpoint: root меняет (ключ генерируется случайно) → новый работает, старый отклоняется в `require_auth`; non-root → forbidden; значение персистится в repo.
- Существующие api-тесты: `/api/admin/pak` продолжает отдавать `"test-key"`.
- `QueueService`/`SessionService`: изменение настройки через `SharedSettings` видно следующему вызову.

## Верификация

- `cargo build --package backend`
- `cargo test --package backend`
- `cargo clippy --package backend --all-targets` (deny-линты: `clippy::exhaustive_structs`, `clippy::new_ret_no_self`)
- `cargo fmt --check`
- `just lint` (ast-grep)

## Будущее: sqlite

- Таблица `settings`/`config` (одна строка): JSON или отдельные колонки под поля `RuntimeConfig` (или `key TEXT PRIMARY KEY, value`).
- Новая реализация `ConfigRepository` (sqlite) подменяет `InMemoryConfigRepository` в вайринге — сервисы и `ConfigStore` не меняются (generic над трейтом).

## Открытые вопросы

- Генерировать ли `access_key` всегда при первом старте (уже решено — да, без env-фолбэка): сгенерированный ключ первого старта теряется, если не вызвать ротацию до рестарта с пустыми данными — приемлемо, это «не настроено».
- Валидация runtime-полей: только верхние границы/>0, без магических лимитов (YAGNI).
- `ConfigStore::update_runtime` (полный патч) пока не имеет HTTP-эндпоинта — добавим вместе с админкой настроек (вне этого плана), сейчас хватает `rotate_access_key`.
