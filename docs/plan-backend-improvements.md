# Backend — план улучшений / рефакторинга / оптимизации

## Приоритеты

Порядок работ: **1 → 3 → 4** (баги/безопасность/наблюдаемость, низкий риск), затем **п. 8** (WS-auth → WS-complete), далее **2** (перед миграцией на sqlx).

> Статус: 1, 3, 4, 7, п. 8 (WS-auth handshake + WS-диспетчер complete), п. 2 (generics + `RarityService`), п. 5 (чистка кода), п. 6 (пагинация/retention), п. 4 (конфиг из `config.toml`) и п. 9 (единообразие сервисного слоя: `QueueService` + `UserService`) — **сделано**. План закрыт.

---

## 1. Корректность / гонки

### 1.1 `dequeue_next` не атомарен

`queue/service.rs:46-92` — «проверка активного» (`list(Spinning)`) и `dequeue_next()` — два разных захвата лока.

Проблемы:

- Два параллельных `POST /api/queue/next` могут оба пройти проверку → **двойной спин** (два entry в `Spinning` одновременно), либо один получит `QueueEmpty` (404) вместо `409`.
- Если `roll()` вернёт `NoSlots`, entry остаётся `Spinning` без `result_slot_id` (орфан; лечится только timeout-задачей).

Решение: выбор entry + roll + запись `result_slot_id` внутри одного лока/транзакции; `AlreadyActive` — результат конкурентного выбора, а не предпроверка.

**Сделано:** `QueueRepository::dequeue_next_with_slot(slot_id)` — выбор entry + установка `Spinning` + запись `slot_id` в одном захвате лока репо (`DequeueOutcome::Picked/AlreadyActive/Empty`). `QueueService::dequeue_next` сначала `roll()`, затем атомарно выставляет статус. Орфана нет: `NoSlots` — до записи статуса.

### 1.2 CAS-семантика статусов

`complete`/`cancel` (`queue/service.rs:95-142`) — get-then-update. В гонке двойной `complete` пройдёт (оба прочитают `Spinning`).

Решение: `update_status` внутри репо проверяет ожидаемый статус (compare-and-swap) и возвращает конфликт.

**Сделано:** `update_status_if(id, expected, status)` → `StatusUpdateOutcome::Updated/NotFound/StatusMismatch`. `complete` требует `Spinning` (гот. гонка → один успешен, другой `NotSpinning`); `cancel` допускает `Pending|Error`.

### 1.3 `mark_timed_out` vs `dequeue`

Timeout-задача может перевести в `Error` entry, который параллельно декивается. Закрыть тем же локом/транзакцией.

**Сделано:** покрыто дизайном из 1.1 — `mark_timed_out` и `dequeue_next_with_slot` используют один и тот же лок репо, гонки нет. Логически не пересекаются: timeout трогает только `Spinning`, деqueue — только `Error`/`Pending`. Для sqlite (п. 2) потребуется транзакция.

### 1.4 Case-sensitive статус в query

`QueueStatus` сериализуется как есть (`Pending`/`Spinning`), а фронтенд по плану шлёт `?status=pending` → `400`. Либо `#[serde(rename_all = "lowercase")]`, либо слать `Pending`.

**Сделано:** фронтенд (`panel.js`) читает статусы только из JSON-ответа, где важно `Spinning` (uppercase) — сериализацию не меняли. Вместо этого кастомный `Deserialize` для `QueueStatus`: query-параметр принимается case-insensitive (`pending`/`Pending`/`SPINNING`). Serialize остаётся uppercase. Тест `list_status_query_is_case_insensitive`.

---

## 2. Подготовка к sqlite (ключевое архитектурное решение)

- **`impl Future` в трейтах репозиториев блокирует `dyn`** (queue/repository.rs:9, roulette/repository.rs:7, user/repository.rs:7, roulette/rarity.rs:54). Трейты не object-safe → нельзя `Arc<dyn QueueRepository>`.

**Сделано (решение: generics, без dyn):** `QueueService<Q, R, S>` и `UniAppState<Q, R, U, P, S>` параметризованы трейтами репозиториев (Q: QueueRepository, R: RarityRepository, U: UserRepository, P: PlatformRepository, S: RouletteSlotRepository). Ручной `Clone` (без лишних bounds на репо). API-слой продолжает использовать `AppState` — это алиас `type AppState = UniAppState<InMemory...>` (`state.rs`), единственная точка замены на sqlite-репо. Трейты остались на `impl Future` (не object-safe) — для generics это не нужно.

- `QueueService` завязан на конкретные `Arc<InMemoryQueueRepository>` / `Arc<InMemoryRarityRepository>` (queue/service.rs:22-24). Типы абстрагировать.

