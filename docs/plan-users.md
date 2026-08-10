# Users

## Сущности

**Platform** — идентификатор и название платформы.

- Заранее известный набор: `twitch`, `youtube`, `vk_video_live`.
- Реализация: `PlatformId(u32)` — newtype, `Platform { id, name }`.

**User** — зритель в системе.

- Поля: id, display_name, created_at, updated_at (обновляется при любом изменении).
- Реализация: `UserId(u32)` — newtype.

**UserPlatform** — связка зрителя с платформой.

- Поля: id, user_id, platform_id, platform_user_id, platform_username.
- Уникальность: на одной платформе не может быть двух зрителей с одинаковым platform_user_id.
- Один User может быть связан с несколькими платформами.
- Реализация: `UserPlatformId(u32)` — newtype.
  > **Изменение относительно плана:** id изначально был `u32`. Сделан newtype, чтобы нельзя было перепутать с UserId или PlatformId при передаче между компонентами.

## Хранилище (репозитории)

Два репозитория: PlatformRepository и UserRepository. `RepositoryError` содержит только `Conflict` и `Database`.

> **Изменение относительно плана:** `RepositoryError::NotFound` удалён. «Не найдено» — рабочее состояние, не ошибка репозитория. Теперь методы, которые раньше возвращали `Err(NotFound(…))`, возвращают `Option<T>` или `bool`.

### PlatformRepository

| Операция             | Описание                      | Возврат                                     |
| -------------------- | ----------------------------- | ------------------------------------------- |
| `find_by_name(name)` | Найти платформу по имени      | `Result<Option<Platform>, RepositoryError>` |
| `find_by_id(id)`     | Найти платформу по id         | `Result<Option<Platform>, RepositoryError>` |
| `load_all()`         | Получить список всех платформ | `Result<Vec<Platform>, RepositoryError>`    |

При старте in-memory репозиторий заполняется тремя платформами.

### UserRepository

| Операция                                                                   | Описание                                                                        | Возврат                                         |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------- |
| `create(display_name)`                                                     | Создать нового зрителя                                                          | `Result<User, RepositoryError>`                 |
| `find_by_platform(platform_id, platform_user_id)`                          | Найти зрителя по платформе + platform_user_id. Чистый поиск без мутаций.        | `Result<Option<User>, RepositoryError>`         |
| `get_by_id(id)`                                                            | Получить зрителя по id. None если нет                                           | `Result<Option<User>, RepositoryError>`         |
| `get_platforms(user_id)`                                                   | Получить список UserPlatform для зрителя                                        | `Result<Vec<UserPlatform>, RepositoryError>`    |
| `link_platform(user_id, platform_id, platform_user_id, platform_username)` | Привязать платформу. Ошибка, если platform_user_id уже занят на этой платформе. | `Result<UserPlatform, RepositoryError>`         |
| `update_display_name(user_id, display_name)`                               | Обновить display_name. None если зритель не найден                              | `Result<Option<User>, RepositoryError>`         |
| `update_platform_username(user_id, platform_id, platform_username)`        | Обновить username в связке. None если связка не найдена                         | `Result<Option<UserPlatform>, RepositoryError>` |
| `delete_platform(user_id, platform_id)`                                    | Отвязать платформу. `true` если была удалена.                                   | `Result<bool, RepositoryError>`                 |
| `delete_user(id)`                                                          | Удалить зрителя и все его платформы. `true` если был удалён.                    | `Result<bool, RepositoryError>`                 |

> **Изменения относительно плана:**
>
> - `get_by_id`, `update_display_name`, `update_platform_username` возвращают `Option` вместо ошибки NotFound.
> - `delete_platform`, `delete_user` возвращают `bool` вместо ошибки NotFound.
> - `link_platform` не проверяет существование пользователя на уровне репозитория — это забота хендлера.

## API

| Method   | Path                                     | Request                 | Response         | Описание                                           |
| -------- | ---------------------------------------- | ----------------------- | ---------------- | -------------------------------------------------- |
| `POST`   | `/api/users`                             | `CreateUserRequest`     | 201 + User       | Создать нового зрителя.                            |
| `GET`    | `/api/users?platform=&platform_user_id=` | —                       | 200 + User       | Найти зрителя по платформе. 404 если не найден.    |
| `GET`    | `/api/users/{id}`                        | —                       | 200 + User       | Зритель с платформами. 404 если нет.               |
| `PATCH`  | `/api/users/{id}`                        | `UpdateUserRequest`     | 200 + User       | Обновить display_name. 404 если нет.               |
| `DELETE` | `/api/users/{id}`                        | —                       | 204              | Удалить зрителя и все его платформы. 404 если нет. |
| `POST`   | `/api/users/{id}/platforms`              | `LinkPlatformRequest`   | 200 + User       | Привязать платформу. 409 если уже занята.          |
| `PATCH`  | `/api/users/{id}/platforms/{platform}`   | `UpdatePlatformRequest` | 200 + User       | Обновить username на платформе. 404 если нет.      |
| `DELETE` | `/api/users/{id}/platforms/{platform}`   | —                       | 200 + User       | Отвязать платформу. 404 если нет.                  |
| `GET`    | `/api/platforms`                         | —                       | 200 + [Platform] | Список всех платформ.                              |

