# Backend — завершение реализации

## Текущее состояние

Всё доменное ядро и API реализованы. Сервер не запускается.

### Реализовано

| Компонент                                                  | Статус |
| ---------------------------------------------------------- | ------ |
| Domain: User, Platform, UserPlatform                       | ✅     |
| Domain: RouletteSlot, Rarity                               | ✅     |
| Domain: QueueEntry, QueueStatus, QueueStats                | ✅     |
| Domain: RouletteService (weighted roll)                    | ✅     |
| InMemory репозитории (5 шт)                                | ✅     |
| API: slots CRUD                                            | ✅     |
| API: users CRUD + platforms                                | ✅     |
| API: queue enqueue/list/peek/dequeue/complete/cancel/stats | ✅     |
| Events: SpinEvent, SpinEventPublisher, NoopEventPublisher  | ✅     |
| Config                                                     | ✅     |
| ApiError + From<RepositoryError>                           | ✅     |
| Error types (RepositoryError, EventError)                  | ✅     |

### Не реализовано

| Компонент                          | Статус | Причина                                      |
| ---------------------------------- | ------ | -------------------------------------------- |
| `main.rs` — HTTP server            | ❌     | Нет `axum::serve`, `TcpListener`, роутера    |
| AppState инициализация + seed data | ❌     | Нет редкостей, слотов, платформ при старте   |
| Rarities API                       | ❌     | Репозиторий есть, эндпоинтов нет             |
| Swagger UI (`/docs`)               | ❌     | utoipa-swagger-ui в Cargo.toml, не подключён |
| CORS                               | ❌     | tower-http/cors в Cargo.toml, не настроен    |
| Static files (Dock/Widget)         | ❌     | tower-http/fs в Cargo.toml, не настроен      |
| Timeout background task            | ❌     | `find_timed_out` есть, tokio task нет        |
| Real event publisher (SSE/WS)      | ❌     | Noop, событий никто не получает              |

---

## Что нужно сделать в main.rs

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = AppState::new(
        InMemoryRouletteSlotRepository::new(),
        InMemoryRarityRepository::new(),
        StandartRandomProvider,
    );

    let app = api::router().with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Seed data

При старте репозитории пусты. Нужна функция seed:

```rust
fn seed(app_state: &AppState) {
    // Редкости
    let common = app_state.rarity_repo.save(Rarity::new(…)).await;
    let rare = app_state.rarity_repo.save(Rarity::new(…)).await;
    let legendary = app_state.rarity_repo.save(Rarity::new(…)).await;

    // Слоты
    app_state.slot_repo.save(RouletteSlot::new(…)).await;
}

// Вызов после создания AppState
```

Либо передавать `new_seeded()` для slot_repo и rarity_repo по аналогии с `InMemoryPlatformRepository::new_seeded()`.

## Эндпоинты которых не хватает

### Rarities CRUD

| Method   | Path                 | Описание              |
| -------- | -------------------- | --------------------- |
| `GET`    | `/api/rarities`      | Список всех редкостей |
| `POST`   | `/api/rarities`      | Создать редкость      |
| `PUT`    | `/api/rarities/{id}` | Обновить редкость     |
| `DELETE` | `/api/rarities/{id}` | Удалить редкость      |

### Swagger

```rust
use utoipa_swagger_ui::SwaggerUi;

let app = api::router()
    .with_state(state)
    .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()));
```

### Static files (для Dock и Widget)

```rust
use tower_http::services::ServeDir;

let app = api::router()
    .with_state(state)
    .nest_service("/", ServeDir::new("static"));
```

---

## Dock panel — процессы

Панель управления стримера. Работает как SPA, общается с бекендом через REST + SSE.

### Состояния и переходы

```
Dock открыт
  │
  ├── Нет активного спина
  │     ├── Показывает очередь (GET /api/queue?status=pending)
  │     ├── Кнопка "Запустить розыгрыш" → POST /api/queue/next
  │     │     └── Ответ: { entry, slot } → переключается в "Спин активен"
  │     └── Кнопка "Отмена" → POST /api/queue/{id}/cancel
  │
  └── Спин активен (есть Spinning)
        ├── Показывает "Крутится..." + имя зрителя + название слота
        ├── Ждёт SSE/опрос: событие Completed или Error
        ├── Кнопка "Отменить розыгрыш" → POST /api/queue/{id}/cancel
        └── После Completed/Error → возвращается к "Нет активного спина"
```