**Сделано:** `QueueService` теперь generic над репо; все зависимости (репо, rarity-service, roulette) передаются в конструктор.

- **Асимметрия: у слотов есть кэш-сервис, у rarities нет.** `list_rarities` и CRUD ходят в репо напрямую (api/rarities.rs:69) — при sqlite это диск на каждый запрос. Сделать `RarityService` с кэшем по аналогии с `RouletteSlotService` (или пересмотреть, нужен ли кэш).

**Сделано:** `RarityService<R>` (`roulette/rarity_service.rs`) — write-through кэш (load_all при build, save/update/delete обновляют кэш), по аналогии с `RouletteSlotService`. `list_rarities`/CRUD ходят в `state.rarity_service`; `QueueService.dequeue_next` берёт display_name редки из кэша (`get_by_id`) вместо `load_all()` на каждый спин. Добавлен blanket-impl `impl<T: RarityRepository> RarityRepository for Arc<T>` (как у слотов).

---

## 3. Безопасность

- **Сравнение токена не constant-time** (api/auth.rs:24): `token == Some(...)` → timing-атака. Заменить на постоянновременное сравнение (например `subtle`).

**Сделано:** `subtle::ConstantTimeEq` для сравнения токена в `require_auth`.

- `CorsLayer::permissive()` (main.rs:46) — для прод ограничить origin'ы.

**Сделано:** `CORS_ORIGINS` (env, через запятую) → `CorsLayer` с `allow_origin`/`allow_methods`/`allow_headers`. Если env не задан — остаётся `permissive` (для локалки).

---

## 4. Наблюдаемость и сервер

- Нет `TraceLayer` (запросное логирование), нет `/health`, нет graceful shutdown (`SIGTERM` → `with_graceful_shutdown`).

**Сделано:** `TraceLayer::new_for_http()`, `GET /health` (без авторизации), `GET /version` (без авторизации, версия и git hash), `with_graceful_shutdown` (SIGINT/SIGTERM).

- `roulette_timeout_secs` захардкожен `10` (config.rs:15) — **не** выносить в env (не хотим кучу env-переменных, только то, что реально настраивается). Позже — загрузка конфига из json/yaml-файла через крейт `config` (переезд в п. 2 при sqlite-миграции).

**Сделано:** крейт `config` (добавлен через `cargo add` + `cargo autoinherit`). В файл `config.toml` вынесены только захардкоженные ранее параметры: `roulette_timeout_secs`, `retention_secs`, `queue_default_limit`. `ACCESS_KEY`/`PORT`/`CORS_ORIGINS` остались в env (дефолты в коде, env переопределяет). Порядок источников: дефолты в коде (`Default` + `#[serde(default)]`) → `config.toml` (optional) → env. Кастомный десериализатор для `cors_origins` (массив из файла ИЛИ comma-строка из env). `Config::load()` паникует, если `access_key` пуст.

---

## 5. Чистка кода

- `unreachable!()` в `From<QueueServiceError> for ApiError` (error/queue.rs:41-44) — реструктурировать.
- `NaiveDateTime` → `DateTime<Utc>` (timezone-однозначность).
- Повторяющиеся generic-типы `RouletteService<StandartRandomProvider, Arc<InMemoryRouletteSlotRepository>>` (queue/service.rs:24, state.rs) — type alias'ы.
- `StandartRandomProvider` — опечатка в имени; `rand` уже в зависимостях — можно упростить.

**Сделано:** `unreachable!()` убран — `From` реструктурирован в полностью исчерпывающий match без раннего `return` (сообщение берётся до match, `Repo` делегируется `ApiError::from(re)`). `NaiveDateTime` → `DateTime<Utc>` во всех доменных типах (`QueueEntry`, `User`), в `QueueRepository::mark_timed_out` и репо-реализациях (`Utc::now()` без `.naive_utc()`, на границе API `.and_utc()` убран). Type alias `Roulette<R> = RouletteService<StandartRandomProvider, Arc<R>>` в `queue/service.rs`. `StandartRandomProvider`: имя **не** переименовывали (решение принято ранее), уже использует `rand` (`rand::rng().random()`) — упрощать нечего.

---

## 6. Рост данных

- Очередь никогда не чистится, `/api/queue` без пагинации (api/queue.rs). Добавить лимит/курсор + retention для `Completed`/`Cancelled`.

**Сделано (keyset по `id`, выбран на обсуждении; queue маленькая, но данные живые — offset давал бы дубли/сдвиги):**

