# Queue

## Сущности

**QueueEntry** — элемент очереди розыгрыша.

- Поля: id, user_id (User), reward_title (опционально), status, result_slot_id (опционально, RouletteSlot), created_at, updated_at.
  > **Изменение относительно плана:** `reward_title` удалён из `QueueEntry`, `EnqueueRequest`, `QueueEntryResponse` и `enqueue()`.
- Статусы: Pending (ждёт), Spinning (крутится), Completed (завершён), Error (ошибка), Cancelled (отменён).
- Один пользователь может иметь несколько Pending одновременно.

**QueueStats** — агрегат по статусам: количество pending, spinning, completed, error, cancelled.

**Требования к статусам:**

- Spinning и Error — не могут существовать одновременно. Всего не более одного из них.
- Pending → Spinning (взят в обработку)
- Spinning → Completed (подтверждён)
- Spinning → Error (таймаут)
- Error → Spinning (retry)
- Pending → Cancelled
- Error → Cancelled
- Отменить можно Pending и Error.
- Завершить (complete) можно только Spinning.

## Хранилище (репозиторий)

`QueueRepository`. `RepositoryError` содержит `Conflict` и `Database`.

| Операция                                     | Описание                                              | Возврат                                       |
| -------------------------------------------- | ----------------------------------------------------- | --------------------------------------------- |
| `enqueue(entry)`                             | Добавить элемент в очередь                            | `Result<QueueEntry, RepositoryError>`         |
| `peek_next()`                                | Error если есть, иначе первый Pending (без изменений) | `Result<Option<QueueEntry>, RepositoryError>` |
| `dequeue_next()`                             | Error если есть, иначе первый Pending (атомарно)      | `Result<Option<QueueEntry>, RepositoryError>` |
| `list(status?)`                              | Список элементов с опциональным фильтром по статусу   | `Result<Vec<QueueEntry>, RepositoryError>`    |
| `get_by_id(id)`                              | Получить элемент по id. None если нет                 | `Result<Option<QueueEntry>, RepositoryError>` |
| `update_status(id, status, result_slot_id?)` | Обновить статус и опционально результат               | `Result<QueueEntry, RepositoryError>`         |
| `count_by_status()`                          | Количество по каждому статусу                         | `Result<QueueStats, RepositoryError>`         |
| `find_timed_out()`                           | Найти Spinning, у которых истёк таймаут               | `Result<Vec<QueueEntry>, RepositoryError>`    |

> **Изменение относительно плана:** `enqueue` принимает `user_id` без `reward_title`.

## API

| Method | Path                       | Caller           | Request                                                            | Response              | Описание                                                                                                             |
| ------ | -------------------------- | ---------------- | ------------------------------------------------------------------ | --------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `POST` | `/api/queue`               | External service | `{ platform, platform_user_id, platform_username, reward_title? }` | 200 + QueueEntry      | Добавить в очередь. Создаёт/находит пользователя.                                                                    |
| `GET`  | `/api/queue`               | Dock             | `?status=`                                                         | 200 + [QueueEntry]    | Список элементов.                                                                                                    |
| `GET`  | `/api/queue/{id}`          | Dock             | —                                                                  | 200 + QueueEntry      | Элемент с пользователем. 404 если нет.                                                                               |
| `GET`  | `/api/queue/next`          | Dock             | —                                                                  | 200 + QueueEntry      | Error если есть, иначе первый Pending (без извлечения). 404 если нет ни Error ни Pending.                            |
| `POST` | `/api/queue/next`          | Dock             | —                                                                  | 200 + { entry, slot } | Error → Spinning. Если Error нет — Pending → Spinning. 409 если уже есть Spinning. 404 если нет ни Error ни Pending. |
| `POST` | `/api/queue/{id}/complete` | Widget           | —                                                                  | 200                   | Подтвердить → Completed. 409 если не Spinning.                                                                       |
| `POST` | `/api/queue/{id}/cancel`   | Dock             | —                                                                  | 200                   | Отменить Pending или Error. 409 если не Pending и не Error.                                                          |
| `GET`  | `/api/queue/stats`         | Dock             | —                                                                  | 200 + QueueStats      | Количество по статусам.                                                                                              |

> **Изменение относительно плана:** `reward_title` удалён из запроса.

## Сценарии использования

### Нормальный розыгрыш

1. External service → `POST /api/queue` → Pending
2. Dock → `GET /api/queue/next` → видят первый Pending
3. Dock → `POST /api/queue/next` → Pending → Spinning + событие spin_started
4. Widget отображает анимацию
5. Widget → `POST /api/queue/{id}/complete` → Completed + событие spin_completed

### Отмена

1. Dock → `POST /api/queue/{id}/cancel` — только если Pending или Error
2. Если статус не Pending и не Error → 409 Conflict

### Таймаут

1. Background task проверяет Spinning с истёкшим таймаутом.
2. Найденные → Error + событие spin_error.

