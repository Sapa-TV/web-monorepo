# WS-presence: статусы активных клиентов (докпанель ↔ админка ↔ виджет)

Статус: **план на согласование**. Пункт бэклога #5: «Нужно добавить flow проверки websocket клиента:
В админке нужен статус - есть ли сейчас активные док панель и виджет.
В док панели нужен статус - есть ли сейчас активный виджет».

Дата: 2026-08-23

## Контекст (текущее состояние)

- Один WS-эндпоинт `/wapi/ws` (`widget_api/ws.rs`) для всех клиентов. Хендшейк: первое
  сообщение `{type:"auth", token}`, токен сравнивается с `widget_access_key` через `ct_eq`.
  Сервер **не различает**, кто подключился — виджет или докпанель.
- Клиенты сегодня:
  - `(widgets)/roulette/+page.svelte` — оверлей, слушает `spin_*`;
  - `(panels)/dock/+page.svelte` — докпанель, слушает `spin_*`, умеет `complete`.
    Оба шлют одинаковый `auth` и переподключаются с экспоненциальным backoff.
- Админка в WS не ходит вообще — только REST c сессией (`api/admin.rs::session_router`,
  там же лежит `GET /admin/widget-access-key` как образец).
- `BroadcastEventPublisher` рассылает `SpinEvent` всем авторизованным клиентам (broadcast);
  для presence он не подходит — события будут шуметь и у виджетов.
- В `UniAppState` уже есть похожий по духу `stream_status: Arc<StreamStatus>` (AtomicBool) —
  presence будет его структурным соседом, но с двумя счётчиками и watch-каналом.
- Связанный документ: `docs/archive/../plan-ws-open-private.md` (PRD про открытый/приватный WS,
  не начат). Поле `role` в хендшейке — задел под него, конфликта нет.

## Дизайн

### Термины

Presence = факт «есть сейчас хотя бы один аутентифицированный WS-клиент роли X».
Роли: `dock`, `widget`. Несколько виджетов разрешены (OBS+браузер), важен факт и количество.

### Принцип

Роль объявляется клиентом в хендшейке (`auth.role`), сервер считает аутентифицированные
сокеты по ролям (RAII-guard на соединение), изменения публикуются через `tokio::sync::watch`.
Докпанель получает presence пушами поверх своего существующего WS; админка забирает
новый REST-эндпоинт опросом.

## План

### Шаг 1. Модуль presence (backend)

Новый `src/presence.rs`:

```rust
#[non_exhaustive] pub enum WsClientRole { Dock, Widget }   // serde: "dock" | "widget"
#[non_exhaustive] pub struct PresenceSnapshot { pub dock: usize, pub widget: usize }

pub struct Presence { /* AtomicUsize x2 + watch::Sender<PresenceSnapshot> */ }
impl Presence {
    pub fn new() -> Arc<Self>;
    pub fn add(self: &Arc<Self>, role: WsClientRole) -> PresenceGuard;
    pub fn snapshot(&self) -> PresenceSnapshot;
    pub fn subscribe(&self) -> watch::Receiver<PresenceSnapshot>;
}
```

- `PresenceGuard` — RAII: `Drop` декрементирует и публикует снапшот; guard живёт ровно
  столько, сколько жив обработчик сокета после AuthOk.
- Изменения публикуются в watch при каждом реальном изменении счётчиков.

Тесты модуля: инкремент/декремент по ролям, Drop гварда, уведомления watch.

### Шаг 2. Интеграция в состояние

- Поле `presence: Arc<Presence>` в `UniAppState`; создаётся внутри `assemble_uni_state`
  (тип не зависит от репозиториев — генерики не разъезжаются, фикстура получает автоматически).

### Шаг 3. WS-хендшейк с ролью

`widget_api/ws.rs`:

- `ClientMessage::Auth { token, role }` — поле обязательное (monorepo, деплоятся вместе;
  старые открытые вкладки после релиза просто получат закрытие соединения как при
  malformed-сообщении — то же, что сегодня без auth).
- После `AuthOk`: `let _guard = state.presence.add(role)` на всё время жизни соединения;
  сразу после auth_ok клиенту отправляется начальный `{type:"presence", ...}` снапшот.
- Докпанели (и виджету — одинаково, фильтрация на фронте) форвардим изменения presence:
  в главный `select` добавляется ветка `presence_rx.changed()` → отправка снапшота.

### Шаг 4. REST для админки

`api/admin.rs`: `GET /admin/presence` → `200 { dock_connected: bool, widget_count: usize }`,
в `session_router()` (сессионная защита как у остальных админских GET) + OpenAPI-аннотация.

### Шаг 5. Фронт

- `lib/api.ts`: типы `presence`-сообщения WS; в auth-сообщение добавить `role`;
  функция `fetchPresence()` для админского REST.
- `(widgets)/roulette`: в auth шлёт `role: "widget"`; сообщение `presence` игнорирует.
- `(panels)/dock`: в auth шлёт `role: "dock"`; обрабатывает `presence` → бейдж
  «Виджет: онлайн/офлайн» рядом с индикатором соединения.
- `admin/panel/widgets/+page.svelte`: опрос `GET /admin/presence` раз в 5 сек пока
  страница открыта (`setInterval` + очистка в `onDestroy`), два индикатора:
  «Докпанель» и «Виджет» (онлайн/офлайн + число виджетов).

### Шаг 6. Тесты и проверка

```sh
cargo nextest run --package backend
cargo clippy --all-targets && cargo fmt --check
pnpm --filter frontend check && pnpm --filter frontend test   # если заведутся
```

Плюс ручной smoke: открыть виджет и докпанель → в админке оба онлайн; закрыть виджет →
у докпанели бейдж погас; убить вкладку докпанели → в админке офлайн.

## Осознанно вне скоупа (v1)

- **Ping/pong-liveness**: полуоткрытые TCP (уснувшая OBS-машина) будут числиться
  живыми до закрытия сокета. Если станет проблемой — серверные ping-интервалы +
  таймаут отдельным пунктом бэклога.
- **Push в админку** (вместо опроса): потребует админского WS поверх сессионной
  авторизации — дороже, чем 5-секундный поллинг одной лёгкой ручки.
- **Открытый/приватный WS** из plan-ws-open-private.md — поле `role` лишь готовит почву.

## Оценка

4–6 ч: шаги 1–2 ~1.5 ч, шаг 3 ~1 ч, шаг 4 ~0.5 ч, шаг 5 ~1.5–2 ч, шаг 6 ~0.5–1 ч.
