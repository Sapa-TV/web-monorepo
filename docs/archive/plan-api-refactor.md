# API: тонкий слой + разделение на `api` (/api) и `widget_api` (/wapi)

Статус: **план**, не начато.

Дата: 2026-08-14

## Контекст (текущее состояние)

- API собирается централизованно в `apps/backend/src/api.rs` (`router_with_auth`): роуты сгруппированы в `public_router()/protected_router()/session_router()/root_router()`, защита навешивается `route_layer` в одном месте.
- Хендлеры содержат бизнес-логику: `session.rs::create_session` (оркестрация cookie/ticket/roles), `users.rs` (`build_user_response`, `resolve_user_platforms` с `for`), queue-маппинг `slot_name`.
- 4 миддлвары в `api/auth.rs`: `require_auth` (PAK-ключ, имя врёт), `require_session`, `require_admin`, `require_root`.
- Юнит-тесты живут в api-слое (~42 через роутер), при этом `queue/service.rs` не имеет своих тестов (0).
- Фронт ходит только в `/api/queue*` и `/ws` (widget.js, panel.js, dock, roulette). Остальные PAK-роуты (rarities/slots/users/stream POST) внешних потребителей не имеют.
- Healthcheck деплоя бьёт в `http://127.0.0.1:3000/api/health` (`deploy/scripts/deploy-backend.sh:30`) — он обновляется в шаге 1 вместе с переносом `/health` под `/api`.

## Цель

1. Убрать бизнес-логику из api-слоя: хендлер = извлечение параметров → вызов сервиса → маппинг DTO → статус. Вся оркестрация/правила — в `*/service.rs`.
2. Ограничить наличие логики в api-слое автоматически (ast-grep правило).
3. Разделить на два независимых модуля: `api` (публичный/сессия/root) и `widget_api` (только PAK-ключ), с разными префиксами `/api` и `/wapi`.
4. Тесты бизнеса живут в сервисах. В api/widget_api — только auth-тесты (исключение).

## Решения

- **Граница**: api/widget_api без бизнес-логики. Механический маппинг через методы/замыкания допускается (без `if/match/for/loop`).
- **Принуждение**: ast-grep правило (как существующие в `.sg/rules`): запрет `if/match/for/loop` в хендлерах `src/api/**` и `src/widget_api/**`; allowlist — `auth.rs` (миддлвары) и `ws.rs` (протокол). + тест правила в `.sg/tests`, гоняется через `just lint`. + doc-комментарий границы в `api.rs`.
- **Тесты**: юнит-тесты бизнеса — в сервисах (в т.ч. добавить `mod tests` в `queue/service.rs`, сейчас 0). В api/widget_api допускаются только auth-тесты: unit-тесты миддлваров + небольшой full-stack «матрицы защиты» (порядок слоёв, 401/403). Handler/router-тесты бизнеса убираются.
- **Миддлвары**: api — 3 (`require_session`, `require_admin`, `require_root`); widget_api — 1 (`require_key`, переименование из `require_auth`). Лежат в своих деревьях (`api/auth.rs`, `widget_api/auth.rs`), `widget_api` не импортирует `crate::api`.
- **WS**: переезжает на `/wapi/ws`.
- **Переезд путей**: резкий, без алиасов. Обновляем фронт (4 файла) в том же шаге.

## Целевая маршрутная карта

| Путь (было)                                                         | Путь (станет)                     | Модуль                         | Защита           |
| ------------------------------------------------------------------- | --------------------------------- | ------------------------------ | ---------------- |
| `/health`, `/version`                                               | `/api/health`, `/api/version`     | `api` (ops)                    | public           |
| `/api/auth/twitch(+/callback)`, `/api/sessions`, `/api/sessions/me` | без изменений                     | `api/session`                  | public / session |
| `/api/admin*`, `/api/admin/twitch*`, `/api/admin/ingress*`          | без изменений                     | `api/admin{,/twitch,/ingress}` | session / root   |
| `GET /api/stream/status`                                            | без изменений                     | `api/stream`                   | public           |
| `/api/queue*`                                                       | `/wapi/queue*`                    | `widget_api/queue`             | key              |
| `/ws`                                                               | `/wapi/ws`                        | `widget_api/ws`                | key (in-band)    |
| `/api/rarities*`                                                    | `/wapi/rarities*`                 | `widget_api/rarities`          | key              |
| `/api/slots*`                                                       | `/wapi/slots*`                    | `widget_api/roulette_slots`    | key              |
| `/api/users*`, `/api/platforms`                                     | `/wapi/users*`, `/wapi/platforms` | `widget_api/users`             | key              |
| `POST /api/stream/status`                                           | `POST /wapi/stream/status`        | `widget_api/stream`            | key              |