## Зависимости

- **`RouletteService`** — `POST /api/queue/next` использует `RouletteService::roll()` для выбора случайного слота по весам. Если слотов нет (roll → None) → 500.
- **`PlatformRepository`** — `POST /api/queue` ищет платформу по имени. Не найдена → 400.
- **`UserRepository`** — `POST /api/queue` создаёт или находит пользователя: `find_by_platform` → если нет → `create` + `link_platform`.
- **`Config`** — статическая структура с настройками модуля.

## Конфиг

```rust
pub struct Config {
    pub roulette_timeout_secs: u64,
}
```

Дефолт: `roulette_timeout_secs = 10`.

## События (порты)

```rust
pub enum SpinEvent {
    Started { entry: QueueEntry, slot: RouletteSlot, user: User },
    Completed { entry: QueueEntry },
    Error { entry: QueueEntry },
}
```

> **Изменение относительно плана:** события содержат только id и display-поля:
>
> ```rust
> Started { entry_id: QueueEntryId, slot_name: String, slot_rarity: String, user_name: String },
> Completed { entry_id: QueueEntryId },
> Error { entry_id: QueueEntryId },
> ```

```rust
/// Инфраструктурная ошибка при отправке события (сеть, канал переполнен, и т.д.)
pub struct EventError(pub String);
```

> **Изменение относительно плана:** `EventError` — `thiserror` enum в `src/error/event.rs`:
>
> ```rust
> #[derive(Debug, Error)]
> pub enum EventError {
>     #[error("publish failed: {0}")]
>     Publish(String),
> }
> ```

```rust
pub trait SpinEventPublisher: Send + Sync {
    async fn publish_spin(&self, event: SpinEvent) -> Result<(), EventError>;
}
```

Если `publish_spin` вернул ошибку — handler возвращает 500. Ошибка публикации не отменяет транзакцию в БД (entry уже в новом статусе).

## Тесты

### QueueRepository

1. `enqueue` — создаёт запись с id = 1, статусом Pending
2. `enqueue` — при отсутствии пользователя → ошибка (нарушение FK)
3. `peek_next` — Error есть → возвращает Error
4. `peek_next` — Error нет, есть Pending → возвращает Pending
5. `peek_next` — ничего нет → None
6. `dequeue_next` — Error есть → возвращает Error (снимает)
7. `dequeue_next` — Error нет, есть Pending → возвращает Pending (снимает)
8. `dequeue_next` — ничего нет → None
9. `list` — без фильтра возвращает все записи
10. `list` — с фильтром по статусу
11. `get_by_id` — находит существующую
12. `get_by_id` — не найдена → None
13. `update_status` — обновляет статус и result_slot_id
14. `update_status` — несуществующий id → None
15. `count_by_status` — возвращает корректные количества
16. `find_timed_out` — находит Spinning с истёкшим таймаутом
17. `find_timed_out` — ничего не находит если все свежие
18. `created_at` — устанавливается при enqueue
19. `updated_at` — меняется при update_status

### API

20. `POST /api/queue` 200
21. `POST /api/queue` — неизвестная платформа → 400
22. `GET /api/queue` 200
23. `GET /api/queue?status=pending` 200
24. `GET /api/queue/{id}` 200
25. `GET /api/queue/{id}` 404
26. `GET /api/queue/next` 200
27. `GET /api/queue/next` — нет Error и нет Pending → 404
28. `POST /api/queue/next` — Pending → Spinning 200
29. `POST /api/queue/next` — Error → Spinning 200
30. `POST /api/queue/next` — уже есть Spinning → 409
31. `POST /api/queue/next` — нет Error и нет Pending → 404
32. `POST /api/queue/{id}/complete` 200
33. `POST /api/queue/{id}/complete` — не Spinning → 409
34. `POST /api/queue/{id}/cancel` — Pending → 200
35. `POST /api/queue/{id}/cancel` — Error → 200
36. `POST /api/queue/{id}/cancel` — не Pending и не Error → 409
37. `GET /api/queue/stats` 200
38. `POST /api/queue` — новый пользователь (создаётся User + link_platform)
39. `POST /api/queue/next` — нет слотов (roll → None) → 500
40. `POST /api/queue/next` — publish_spin вернул Err → 500
41. `POST /api/queue/{id}/complete` — publish_spin вернул Err → 500

### SpinEventPublisher (с моком)

42. `POST /api/queue/next` → Pending → Spinning → вызван `SpinEventPublisher::publish_spin(Started)`
43. `POST /api/queue/next` → Error → Spinning → вызван `SpinEventPublisher::publish_spin(Started)`
44. `POST /api/queue/{id}/complete` → Spinning → Completed → вызван `SpinEventPublisher::publish_spin(Completed)`
45. Таймаут: запуск проверки → Spinning истёк → Error + вызван `SpinEventPublisher::publish_spin(Error)`