- `GET /api/queue?limit&cursor` → `QueueListResponse { entries, next_cursor }`. Keyset по возрастанию `id` (FIFO), `next_cursor = null` когда страница последняя. `QueueService::list(status, cursor, limit)` возвращает `QueuePage`, лимит клампится `[1, 100]`.
- Retention по времени: `QueueRepository::purge_completed_cancelled(cutoff)` чистит `Completed`/`Cancelled` старше cutoff; `QueueService::purge_expired()` вызывается в таймаут-задаче (main.rs) вместе с `mark_timed_out`.
- Конфиг: `retention_secs` (по умолчанию 24ч) и `queue_default_limit` (20) — в коде (`config.rs`), без env, как решили.
- Тесты: репо (`list_is_paginated_by_keyset_cursor`, `list_filters_by_status`, `purge_removes_only_expired_completed_and_cancelled`, `purge_skips_fresh_completed`), HTTP (`list_is_paginated_with_cursor`). Фронтенд: `panel.js` читает `data.entries`.

---

## 7. Тесты (покрывают пункты 1)

В `apps/backend/src/api/queue.rs`, модуль `tests`:

| Тест                                    | Что проверяет                                     | Покрывает |
| --------------------------------------- | ------------------------------------------------- | --------- |
| `dequeue_next_parallel_only_one_spin`   | два `next` параллельно → один 200, второй 409     | 1.1       |
| `dequeue_next_no_slots_no_orphan`       | `next` без слотов → 422, орфанов в `Spinning` нет | 1.1       |
| `complete_parallel_only_one_success`    | два `complete` параллельно → один 200, второй 409 | 1.2       |
| `slot_created_via_api_used_in_roll`     | созданный через API слот виден в `roll`           | —         |
| `list_status_query_is_case_insensitive` | `?status=pending\|Pending` → 200                  | 1.4       |

`#[ignore]` сняты по мере реализации пунктов 1.1-1.2 (сейчас все сняты).

---

## 8. Принято: подтверждение `complete` по WS (основной канал) + REST (резервный)

Обсуждение 2026-07-31. Решение принято: подтверждение спина идёт по WS, REST остаётся как резервный канал и для тестов.

**Сделано:** first-message handshake `{"type":"auth","token":...}` → `auth_ok`/`auth_err` (constant-time сравнение через `subtle`), после `auth_ok` клиент получает события. WS-диспетчер `handle_message(state, msg) -> ServerMessage` в `api/ws.rs` (`ClientMessage::{Auth,Complete}`, `ServerMessage::{AuthOk,AuthErr,CompleteOk,CompleteErr}`, сериализация через `#[serde(tag="type")]` как у `SpinEvent`). Команда `complete` ходит в `QueueService.complete` (единая точка истины). Тесты: `auth_handshake_validates_token`, `ws_and_rest_complete_are_equivalent` (WS CompleteOk ↔ REST 200 + error-case CompleteErr ↔ 409), `complete_ok_and_err_serialize_with_type_tag`. Frontend (`widget.js`/`panel.js`) отправляет auth-сообщение при подключении; REST-complete остаётся рабочим каналом.

### Обязательный пререквизит

**WS-auth** — закрыть поток событий (`/ws` сейчас без авторизации, п. 3). Без него команды по WS не вводить. Токен в query (`?token=`) утекает в логи — предпочтителен first-message handshake.

### Архитектура: port/adapter (hexagonal)

Ядро — `QueueService.complete` (единая точка истины). Два тонких транспортных адаптера:

- **REST**: `api/queue.rs::complete` — уже есть, ~10 строк.
- **WS**: тонкая async-функция-диспетчер:
  ```rust
  async fn handle_message(state: &AppState, msg: ClientMessage) -> ServerMessage
  ```
  `Complete { entry_id }` → `QueueService.complete(...)` → `ServerMessage::CompleteOk/CompleteErr`. Клей (входящий парсинг + отправка ответа) — в `handle_socket`, не тестируется отдельно.

Мок сокета не нужен: диспетчер — обычная функция, тестируется вызовом напрямую.

### Протокол

```rust
enum ClientMessage { Complete { entry_id } }
enum ServerMessage { CompleteOk { entry_id }, CompleteErr { entry_id, error } }
```

Correlation-id не нужен — активный спин всегда один (`AlreadyActive`).

### Тесты

- **Один тест эквивалентности**: WS-диспетчер с `Complete` → `ServerMessage::CompleteOk` и entry стал `Completed`; REST-oneshot → 200 и entry стал `Completed`; плюс один error-case (не-`Spinning` → WS `CompleteErr`, REST 409).
- Остальные 69 тестов остаются через REST — без изменений.
- Реальный WS-интеграционный тест (handshake/фреймы) — опционально, позже.

### Поведение виджета

При реконнекте: если WS-complete уже прошёл, а клиент не узнал — повторный `complete` вернёт `NotSpinning`; клиенту трактовать это как «уже готово, ок». Ресинк состояния после reconnect — через `GET /api/queue?status=Spinning`.

---