## Целевая структура

Крейт уже на edition 2024: корневой модуль дерева = `foo.rs`, вложенные — `foo/bar.rs` (без `mod.rs`). Поэтому `api.rs`/`widget_api.rs` остаются корневыми файлами модулей, `lib.rs` объявляет только `pub mod api; pub mod widget_api;`, а импорты идут по-новому — `use crate::api::auth::require_session;` (без `super::super::`).

```
src/lib.rs                     pub mod api; pub mod widget_api; ApiDoc/MergeSubdocs → backend::ApiDoc
src/api.rs                     api::router(state): nest("/api", public + session/admin/root) — префикс в одном месте
src/api/
  auth.rs                      require_session/require_admin/require_root + cookie-хелперы
  session.rs, admin.rs, admin/{twitch,ingress}.rs, stream.rs
src/widget_api.rs              widget_api::router(state): nest("/wapi", ws + require_key) — префикс в одном месте
src/widget_api/
  auth.rs                      require_key
  queue.rs, ws.rs, rarities.rs, roulette_slots.rs, users.rs, stream.rs
```

Роуты и utoipa-аннотации оперируют относительными путями без префикса (`/queue`, `/admin`, `/stream/status`), префиксы `nest`-ауются в корнях деревьев; `/health`/`/version` живут в `api::public_router` и попадают под `/api`:

```rust
// api.rs
pub fn router(state: AppState) -> Router {
    ...
    Router::new()
        .nest("/api", public_router().merge(session_protected))
        .with_state(state)
}
// widget_api.rs
pub fn router(state: AppState) -> Router {
    let key = from_fn_with_state(state.clone(), require_key);
    let key_protected = key_protected_routes().route_layer(key);
    Router::new()
        .nest("/wapi", Router::new().merge(ws::public_router()).merge(key_protected))
        .with_state(state)
}
```

Паттерн «модуль владеет защитой» (пути относительные):

```rust
// widget_api/queue.rs
pub fn router(state: AppState) -> Router<AppState> {
    let key = from_fn_with_state(state, require_key);
    Router::new()
        .route("/queue", post(enqueue))
        // ...
        .route_layer(key)
}
```

Порядок лейеров для admin/root (последний `.route_layer()` = внешний, выполняется первым; session-слой ставим последним):

```rust
.route("/admin", get(list_admins))
.route_layer(admin_layer)      // 2-м
.route_layer(session_layer);   // 1-м → кладёт Session
```

## Шаги

### Шаг 1 — Сплит + префиксы через nest (структура, фронт)

Чисто механический, без изменения логики.

