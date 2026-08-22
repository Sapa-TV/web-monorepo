# OpenAPI: миграция роутеров на utoipa-axum (query-параметры как `in: query`)

Статус: **план готов, не начато**.

Дата: 2026-08-22

Задача из `backlog.md`: бекенд выдаёт query-параметры в openapi.json как `in: path`
(у `GET /api/queue` status/limit/cursor, у `GET /api/users` platform/platform_user_id и т.п.),
в сгенерированном клиенте они объявлены аргументами, но в запрос не попадают.
Обходной путь во фронте: ручная передача `{ query: { ... } }`
(`apps/frontend/src/lib/admin/creds.ts:15`, `apps/frontend/src/lib/admin/session.ts:41`).

## Причина бага (проверено по исходникам utoipa 5.5.0)

Цепочка из трёх звеньев:

1. Роутеры собраны обычным `axum::Router`, а не `utoipa_axum::OpenApiRouter` —
   utoipa не может вывести расположение параметров из сигнатур хендлеров
   (`apps/backend/src/widget_api.rs:23-43`, `apps/backend/src/api.rs:41-55`).
2. В атрибуте `#[utoipa::path]` параметры переданы голым типом: `params(ListQuery)`
   (`widget_api\queue.rs:132-159`, `users.rs:140-166`). Макрос при этом жёстко подставляет
   provider `|| None`.
3. Структуры параметров имеют derive `IntoParams`, но **без**
   `#[into_params(parameter_in = Query)]`. Тогда каждое поле получает
   `parameter_in(None.unwrap_or_default())`, а `Default for ParameterIn` в utoipa — это
   **`Path`**. Бонус: при `Path` принудительно ставится `"required": true`
   (поэтому у `Option`-полей стоит required) и `Option<T>` рендерится как `oneOf [null, $ref]`.

Итог виден в `generated/openapi.json:971-1019` (`/wapi/queue`) и `:1330-1353` (`/wapi/users`):
все параметры `"in": "path", "required": true`; swagger-typescript-api превращает их
в позиционные аргументы клиента, которые не отправляются
(`packages/api-client/generated/Wapi.ts:60-71`).

## Решение: миграция на `utoipa_axum::OpenApiRouter`

Устраняет класс проблем навсегда:

- пути регистрируются в **одном** месте (роутере); сейчас их три: роутер,
  `#[utoipa::path]`, и списки `paths(...)` в 12 структурах `XxxApiDoc`;
- фича **`axum_extras`** у utoipa резолвит `parameter_in` из сигнатуры хендлера
  (`Query<T>` → query, `Path<T>` → path) даже для голых типов в `params(...)` —
  подтверждено докой utoipa-gen 5.5 (раздел «axum_extras feature support»);
- `routes!(...)` автоматически собирает схемы компонентов (тест
  `openapi_with_auto_collected_schemas` в исходниках utoipa-axum 0.2) —
  ручные списки `components(schemas(...))` больше не нужны;
- `OpenApiRouter::nest()` префиксует пути в спеке — самописный `MergeSubdocs`
  с ручным мержем доков удаляется.

Версии уже подключены: workspace `utoipa = "5.5.0"`, `utoipa-axum = "0.2.0"`
(`Cargo.toml:30-31`, `apps/backend/Cargo.toml:24-25`).

API `utoipa_axum 0.2` (проверено на docs.rs):
`new/with_openapi/routes/route/route_layer/layer/nest/merge/with_state/split_for_parts/into_openapi`,
`From<Router<S>> for OpenApiRouter<S>`.

## Шаги реализации

### 1. Workspace `Cargo.toml`

```toml
utoipa = { version = "5.5.0", features = ["axum_extras"] }
```

### 2. Модули-роутеры (~13 файлов)

`widget_api\{queue,rarities,roulette_slots,users,stream}.rs`,
`api.rs`, `api\{stream,session}.rs`, `api\admin.rs`,
`api\admin\{twitch,ingress,actions,roulette,rules,rewards}.rs`:

- `router() -> axum::Router<AppState>` → `router() -> OpenApiRouter<AppState>`
  через `OpenApiRouter::new().routes(routes!(handler1, handler2, ...))`;
- комбинированные маршруты (`.route("/admin/rules/{id}", put(update_rule).delete(delete_rule))`)
  превращаются просто в два хендлера внутри одного `routes!(...)`;
- ws-роутер (`widget_api\ws.rs`, `/ws` без аннотации) остаётся обычным Router,
  подмешивается через `.into()`;
- аннотации `#[utoipa::path]` и списки `params(ListQuery)` **остаются как есть** —
  теперь они корректно резолвятся благодаря `axum_extras`.

### 3. Удалить дубли

12 структур `XxxApiDoc` (+ их `paths(...)`/`components(schemas(...))`) и модификатор
`MergeSubdocs` в `apps/backend/src/lib.rs:30-79`.

### 4. Верхний уровень

В каждом модуле две функции поверх общего чистого билдера:

- `openapi_router() -> OpenApiRouter<AppState>` — только сбор маршрутов/спеки, без стейта
  (ключ к тому, чтобы gen-openapi не тянул БД/конфиг);
- `router(state: AppState) -> Router` — навешивает auth-слои pass-through
  `route_layer` (`require_key`/`require_session`/`require_admin`/`require_root`),
  делает `.nest("/wapi"|"/api", ...)`, `with_state(state)`,
  `split_for_parts().0` → обычный `Router` для main.rs.

### 5. Спека наружу

- `lib.rs`: корневой `ApiDoc` остаётся только с `info(...)`/`tags(...)`;
  добавить
  ```rust
  pub fn openapi() -> OpenApi {
      OpenApiRouter::<AppState>::with_openapi(ApiDoc::openapi())
          .nest("/wapi", widget_api::openapi_router())
          .nest("/api", api::openapi_router())
          .into_openapi()
  }
  ```
- `main.rs:121-122`: SwaggerUi/Redoc получают `backend::openapi()` вместо `ApiDoc::openapi()`;
- `tools/gen-openapi/src/main.rs`: `backend::openapi().to_pretty_json()` вместо `ApiDoc::openapi()`.

### 6. Аннотировать `/health` и `/version` (решено: да)

Сейчас единственные неаннотированные REST-роуты (`api.rs:24-25`) — добавить
`#[utoipa::path]` и включить в `routes!`, чтобы попали в спеку.

### 7. Верификация

```sh
cargo check --package backend
cargo clippy --all-targets
cargo fmt
cargo nextest run --package backend
just gen-client
```

- diff `generated/openapi.json`: у `/wapi/queue` и подобных — `"in": "query"`,
  `required` соответствует опциональности полей, пути `/wapi/*` на месте;
- фронт: убрать обходные пути — `creds.ts`/`session.ts` передают `code`/`state`
  позиционными аргументами (клиент сам шлёт их в query); обновить тесты
  `creds.test.ts`/`session.test.ts`, прогнать тесты фронта;
- отметить задачу в `backlog.md`.

## Решённые вопросы

1. `/health` и `/version` — аннотировать, включить в спеку.
2. Списки `params(...)` в аннотациях — оставить (это объявление параметров, не дублирование
   регистрации путей; расположение теперь выводится из сигнатуры хендлера).
3. Русские throw-сообщения в `creds.ts` — вне скоупа (отдельная задача backlog
   про обработку ошибок).

## Риски / заметки

- operationId берётся из имён функций — не меняется; имена методов клиента стабильны.
- Известные коллизии operationId между widget/admin (`list_slots`) уже есть и никуда не деваются.
- Тесты в `api\admin.rs:186+` строят приложение через `router(state)` — сигнатура сохраняется,
  изменения минимальны.