### REST запросы

| Что                    | Запрос                           |
| ---------------------- | -------------------------------- |
| Получить очередь       | `GET /api/queue?status=pending`  |
| Получить активный спин | `GET /api/queue?status=spinning` |
| Запустить              | `POST /api/queue/next`           |
| Отменить               | `POST /api/queue/{id}/cancel`    |
| Статистика             | `GET /api/queue/stats`           |
| Список слотов          | `GET /api/slots`                 |
| Список редкостей       | `GET /api/rarities`              |
| Список пользователей   | `GET /api/users`                 |

### SSE (реалтайм)

```typescript
// Dock подписывается на события очереди
const sse = new EventSource("/api/events");

sse.addEventListener("spin_started", (e) => {
  // { entry_id, slot_name, slot_rarity, user_name }
  dock.showSpinning(e.data);
});

sse.addEventListener("spin_completed", (e) => {
  // { entry_id }
  dock.showCompleted(e.data);
  dock.refreshQueue();
});

sse.addEventListener("queue_stats", (e) => {
  // { pending, spinning, completed, error, cancelled }
  dock.updateStats(e.data);
});
```

---

## Widget — процессы

Оверлей на стриме. Показывает анимацию розыгрыша. Работает как SPA.

### Состояния и переходы

```
Widget открыт
  │
  ├── Ожидание (нет активного спина)
  │     └── Показывает "Ожидание розыгрыша..."
  │         подписан на SSE: spin_started
  │
  ├── Анимация спина
  │     ├── Получено spin_started: имя зрителя, редкость, название слота
  │     ├── Крутит анимацию (N секунд)
  │     ├── Показывает результат
  │     └── Отправляет POST /api/queue/{id}/complete
  │         └── Возвращается в "Ожидание"
  │
  └── Ошибка
        └── Получено spin_error → показать "Ошибка"
```

### REST запросы

| Что              | Запрос                                |
| ---------------- | ------------------------------------- |
| Подтвердить спин | `POST /api/queue/{entry_id}/complete` |

### SSE (реалтайм)

```typescript
const sse = new EventSource("/api/events");

sse.addEventListener("spin_started", (e) => {
  const { entry_id, slot_name, slot_rarity, user_name } = JSON.parse(e.data);
  widget.showEntryId = entry_id;
  widget.startAnimation({ user_name, slot_name, slot_rarity });
});

sse.addEventListener("spin_error", (e) => {
  widget.showError();
});
```

### Таймаут

Если Widget не отправил `complete` в течение `roulette_timeout_secs`:

- Background task переводит Spinning → Error
- SSE шлёт `spin_error`
- Dock видит Error и может retry

---

## SSE endpoint

Бекенд держит пул SSE-клиентов. Когда `SpinEventPublisher::publish_spin` вызывается, шлёт событие всем подключённым.

```rust
// GET /api/events
pub async fn events(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, ...>>> {
    let rx = state.event_publisher.subscribe();
    Sse::new(
        rx.map(|event| match event {
            SpinEvent::Started { entry_id, slot_name, .. } => {
                Event::default().event("spin_started").data(serde_json::to_string(&...))
            }
            SpinEvent::Completed { entry_id } => {
                Event::default().event("spin_completed").data(...)
            }
        })
    )
}
```

Потребуется заменить `NoopEventPublisher` на SSE-версию, которая:

- Принимает подписчиков (broadcast channel)
- При `publish_spin` рассылает всем подписчикам
- При отключении клиента — автоматически удаляет

---

## Очередность реализации

1. **main.rs** — поднять сервер (AppState + router + listen)
2. **Seed data** — слоты и редкости при старте
3. **CORS** — чтобы Dock/Widget с других портов могли стучаться
4. **Rarities API** — полный CRUD (как у slots)
5. **Swagger UI** — документация
6. **SSE EventPublisher** — замена Noop, реалтайм для Dock и Widget
7. **Static files** — раздача фронтенда
8. **Timeout background task** — tokio task с `find_timed_out`
