# Backend — план улучшений / рефакторинга / оптимизации

## Приоритеты

Порядок работ: **1 → 3 → 4** (баги/безопасность/наблюдаемость, низкий риск), затем **п. 8** (WS-auth → WS-complete), далее **2** (перед миграцией на sqlx).

---

## 1. Корректность / гонки

### 1.1 `dequeue_next` не атомарен

`queue/service.rs:46-92` — «проверка активного» (`list(Spinning)`) и `dequeue_next()` — два разных захвата лока.

Проблемы:

- Два параллельных `POST /api/queue/next` могут оба пройти проверку → **двойной спин** (два entry в `Spinning` одновременно), либо один получит `QueueEmpty` (404) вместо `409`.
- Если `roll()` вернёт `NoSlots`, entry остаётся `Spinning` без `result_slot_id` (орфан; лечится только timeout-задачей).

Решение: выбор entry + roll + запись `result_slot_id` внутри одного лока/транзакции; `AlreadyActive` — результат конкурентного выбора, а не предпроверка.

### 1.2 CAS-семантика статусов

`complete`/`cancel` (`queue/service.rs:95-142`) — get-then-update. В гонке двойной `complete` пройдёт (оба прочитают `Spinning`).

Решение: `update_status` внутри репо проверяет ожидаемый статус (compare-and-swap) и возвращает конфликт.

### 1.3 `mark_timed_out` vs `dequeue`

Timeout-задача может перевести в `Error` entry, который параллельно декивается. Закрыть тем же локом/транзакцией.

### 1.4 Case-sensitive статус в query

`QueueStatus` сериализуется как есть (`Pending`/`Spinning`), а фронтенд по плану шлёт `?status=pending` → `400`. Либо `#[serde(rename_all = "lowercase")]`, либо слать `Pending`.

---

## 2. Подготовка к sqlite (ключевое архитектурное решение)

- **`impl Future` в трейтах репозиториев блокирует `dyn`** (queue/repository.rs:9, roulette/repository.rs:7, user/repository.rs:7, roulette/rarity.rs:54). Трейты не object-safe → нельзя `Arc<dyn QueueRepository>`. Выбор перед миграцией:
  - пробросить generic'и через `QueueService` / `AppState`, либо
  - перейти на `async_trait` / боксированные фьючеры.
- `QueueService` завязан на конкретные `Arc<InMemoryQueueRepository>` / `Arc<InMemoryRarityRepository>` (queue/service.rs:22-24). Типы абстрагировать.
- **Асимметрия: у слотов есть кэш-сервис, у rarities нет.** `list_rarities` и CRUD ходят в репо напрямую (api/rarities.rs:69) — при sqlite это диск на каждый запрос. Сделать `RarityService` с кэшем по аналогии с `RouletteSlotService` (или пересмотреть, нужен ли кэш).

---

## 3. Безопасность

- **Сравнение токена не constant-time** (api/auth.rs:24): `token == Some(...)` → timing-атака. Заменить на постоянновременное сравнение (например `subtle`).
- `CorsLayer::permissive()` (main.rs:46) — для прод ограничить origin'ы.
- `/ws` без авторизации — любой подключённый получает все события.

---

## 4. Наблюдаемость и сервер

- Нет `TraceLayer` (запросное логирование), нет `/health`, нет graceful shutdown (`SIGTERM` → `with_graceful_shutdown`).
- `roulette_timeout_secs` захардкожен `10` (config.rs:15) — вынести в env как `PORT`/`ACCESS_KEY`.

---

## 5. Чистка кода

- `unreachable!()` в `From<QueueServiceError> for ApiError` (error/queue.rs:41-44) — реструктурировать.
- `NaiveDateTime` → `DateTime<Utc>` (timezone-однозначность).
- Повторяющиеся generic-типы `RouletteService<StandartRandomProvider, Arc<InMemoryRouletteSlotRepository>>` (queue/service.rs:24, state.rs) — type alias'ы.
- `StandartRandomProvider` — опечатка в имени; `rand` уже в зависимостях — можно упростить.

---

## 6. Рост данных

- Очередь никогда не чистится, `/api/queue` без пагинации (api/queue.rs). Добавить лимит/курсор + retention для `Completed`/`Cancelled`.

---

## 7. Тесты (покрывают пункты 1)

В `apps/backend/src/api/queue.rs`, модуль `tests`:

| Тест                                  | Поведение                                                | Статус                                                 |
| ------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------ |
| `dequeue_next_parallel_only_one_spin` | два параллельных `next` → ровно один 200, другой 409     | `#[ignore]` — сейчас двойной спин                      |
| `dequeue_next_no_slots_no_orphan`     | `next` без слотов → 422, в очереди нет `Spinning`        | `#[ignore]` — сейчас entry застревает в `Spinning`     |
| `complete_parallel_only_one_success`  | два параллельных `complete` → ровно один 200, другой 409 | `#[ignore]` — сейчас оба проходят                      |
| `slot_created_via_api_used_in_roll`   | `POST /api/slots` → следующий `roll` видит слот          | проходит (валидирует общий `Arc<RouletteSlotService>`) |

`#[ignore]` сняты по мере реализации пунктов 1.1-1.2.

---

## 8. Принято: подтверждение `complete` по WS (основной канал) + REST (резервный)

Обсуждение 2026-07-31. Решение принято: подтверждение спина идёт по WS, REST остаётся как резервный канал и для тестов.

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

## Очередность реализации

1. Атомарность `dequeue_next` (1.1) + CAS-статусы (1.2) → снять `#[ignore]` с тестов
2. Constant-time токен, CORS, `/health`, graceful shutdown, TraceLayer (3, 4)
3. **WS-auth (first-message handshake)** → WS-диспетчер `complete` + тест эквивалентности (п. 8); REST-complete остаётся резервом
4. `roulette_timeout_secs` из env, чистка кода (4, 5)
5. Пагинация/retention очереди (6)
6. Решение generics vs dyn + `RarityService` перед sqlx (2)
