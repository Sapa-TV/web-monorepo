# Users

## Сущности

**Platform** — идентификатор и название платформы.

- Заранее известный набор: `twitch`, `youtube`, `vk_video_live`.

**User** — зритель в системе.

- Поля: id, display_name, created_at, updated_at (обновляется при любом изменении).

**UserPlatform** — связка зрителя с платформой.

- Поля: id, user_id, platform_id, platform_user_id, platform_username.
- Уникальность: на одной платформе не может быть двух зрителей с одинаковым platform_user_id.
- Один User может быть связан с несколькими платформами.

## Хранилище (репозитории)

Два репозитория: PlatformRepository и UserRepository.

### PlatformRepository

| Операция             | Описание                      |
| -------------------- | ----------------------------- |
| `find_by_name(name)` | Найти платформу по имени      |
| `find_by_id(id)`     | Найти платформу по id         |
| `load_all()`         | Получить список всех платформ |

При старте in-memory репозиторий заполняется тремя платформами.

### UserRepository

| Операция                                | Описание                                                                 |
| --------------------------------------- | ------------------------------------------------------------------------ |
| `create(display_name)`                  | Создать нового зрителя. Возвращает User.                                 |
| `find_by_platform(...)`                 | Найти зрителя по платформе + platform_user_id. Чистый поиск без мутаций. |
| `get_by_id(id)`                         | Получить зрителя по id.                                                  |
| `get_platforms(user_id)`                | Получить список UserPlatform для зрителя.                                |
| `link_platform(...)`                    | Привязать к зрителю платформу. Ошибка, если такая связка уже есть.       |
| `update_display_name(...)`              | Обновить display_name зрителя.                                           |
| `update_platform_username(...)`         | Обновить platform_username в существующей связке.                        |
| `delete_platform(user_id, platform_id)` | Отвязать платформу от зрителя.                                           |
| `delete_user(id)`                       | Удалить зрителя и все его платформы.                                     |

## API

| Method   | Path                                     | Request                 | Response         | Описание                                           |
| -------- | ---------------------------------------- | ----------------------- | ---------------- | -------------------------------------------------- |
| `POST`   | `/api/users`                             | `CreateUserRequest`     | 201 + User       | Создать нового зрителя.                            |
| `GET`    | `/api/users?platform=&platform_user_id=` | —                       | 200 + User       | Найти зрителя по платформе. 404 если не найден.    |
| `GET`    | `/api/users/{id}`                        | —                       | 200 + User       | Зритель с платформами. 404 если нет.               |
| `PATCH`  | `/api/users/{id}`                        | `UpdateUserRequest`     | 200 + User       | Обновить display_name.                             |
| `DELETE` | `/api/users/{id}`                        | —                       | 204              | Удалить зрителя и все его платформы. 404 если нет. |
| `POST`   | `/api/users/{id}/platforms`              | `LinkPlatformRequest`   | 200 + User       | Привязать платформу.                               |
| `PATCH`  | `/api/users/{id}/platforms/{platform}`   | `UpdatePlatformRequest` | 200 + User       | Обновить username на платформе.                    |
| `DELETE` | `/api/users/{id}/platforms/{platform}`   | —                       | 200 + User       | Отвязать платформу. 404 если нет.                  |
| `GET`    | `/api/platforms`                         | —                       | 200 + [Platform] | Список всех платформ.                              |

### CreateUserRequest

- `display_name`

### LinkPlatformRequest

- `platform` (имя, не id), `platform_user_id`, `platform_username`

### UpdateUserRequest

- `display_name`

### UpdatePlatformRequest

- `platform_username`

### UserResponse

- id, display_name, список UserPlatform (с названием платформы), created_at, updated_at

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

### PlatformRepository

1. `find_by_name` — найдена существующая платформа
2. `find_by_name` — не найдена (None)
3. `load_all` — возвращает 3 платформы

### UserRepository

4. `create` — создаёт пользователя с display_name, id = 1, created_at/updated_at заполнены
5. `find_by_platform` — находит существующего зрителя по платформе + platform_user_id
6. `find_by_platform` — не находит (None)
7. `link_platform` — добавляет платформу пользователю, возвращает UserPlatform
8. `link_platform` — повторная привязка той же (platform_id, platform_user_id) → ошибка
9. `get_platforms` — возвращает все платформы зрителя
10. `get_platforms` — пустой список, если платформ нет
11. `update_display_name` — обновляет имя
12. `update_platform_username` — обновляет username в существующей связке
13. `get_by_id` — находит существующего зрителя
14. `get_by_id` — не находит (None/ошибка)
15. `delete_platform` — отвязывает платформу, остальные не трогает
16. `delete_user` — удаляет зрителя и его платформы

### API

17. `POST /api/users` 201
18. `GET /api/users?platform=twitch&platform_user_id=123` 200
19. `GET /api/users?platform=twitch&platform_user_id=unknown` 404
20. `GET /api/users/{id}` 200
21. `GET /api/users/{id}` 404
22. `PATCH /api/users/{id}` 200
23. `DELETE /api/users/{id}` 204
24. `DELETE /api/users/{id}` 404
25. `POST /api/users/{id}/platforms` 200
26. `POST /api/users/{id}/platforms` — платформа уже привязана → 409
27. `PATCH /api/users/{id}/platforms/{platform}` 200
28. `DELETE /api/users/{id}/platforms/{platform}` 200
29. `DELETE /api/users/{id}/platforms/{platform}` 404
30. `GET /api/platforms` 200