- Перенос файлов: `widget_api/{queue,ws,rarities,roulette_slots,users,stream}.rs` + `widget_api/auth.rs` (`require_key`); `api.rs` остаётся корневым (session, admin, admin/{twitch,ingress}, stream GET), поднятые подкаталоги `api/admin/*` без изменений.
- Пути: роутеры и utoipa-аннотации переходят на относительные пути (без `/api`/`/wapi`). Префиксы — только в корнях: `api.rs` `nest("/api", ...)`, `widget_api.rs` `nest("/wapi", ...)`. Итог: `/api/...` и `/wapi/...`, `/ws` → `/wapi/ws`, `/health`/`/version` → `/api/health`/`/api/version`.
- `lib.rs`: `pub mod api; pub mod widget_api;` (без `mod.rs`). `ApiDoc`/`MergeSubdocs` → `backend::ApiDoc`; MergeSubdocs строит сабдоки (`wapi` из widget-модулей, `api`/main из api-модулей) и нэстит их через `OpenApi::nest` — каждый префикс один раз (обновить `tools/gen-openapi`).
- `main.rs`: `api::router(state.clone()).merge(widget_api::router(state.clone()))` + cors/trace/swagger/redoc сверху.
- Импорты по edition 2024: `use crate::api::auth::{...}; use crate::widget_api::...` — без цепочек `super::super::`.
- `require_auth` → `require_key`.
- Фронт (4 файла): `html/js/widget.js`, `html/js/panel.js`, `src/routes/(panels)/dock/+page.svelte`, `src/routes/(widgets)/roulette/+page.svelte` — `/api/queue*` → `/wapi/queue*`, ws → `/wapi/ws`.
- Тесты: `.uri(...)` остаются полными (`/wapi/queue`, `/api/admin/...`); ассемблер — единый `test_fixtures::test_router(state)` = `api::router(state).merge(widget_api::router_no_auth(state))` (api с auth-леерами, widget без key). `#[cfg(test)] router_no_auth` в `widget_api.rs` — переходный, удаляется в шагах 2–3. Правки тестов = только путь импорта.
- Healthcheck деплоя: `deploy/scripts/deploy-backend.sh:30` → `/api/health`.

Верификация: `cargo test -p backend`, `cargo clippy`, `just gen` (пути `/api/*`, `/wapi/*` в spec), фронт собирается.

### Шаг 2 — Тонкий `widget_api` + lint

- Логика → сервисы:
  - `users.rs`: `build_user_response`/`resolve_user_platforms` (там `for`) → `user/service.rs`.
  - `queue.rs`: маппинг `slot_name` → сервис/чистый маппер без управляющих конструкций.
  - `ws.rs`: остаётся (allowlist); `complete` уже через `queue_service`.
- Тесты: router-тесты users/queue/ws → сервис-уровень. Добавить `mod tests` в `queue/service.rs` (перенести assert'ы: enqueue/next/cursor/retry/409/параллельность/anonymous).
- Добавить ast-grep правило со скоупом `widget_api/**` (allowlist ws.rs) + тест правила в `.sg/tests`; починить нарушения.

Верификация: `cargo test`, `just lint`, `cargo clippy`.

### Шаг 3 — Тонкий `api` + lint

- Логика → сервисы:
  - `session.rs`: оркестрация `create_session` (cookie↔ticket, is_admin→update_display_name, issue, is_root) → `session/service.rs`; `twitch_login_callback` — тонкий.
  - `admin.rs`: уже тонкий; assert'ы PAK-ротации при необходимости → `config/store.rs` тесты.
- Тесты: router-тесты session/admin → сервис-уровень. Оставить auth-тесты (миддлвары + full-stack матрица порядка слоёв — существующие тесты защиты в admin/session).
- Расширить ast-grep правило на `api/**` (allowlist auth.rs); починить нарушения.

Верификация: `cargo test`, `just lint`, `cargo clippy`.

### Шаг 4 — Вычистка (можно влить в шаг 3)

- Удалить переходный cfg-test код и осиротевшие router-тесты.
- `just gen` → новый spec (/wapi) + `just gen-client` при необходимости; обновить пути в `docs/plan-{queue,users,ws-open-private}.md`.
- Проверить: `/api/health` (healthcheck деплоя синхронизирован в шаге 1), CORS, swagger/redoc.

## Риски

- Breaking change URL для внешних оверлеев (`/api/queue`, `/ws`) — по данным поиска внешних потребителей нет, фронт правится в шаге 1.
- `stream` разрезан между деревьями (GET public в api, POST key в wapi) — осознанно, без смены защиты.
- Между шагами 1↔2↔3 часть тестов живёт «дважды» (router + сервис) — переходное состояние, устраняется в шаге 3–4.
- Порядок лейеров admin/root — footgun; прикрыт оставшимися full-stack auth-тестами.

## Вне зоны ответственности

Доменный слой (`src/{admin,user,queue,session,roulette,ingress,config,db,error}`), семантика ws, utoipa-tag'и, CORS/деплой-конфиги, права доступа к виджетам. Меняется только организация api-слоя + минимальные правки фронта на пути.