> **Изменение относительно плана:** хендлеры сами конвертят `None`/`false` из репозитория в 404 через `ApiError::new()`.

### Request/Response модели

- `CreateUserRequest` — `display_name`
- `LinkPlatformRequest` — `platform` (имя, не id), `platform_user_id`, `platform_username`
- `UpdateUserRequest` — `display_name`
- `UpdatePlatformRequest` — `platform_username`
- `UserResponse` — id, display_name, список UserPlatform с названием платформы, created_at, updated_at (RFC 3339)
  > **Изменение:** `UserPlatformResponse.id` — `UserPlatformId`, не `u32`.

## Сценарии использования

### Новый зритель

1. `GET /api/users?platform=twitch&platform_user_id=123` → 404
2. `POST /api/users { "display_name": "Viewer" }` → 201 + User
3. `POST /api/users/{id}/platforms { "platform": "twitch", "platform_user_id": "123", "platform_username": "viewer" }` → 200 + User

### Вернувшийся зритель

1. `GET /api/users?platform=twitch&platform_user_id=123` → 200 + User
2. Если username на платформе изменился: `PATCH /api/users/{id}/platforms/twitch { "platform_username": "new_name" }` → 200 + User

### Зритель на другой платформе

1. `GET /api/users/{id}` → 200 + User (видно обе платформы)
2. `POST /api/users/{id}/platforms { "platform": "youtube", ... }` → 200 + User

## Тесты

### PlatformRepository (+2 теста сверх плана)

1. `find_by_name` — найдена существующая платформа
2. `find_by_name` — не найдена (None)
3. `find_by_id` — найдена существующая
4. `find_by_id` — не найдена
5. `load_all` — возвращает 3 платформы

### UserRepository (без NotFound, +7 тестов сверх плана)

6. `create` — создаёт пользователя с display_name, id = 1
7. `find_by_platform` — находит существующего зрителя
8. `find_by_platform` — не находит (None)
9. `link_platform` — добавляет платформу пользователю
10. `link_platform` — повторная привязка той же (platform_id, platform_user_id) → Conflict
11. `get_platforms` — возвращает все платформы зрителя
12. `get_platforms` — пустой список
13. `update_display_name` — обновляет имя
14. `update_display_name` — несуществующий пользователь → None
15. `update_platform_username` — обновляет username
16. `update_platform_username` — несуществующая связка → None
17. `get_by_id` — находит существующего
18. `get_by_id` — не находит (None)
19. `delete_platform` — отвязывает платформу, остальные не трогает
20. `delete_platform` — несуществующая связка → false
21. `delete_user` — удаляет зрителя и его платформы
22. `delete_user` — несуществующий пользователь → false
23. `updated_at` — меняется при update_display_name
24. `updated_at` — меняется при link_platform
25. `updated_at` — меняется при delete_platform

> **Изменения:** убран тест `link_platform_nonexistent_user` — репозиторий больше не проверяет существование пользователя. NotFound заменён на Option/bool.

### API (+4 теста сверх плана)

26. `POST /api/users` 201
27. `GET /api/users?platform=twitch&platform_user_id=123` 200
28. `GET /api/users?platform=twitch&platform_user_id=unknown` 404
29. `GET /api/users?platform=unknown&platform_user_id=123` 400
30. `GET /api/users/{id}` 200
31. `GET /api/users/{id}` 404
32. `PATCH /api/users/{id}` 200
33. `DELETE /api/users/{id}` 204
34. `DELETE /api/users/{id}` 404
35. `POST /api/users/{id}/platforms` 200
36. `POST /api/users/{id}/platforms` — платформа уже привязана → 409
37. `POST /api/users/{id}/platforms` — неизвестная платформа → 400
38. `PATCH /api/users/{id}/platforms/{platform}` 200
39. `DELETE /api/users/{id}/platforms/{platform}` 200
40. `DELETE /api/users/{id}/platforms/{platform}` 404
41. `DELETE /api/users/{id}/platforms/{platform}` — неизвестная платформа → 400
42. `GET /api/platforms` 200
43. `full_flow_new_viewer` — e2e: поиск → создание → привязка → поиск

---

## Не выполнено (2026-08-10)

- `PlatformRepository::find_by_id(id)` — заявлен в плане (таблица репозитория + тесты №3/№4), в текущем коде отсутствует: `PlatformRepository` содержит только `find_by_name` и `load_all`. В коде нигде не используется. Нужно решить: добавить по плану или принять как ненужный (YAGNI) и убрать из спецификации плана.
