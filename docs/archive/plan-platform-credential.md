# План: PlatformCredentialRepository и унификация Platform/PlatformId (убрать PlatformKind)

## Цель

1. Заменить узкий `TwitchTokenRepository` (добавлен ранее в `ingress/twitch_auth.rs`) на общий `PlatformCredentialRepository` — одна сущность для всех платформ (twitch/youtube/vk_video_live), готовая к sqlite.
2. Убрать дублирование концепций платформы: сейчас в коде **две параллельные сущности**:
   - `PlatformKind` (enum в `ingress/event.rs`) — для ингресса/событий;
   - `Platform { id: PlatformId, name: String }` (`src/platform.rs`) — DB-концепция, сиды `1=twitch, 2=youtube, 3=vk_video_live`.
     Убираем `PlatformKind`, единой сущностью становится `Platform`/`PlatformId`.
3. При переходе на sqlite: таблица `platform` (соответствует текущему `PlatformRepository`), `platform_credentials` ссылается на неё через `platform_id` FK. Сиды обеих таблиц задаются из одних констант.

## Контекст (текущее состояние)

- `src/ingress/twitch_auth.rs`:
  - `pub trait TwitchTokenRepository: Send + Sync` — `load()` / `save(&str)` / `clear()`, все async, `Result<_, RepositoryError>`;
  - `TwitchAuthService<R: TwitchTokenRepository>` — generic над этим репозиторием, `new(config, token_repo: Arc<R>)`;
  - методы `current_refresh_token()`, `persist_rotated()` делают `.await` на репо.
- `src/db/inmemory_twitch_auth.rs` — `InMemoryTwitchTokenRepository` (`Mutex<Option<String>>`, секст `seeded()`), зарегистрирован в `src/db.rs`.
- `src/admin/auth.rs` — `AdminAuthService<R: TwitchTokenRepository>`, generic, `new(config, token_repo)`; `complete()` сохраняет refresh_token, `is_ingress_credentials_configured()` / `revoke_ingress_credentials()` — async.
- `src/ingress/twitch.rs` — `TwitchPlatformService<R: TwitchTokenRepository>` (поле `auth: Arc<TwitchAuthService<R>>`, сами вызовы через `PlatformKind::Twitch`), `PlatformService::kind() -> PlatformKind`.
- `src/ingress/event.rs` — `PlatformEvent.platform: PlatformKind`; `PlatformKind::{Twitch, YouTube, VkVideoLive}` (Copy, serde snake_case, `as_name()`).
- `src/platform.rs` — `PlatformId(u32)` (transparent, ToSchema, `pub(crate) const fn new`), `Platform { id, name }` (Debug/Clone/PartialEq, `#[non_exhaustive]`), `PlatformRepository` (`find_by_name`, `load_all`).
- `src/db/inmemory_platform.rs` — `SEEDED = [(1,"twitch"),(2,"youtube"),(3,"vk_video_live")]`, `new_seeded()`.
- Вайринг: `state.rs` (builder получает `token_repo: Arc<InMemoryTwitchTokenRepository>`, отдаёт в `AdminAuthService`), `main.rs` (создаёт общий `token_repo`, передаёт в builder И в `TwitchPlatformService` — админ пишет, ингресс читает), `test_fixtures.rs`.
- Где используется `PlatformKind`: `ingress/event.rs`, `ingress/platform.rs`, `ingress/service.rs` (tests), `ingress/twitch.rs`, `main.rs` (`service.kind().as_name()`).
- Конвенции: `#[non_exhaustive]`, `std::sync::nonpoison::Mutex`, RPITIT в трейтах, `thiserror` в `src/error/*`, тесты `#[cfg(test)]` под модулем, `cargo clippy`/`cargo test`/`cargo fmt`.

## Решения