## 9. Единообразие сервисного слоя

Не должно быть так, что часть операций с одной сущностью идёт через сервис, а часть напрямую через репо.

- **`queue_repo` спрятать в `QueueService`.** Сейчас `queue_repo` торчит в `AppState` (state.rs:38), и хендлеры делятся на два пути: сложное — через `QueueService` (dequeue/complete/cancel/mark_timed_out), простое — через `queue_repo` напрямую (enqueue, list, get_by_id, count_by_status в api/queue.rs). Решение: перенести `enqueue`/`list`/`get_by_id`/`count_by_status` методами `QueueService` (учесть при пагинации из п. 6), поле `queue_repo` из `AppState` убрать.
- **`UserService` — когда появятся реальные методы.** У `user_repo`/`platform_repo` сейчас сервиса нет (чистый CRUD в репо, бизнес-правила — в хендлерах). Пользователям он, скорее всего, понадобится со своими методами (дедупликация, link-логика, агрегация профиля и т.п.). Вводить по требованию, по аналогии с `QueueService` (сервис — бизнес-правила, репо — хранение), а не заранее ради boilerplate.

**Сделано (полностью):** `queue_repo` убран из `UniAppState`/`Clone`/builder. `QueueService` получил методы-обёртки: `enqueue`, `list`, `get_by_id`, `peek_next`, `count_by_status` (все возвращают `QueueServiceError`, не репо-ошибку). Хендлеры `api/queue.rs` и тесты `api/ws.rs` ходят только через `state.queue_service`. Поведение «деqueue/peek выбирают `Error` раньше `Pending`» покрыто юнит-тестами в `inmemory_queue.rs` (`peek_prefers_error_over_pending`, `dequeue_prefers_error_over_pending`), а HTTP-тест `dequeue_next_retries_error_entry` гоняет `Error` через публичный `mark_timed_out` — доступа к репо нигде нет. Тест-конфиг использует `roulette_timeout_secs: 0` (таймаут влияет только на `mark_timed_out`/`timeout()`, которые вне main.rs вызывает лишь этот тест).

`UserService<U, P>` (user/service.rs) введён по аналогии: `user_repo` и `platform_repo` спрятаны внутри, `guest_user_id` перенесён из `AppState` в сервис (`OnceLock` внутри). Методы — минимальные, под текущие юз-кейсы: `create`, `find_by_platform`, `get_user`, `get_platforms`, `update_user`, `delete_user`, `link_platform`, `update_platform_username`, `delete_platform`, `list_platforms`, `guest_user_id`. Все возвращают `UserServiceError` (`error/user.rs`): `UserNotFound`/`PlatformLinkNotFound` → 404, `UnknownPlatform` → 400, `Repo` → 409/500. Хендлеры `api/users.rs` и `api/queue.rs` (anonymous) ходят только через `state.user_service`. `link_platform` проверяет существование юзера → `UserNotFound` (соответствует документированному 404; раньше линк несуществующему юзеру молча создавался). Юнит-тесты сервиса (`user/service.rs`) покрывают guest-кэш и маппинг ошибок; HTTP-тесты дополнены: `update_user_404`, `update_platform_username_404`, `link_platform_404`, `enqueue_anonymous_reuses_single_guest`. Расширять — по мере появления новых методов.

---

## Очередность реализации

1. Атомарность `dequeue_next` (1.1) + CAS-статусы (1.2) → снять `#[ignore]` с тестов → **сделано**
2. Constant-time токен, CORS, `/health`, graceful shutdown, TraceLayer (3, 4) → **сделано**
3. WS-auth (first-message handshake) → WS-диспетчер `complete` + тест эквивалентности (п. 8); REST-complete остаётся резервом → **сделано**
4. `roulette_timeout_secs` из env, чистка кода (4, 5) → чистка **сделана**; п. 4 → **сделано** (конфиг из `config.toml` через крейт `config`: только захардкоженные параметры, `ACCESS_KEY`/`PORT`/`CORS_ORIGINS` остаются в env; defaults → файл → env)
5. Пагинация/retention очереди (6) → **сделано** (keyset-курсор по `id` + purge по возрасту, конфиг `retention_secs`/`queue_default_limit` без env)
6. Решение generics vs dyn + `RarityService` перед sqlx (2) → **сделано** (generics + `RarityService`; трейты остались на `impl Future`)
7. Спрятать `queue_repo` в `QueueService` — все операции через сервис (9) → **сделано** (`queue_repo` убран из `AppState`, методы на сервисе; `#[cfg(test)]`-доступ к репо для теста); `UserService` → **сделано** (`user_repo`+`platform_repo`+`guest_user_id` спрятаны в `UserService<U,P>`, `UserServiceError` с маппингом 400/404/409/500)
