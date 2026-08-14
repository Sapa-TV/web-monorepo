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

| Method | Path                        | Описание                                                                                                        |
| ------ | --------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `POST` | `/wapi/queue`               | External service. Добавить в очередь; создаёт/находит пользователя. 400 неизвестная платформа.                  |
| `GET`  | `/wapi/queue`               | Dock. Список элементов (`?status=` фильтр).                                                                     |
| `GET`  | `/wapi/queue/{id}`          | Dock. Элемент с пользователем. 404 если нет.                                                                    |
| `GET`  | `/wapi/queue/next`          | Dock. Error если есть, иначе первый Pending (без извлечения). 404 если нет ни Error ни Pending.                 |
| `POST` | `/wapi/queue/next`          | Dock. Error → Spinning, иначе Pending → Spinning. 409 если уже есть Spinning. 404 если нет ни Error ни Pending. |
| `POST` | `/wapi/queue/{id}/complete` | Widget. Подтвердить → Completed. 409 если не Spinning.                                                          |
| `POST` | `/wapi/queue/{id}/cancel`   | Dock. Отменить Pending или Error. 409 если не Pending и не Error.                                               |
| `GET`  | `/wapi/queue/stats`         | Dock. Количество по статусам (`QueueStats`).                                                                    |

> **Изменение относительно плана:** `reward_title` удалён из запроса.

## Сценарии использования

### Нормальный розыгрыш

1. External service → `POST /wapi/queue` → Pending
2. Dock → `GET /wapi/queue/next` → видят первый Pending
3. Dock → `POST /wapi/queue/next` → Pending → Spinning + событие spin_started
4. Widget отображает анимацию
5. Widget → `POST /wapi/queue/{id}/complete` → Completed + событие spin_completed

### Отмена

1. Dock → `POST /wapi/queue/{id}/cancel` — только если Pending или Error
2. Если статус не Pending и не Error → 409 Conflict

### Таймаут

1. Background task проверяет Spinning с истёкшим таймаутом.
2. Найденные → Error + событие spin_error.

## Зависимости

- **`RouletteService`** — `POST /wapi/queue/next` использует `RouletteService::roll()` для выбора случайного слота по весам. Если слотов нет (roll → None) → 500.
- **`PlatformRepository`** — `POST /wapi/queue` ищет платформу по имени. Не найдена → 400.
- **`UserRepository`** — `POST /wapi/queue` создаёт или находит пользователя: `find_by_platform` → если нет → `create` + `link_platform`.
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

20. `POST /wapi/queue` 200
21. `POST /wapi/queue` — неизвестная платформа → 400
22. `GET /wapi/queue` 200
23. `GET /wapi/queue?status=pending` 200
24. `GET /wapi/queue/{id}` 200
25. `GET /wapi/queue/{id}` 404
26. `GET /wapi/queue/next` 200
27. `GET /wapi/queue/next` — нет Error и нет Pending → 404
28. `POST /wapi/queue/next` — Pending → Spinning 200
29. `POST /wapi/queue/next` — Error → Spinning 200
30. `POST /wapi/queue/next` — уже есть Spinning → 409
31. `POST /wapi/queue/next` — нет Error и нет Pending → 404
32. `POST /wapi/queue/{id}/complete` 200
33. `POST /wapi/queue/{id}/complete` — не Spinning → 409
34. `POST /wapi/queue/{id}/cancel` — Pending → 200
35. `POST /wapi/queue/{id}/cancel` — Error → 200
36. `POST /wapi/queue/{id}/cancel` — не Pending и не Error → 409
37. `GET /wapi/queue/stats` 200
38. `POST /wapi/queue` — новый пользователь (создаётся User + link_platform)
39. `POST /wapi/queue/next` — нет слотов (roll → None) → 500
40. `POST /wapi/queue/next` — publish_spin вернул Err → 500
41. `POST /wapi/queue/{id}/complete` — publish_spin вернул Err → 500

### SpinEventPublisher (с моком)

42. `POST /wapi/queue/next` → Pending → Spinning → вызван `SpinEventPublisher::publish_spin(Started)`
43. `POST /wapi/queue/next` → Error → Spinning → вызван `SpinEventPublisher::publish_spin(Started)`
44. `POST /wapi/queue/{id}/complete` → Spinning → Completed → вызван `SpinEventPublisher::publish_spin(Completed)`
45. Таймаут: запуск проверки → Spinning истёк → Error + вызван `SpinEventPublisher::publish_spin(Error)`