- **Ключ репозитория кредов — `PlatformId`** (не text, не enum). Отдельная `PlatformEnum`-таблица не нужна — она уже существует как `Platform`. В sqlite: `platform_credentials(platform_id INTEGER NOT NULL UNIQUE REFERENCES platform(id), refresh_token TEXT NOT NULL)`.
- **`PlatformKind` удаляется**. События несут только `PlatformId`, сиды и маппинг id↔name — один источник истины в `platform.rs` (константы `PlatformId::TWITCH/YOUTUBE/VK_VIDEO_LIVE` + `PlatformId::name()`).
- Текстовый формат в БД/событиях: имя платформы ("twitch") генерируется из `PlatformId::name()`, поэтому опечатки исключены на уровне API.
- Формат JSON события меняется: `event.platform` из `"twitch"` станет числом `1` (transparent serde `PlatformId`); имя на проводе не передаётся — резолвится через `PlatformId::name()`.

## Шаги

### 1. `src/platform.rs` — единый источник истины

- `PlatformId`: константы

  ```rust
  impl PlatformId {
      pub const TWITCH: PlatformId = PlatformId::new(1);
      pub const YOUTUBE: PlatformId = PlatformId::new(2);
      pub const VK_VIDEO_LIVE: PlatformId = PlatformId::new(3);

      pub const fn name(self) -> &'static str { /* "twitch" | "youtube" | "vk_video_live" */ }
  }
  ```

- `Platform`:
  - добавить `Serialize`/`Deserialize` (нужно для событий через WS);
  - `pub fn from_id(id: PlatformId) -> Platform` → `Platform::new(id, id.name())`;
  - `pub fn as_name(&self) -> &'static str` (делегирует `id.name()`) / `Display` — для логов и `main.rs`.
- Новый трейт:

  ```rust
  pub trait PlatformCredentialRepository: Send + Sync {
      fn load_credential(&self, platform: PlatformId)
          -> impl Future<Output = Result<Option<String>, RepositoryError>> + Send;
      fn save_credential(&self, platform: PlatformId, credential: &str)
          -> impl Future<Output = Result<(), RepositoryError>> + Send;
      fn clear_credential(&self, platform: PlatformId)
          -> impl Future<Output = Result<(), RepositoryError>> + Send;
  }
  ```

### 2. Сиды — `src/db/inmemory_platform.rs`

- Убрать `(u32, &str)` кортежи; сидить из констант:
  `[(PlatformId::TWITCH, PlatformId::TWITCH.name()), …]` (или массив `Platform::from_id(..)`).
- Тесты не ломать (id/имена прежние).

### 3. Новый `src/db/inmemory_platform_credential.rs`

- `InMemoryPlatformCredentialRepository` на `std::sync::nonpoison::Mutex<HashMap<PlatformId, String>>`:
  - `load_credential(id) -> Ok(repo.get(id).cloned())`;
  - `save_credential(id, credential)` → `insert(id, credential.trim())`;
  - `clear_credential(id)` → `remove(id)`.
- `new()`, `Default`, `seeded(...)` по аналогии.
- Тесты: roundtrip, clear, независимость ключей (twitch ≠ youtube).
- `src/db.rs`: добавить `pub mod inmemory_platform_credential;` (модуль `inmemory_twitch_auth` пока остаётся — его удаление переносится в шаг 7).

### 4. `src/ingress/event.rs` — убрать `PlatformKind`

- Удалить enum `PlatformKind` и его `as_name()`.
- `PlatformEvent.platform: PlatformId` (Copy, без клонов на хот-пате; имя доступно через `PlatformId::name()`); `PlatformEvent::chat_message(platform: PlatformId, …)`.
- `PlatformEventPayload::ChatMessage` — без изменений.
- Тесты: заменить `PlatformKind::*` на `PlatformId::TWITCH/etc`; отдельный serde-тест платформы не нужен — реальный формат события проверяется в шаге 8.

### 5. Ингресс

- `src/ingress/platform.rs`:
  - `PlatformService::kind(&self) -> PlatformKind` → `fn platform(&self) -> Platform`.
- `src/ingress/twitch.rs`:
  - `TwitchPlatformService<R: PlatformCredentialRepository>`; поле `platform: PlatformId` (Copy), инициализируется `PlatformId::TWITCH` в `new()`;
  - `chat_event_from` / `consume_loop` принимают `PlatformId` (без клонов); `PlatformService::platform()` → `Platform::from_id(self.platform)`;
  - `use crate::platform::{Platform, PlatformCredentialRepository, PlatformId}` вместо `PlatformKind`.
- `src/ingress/twitch_auth.rs`:
  - импорт трейта сменить на `platform::PlatformCredentialRepository`;
  - `TwitchAuthService<R: PlatformCredentialRepository>`;
  - `new(config, token_repo: Arc<R>)`;
  - `current_refresh_token()` → `token_repo.load_credential(PlatformId::TWITCH)`;
  - `persist_rotated()` → `token_repo.save_credential(PlatformId::TWITCH, …)`;
  - тесты — `InMemoryPlatformCredentialRepository`.
- `src/ingress/service.rs`: тесты — `PlatformId::TWITCH/YOUTUBE`, сравнения `received.platform == platform`.

### 6. Админ

- `src/admin/auth.rs`:
  - `AdminAuthService<R: PlatformCredentialRepository>`, `new(config, token_repo)`;
  - `complete()` → `token_repo.save_credential(PlatformId::TWITCH, …)`;
  - `is_ingress_credentials_configured()` → `token_repo.load_credential(PlatformId::TWITCH)`;
  - `revoke_ingress_credentials()` → `clear_credential(PlatformId::TWITCH)`;
  - тесты — `InMemoryPlatformCredentialRepository`.

### 7. Вайринг

> `token_repo` переименован в `credentials_repo` (во всём коде).

- `src/state.rs`:
  - generic-параметр `T: PlatformCredentialRepository`, поле `admin_auth: Arc<AdminAuthService<T>>`;
  - `AppState` alias — `InMemoryPlatformCredentialRepository`;
  - `AppStateBuilder::new(random, config, credentials_repo: Arc<InMemoryPlatformCredentialRepository>)`.
- `src/main.rs`:
  - `credentials_repo = Arc::new(InMemoryPlatformCredentialRepository::new())`, передать в builder и в `TwitchPlatformService::new(twitch.clone(), Arc::clone(&credentials_repo))`;
  - лог: `service.platform().as_name()`.
- `src/test_fixtures.rs`: `Arc::new(InMemoryPlatformCredentialRepository::new())`.
- После замены всех использований удалить старый модуль: файл `src/db/inmemory_twitch_auth.rs` и `pub mod inmemory_twitch_auth;` из `src/db.rs`.
- Проверить `bas src/api/*` — `State<AppState>` ссылает тип через alias, менять не должно понадобиться (кроме проверки `api/ws.rs`).

### 8. Проверка `src/api/ws.rs`

- Тесты сериализации событий над WS: если сравнивают `event.platform` как строку — обновить на новую форму (число `PlatformId`, напр. `1`).

### 9. Верификация

- `cargo build --package backend`
- `cargo test --package backend`
- `cargo clippy --package backend --all-targets` (deny-линты: `clippy::exhaustive_structs`, `clippy::new_ret_no_self`)
- `cargo fmt --check`

## Будущее: sqlite

- Таблица `platform(id INTEGER PK, name TEXT NOT NULL UNIQUE)` — сиды из тех же констант `PlatformId::{TWITCH,.name()}`; `PlatformRepository` получает sqlite-реализацию (та же сигнатура).
- Таблица `platform_credentials(platform_id INTEGER NOT NULL UNIQUE REFERENCES platform(id) ON DELETE CASCADE, refresh_token TEXT NOT NULL)`.
- Новая реализация `PlatformCredentialRepository` (sqlite) подменяет `InMemoryPlatformCredentialRepository` в вайринге — сервисы не меняются (generic над трейтом).

## Открытые вопросы

- Нужен ли `PlatformCredentialService` (тонкая обёртка над репо) — пока репо в одиночку покрывает потребности, сервис = лишняя прослойка (YAGNI). Добавить, если появится логика вроде «отдать статус по всем платформам».
- Решено: в событиях хранится только `PlatformId` (Copy, лёгкий на проводе). Имя не передаётся — резолвится из константы `PlatformId::name()`. Полный `Platform` строится на месте (`Platform::from_id`) только там, где нужен (напр., `PlatformService::platform()`).
