# План: движок правил «событие → действие» (награды/чат → очередь рулетки)

Статус: **план, ожидает подтверждения**. Реализация начнётся только после команды.

Дата: 2026-08-18 (ред. 3)

Прогресс: ✅ шаги 1-13 (шаг 6 выполнен вместе с шагом 5 — типы ошибок нужны сервисам;
`ensure_user_by_platform` вынесен в `UserService` на шаге 10, исполнитель переключён на него;
в шаге 12 «убрать привязку ворка к twitch» — движок и исполнитель запускаются всегда,
twitch в исполнителе опционален: `Option<Arc<TwitchAuthService<C>>>` + `Option<Arc<TwitchConfig>>`,
ChatReply без twitch → ошибка в warn, задача живёт).
Составление пайплайна вынесено в фабрику `runtime.rs::start_rule_pipeline(state)` —
канал шины B и `ActionExecutor` создаются там (не в `UniAppState`); в стейте остаются только
`rule_service`, `action_service`, `twitch_api` (нужны админ-API, шаг 13).
Шаг 13: `api/admin/{rules,actions,rewards}.rs` + OpenAPI-доки (замечание: тест списка наград —
только ветки ошибок 400/401, т.к. для успешного `get_custom_rewards` нужен мок HTTP-клиента).
Осталось: 14-16.

## Контекст (текущее состояние)

- EventSub уже подписан на `channel.channel_points_custom_reward_redemption.add` и
  `channel.chat.message` (`apps/backend/src/ingress/twitch.rs`), события проходят через
  `EventIngress` (`apps/backend/src/ingress/service.rs`) с дедупликацией по event_id.
- Сейчас `RewardRedemption`/`ChatMessage` только логируются (`spawn_logging_handler`,
  `apps/backend/src/state.rs`). Консьюмера, который добавлял бы зрителя в очередь, нет.
- `QueueService::enqueue(user_id, user_name)` (`apps/backend/src/queue/service.rs`) просто
  добавляет запись; дубли между покупками разрешены (каждое событие = новая запись).
- Скоуп `channel:read:redemptions` уже есть в `INGRESS_SCOPES`
  (`apps/backend/src/ingress/twitch_auth.rs`). Для отправки в чат дополнительно нужен
  `user:write:chat` (см. раздел «Решения»).
- Крэйт `twitch_api 0.8.0` умеет `HelixClient::get_custom_rewards`, `send_chat_message`
  и др. (workspace `Cargo.toml` → `twitch_api = "0.8.0"`).

## Архитектура (конвейер, цикл невозможен по построению)

```
Twitch EventSub
  → Platform Ingress Event   (шина A: EventIngress, broadcast<Arc<PlatformEvent>>)
      → RuleEngine: trigger + conditions → собирает ActionEvent
          → ActionEvent      (шина B: mpsc, очередь движок → исполнитель)
              → ActionExecutor (терминальный консьюмер шины B): сайд-эффекты (КОНЕЦ)
```

- Правила читают **только шину A**; движок собирает из совпавшего правила `ActionEvent`
  и публикует его в **шину B**.
- `ActionExecutor` — терминальный консьюмер шины B и единственный исполнитель всех
  экшенов: только получает `ActionEvent` и вызывает сайд-эффекты. Наружу ничего не пишет.
- Маппер не читает собственный выход, исполнитель не имеет доступа к шине A →
  зацикливание исключено.

## Модель домена

```rust
// вход (уже есть) — триггер = автодискриминант от PlatformEventPayload
#[derive(EnumDiscriminants)]                       // +strum, в ingress/event.rs
enum PlatformEventPayload { ChatMessage(..), RewardRedemption(..), .. }
type RuleTrigger = PlatformEventPayloadDiscriminants;   // новый payload → новый триггер сам

// условия по триггеру — консистентность гарантирована конструкцией (имена совпадают с дискриминантами)
enum RuleConditions {
    ChatMessage(MessageConditions),     // matcher + pattern
    RewardRedemption(RewardConditions), // reward_id
}
struct MessageConditions { matcher: MessageMatcher, pattern: Option<String> }
enum MessageMatcher { Contains, StartsWith, Equals, EndsWith }   // расширяемо
struct RewardConditions { reward_id: Option<String> }             // None = любая награда

// actions — отдельный модуль
struct Action { id: ActionId, name: String, kind: ActionKind, enabled: bool, created_at, updated_at }
enum ActionKind {
    NoAction,                                 // пустое действие (no-op), валидный вариант
    EnqueueRoulette,                          // без параметров
    ChatReply { message_template: String },   // интерполяция {username}, {reward_title}, {cost}, {user_input}
}

// правило ссылается на action по id (несколько правил могут переиспользовать один action)
struct Rule { id: RuleId, name: String, enabled: bool, trigger: RuleTrigger,
              conditions: RuleConditions, action_id: ActionId, created_at, updated_at }

// шина B — сообщение между движком и исполнителем
struct ActionEvent {
    source: Arc<PlatformEvent>,   // оригинал с шины A (platform, event_id, payload)
    action_id: ActionId,
    kind: ActionKind,
    ctx: EventContext,            // выжимка payload для шаблонов и сайд-эффектов
}
// EventContext: user_name, text / reward_title, reward_cost, user_input
// шина B = mpsc::Sender<ActionEvent> (tx → движок) / mpsc::Receiver<ActionEvent> (rx → executor)
```

## Решения (подтверждено)

- Расширяемый движок правил: правило = триггер + условия + действие; управляется из админ-панели.
- Хранение — in-memory (как весь текущий бэкенд), персистенс в беклог.
- MVP-фильтры: `RewardId` и `MessageContains` с матчерами `Contains|StartsWith|Equals|EndsWith`.
- MVP-действия: `EnqueueRoulette`, `ChatReply` (шаблоны). Никаких событий наружу —
  исполнитель терминальный.
- Шина B = канал `ActionEvent` (mpsc, ёмкость 256) между движком и исполнителем; даёт
  развязку и бэкпрешер (полный канал → движок ждёт). Обе стороны — отдельные фоновые таски.
- `ChatReply` требует `user:write:chat` в `INGRESS_SCOPES` → **стример один раз ре-авторизуется**
  (кнопка «Авторизовать» в админке); скоупы фиксируются при авторизации.
- Создание наград через Twitch API, автофулфилл, плагины экшенов без кода, персистенс,
  событийный поток наружу (WS/оверлей) — беклог.

---

## Шаги реализации

### 1. Автодискриминант триггера

- `Cargo.toml`: добавить `strum` (derive-фича) в workspace/backend.
- `ingress/event.rs`: `#[derive(EnumDiscriminants)]` на `PlatformEventPayload`;
  ре-экспорт `RuleTrigger = PlatformEventPayloadDiscriminants` (переместить в `rules/rule.rs`).
- Тесты: `payload.discriminant()` возвращает ожидаемый триггер.

### 2. Домен `rules/rule.rs`

- `RuleId(u32)` newtype, `Rule`, `RuleConditions`, `MessageConditions`, `MessageMatcher`,
  `RewardConditions`.
- serde-таггированные enum'ы; unit-тесты на раундтрип и соответствие вариант/условия.

### 3. Домен `actions/action.rs`

- `ActionId(u32)` newtype, `Action`, `ActionKind` (без `EmitEvent`).
- `EventContext` — выжимка из payload (`user_name`, `text` / `reward_title`,
  `reward_cost`, `user_input`, `user_id`).
- Интерполяция шаблонов `{key}`: `fn render(template, &EventContext) -> String`;
  `EventContext` заполняется из payload события.
- Тесты: рендер известных ключей, неизвестный ключ → остаётся как есть.

### 4. Репозитории + in-memory

- `rules/repository.rs`: trait `RuleRepository` (`create/get/list/update/delete`, паттерн
  `AdminRepository`); `db/inmemory_rules.rs`: `Mutex<Vec<Rule>>`, авто-инкремент id, тесты.
- `actions/repository.rs`: trait `ActionRepository` (аналогично); `db/inmemory_actions.rs`,
  тесты.
- Ошибки дублей — существующий `RepositoryError::Conflict`.

### 5. Сервисы

- `rules/service.rs`: `RuleService<R>` — CRUD + валидация (для `StartsWith|Equals|EndsWith`
  `pattern` обязателен; `action_id` должен существовать в ActionService) +
  `subscribe_lifecycle()` через `watch` (паттерн `PlatformCredentialService`,
  `apps/backend/src/platform.rs`) + кэш включённых правил.
- `actions/service.rs`: `ActionService<A>` — CRUD + lifecycle + метод `get(id)`.
- Тесты: валидация, lifecycle-уведомления, кэш.

### 6. Ошибки

- `error/rules.rs`: `RuleServiceError` → StatusCode (400/404/409).
- `error/actions.rs`: `ActionServiceError` → StatusCode (аналогично).

### 7. Шина B: канал `ActionEvent` + сборка `ActionEvent`

- `actions/event.rs`: тип `ActionEvent`, конструктор `ActionEvent::from_action(action, source)`
  из `PlatformEvent` + `Action` (формирует `ctx` из `payload` через
  `From<&PlatformEventPayload> for EventContext`).
- Сам канал `mpsc::channel::<ActionEvent>(256)` создаётся на шаге 12 (wiring, `state.rs`):
  `tx` → движок, `rx` → исполнитель.
- Тесты: сборка `ActionEvent` из chat/reward события, раундтрип через канал.

### 8. Исполнитель: `actions/executor.rs`

- `ActionExecutor` — терминальный консьюмер шины B: `run(rx: mpsc::Receiver<ActionEvent>)`
  фоновая таска. На каждый `ActionEvent` — исчерпывающий `match` по `ActionKind`
  (новый вариант не скомпилируется без ветки). Пока статично (без сворачивания в сервис),
  держит `queue_service`, `user_service`, `twitch_auth: Arc<TwitchAuthService>` +
  `broadcaster_id` из `TwitchConfig` (для `send_chat_message`):
  - `NoAction` → ничего не делать.
  - `EnqueueRoulette` → ensure-пользователь (инлайн: `find_by_platform` → иначе
    `create` + `link_platform`; на шаге 10 вынести в `UserService` и переключить вызов) →
    `queue_service.enqueue`.
  - `ChatReply` → рендер шаблона → `HelixClient::send_chat_message` (broadcaster_id,
    sender_id из токена, message).
- Ничего наружу не публикует; `ExecutorError` (error/executor.rs) только логируется
  (`tracing::warn`, таска не роняется). Тесты: NoAction/Equeue state, переиспользование
  юзера, выживание таски при падении chat-отправки без кредов.
- Ничего наружу не публикует. Ошибки логировать, не ронять таску. Тесты на каждый
  вариант (chat-отправка через мок).

### 9. Движок: `rules/engine.rs`

- `RuleEngine::run(rx: broadcast::Receiver<Arc<PlatformEvent>>, tx: mpsc::Sender<ActionEvent>)`
  — фоновая таска: подписка на шину A + lifecycle правил/экшенов (кэш перезагружается при
  изменении); на событие: `trigger == payload.discriminant()` и `rule.enabled` и
  `action.enabled` → проверить `conditions` → построить `ActionEvent` → `tx.send(...)`.
- Дедуп уже в `EventIngress`; на ошибки логировать. Тесты: совпал/не совпал фильтр,
  матчеры, reload после изменения, движок читает только шину A (не свой выход).

### 10. `UserService::ensure_user_by_platform` (`user/service.rs`)

- Helper: `find_by_platform("twitch", id)` → иначе `create(display_name)` + `link_platform`;
  возвращает `UserId`. Тесты: существующий/новый пользователь.
- Переключить `ActionExecutor::ensure_user` на этот метод (сейчас логика инлайн в
  `actions/executor.rs`).

### 11. Scope: `ingress/twitch_auth.rs`

- В `INGRESS_SCOPES` добавить `Scope::UserWriteChat`.
- В плане релиза — памятка «стримеру пережать Авторизовать».

### 12. Wiring: `state.rs` + `main.rs`

- `UniAppState`: тип-параметры `L: RuleRepository`, `M: ActionRepository`; поля
  `rule_service`, `action_service`, `action_executor`, `twitch_api: Arc<TwitchAuthService<C>>`.
  `AppStateBuilder::build` создаёт репо/сервисы и канал шины B (`mpsc`, ёмкость 256);
  `AppState` alias дополняется.
- `main.rs::start_background_tasks`: `tokio::spawn(rule_engine.run(subscribe(), tx))` и
  `tokio::spawn(action_executor.run(rx))`.

### 13. Admin API ✅

- `api/admin/rules.rs`: `GET /admin/rules` (сессия), `POST|PUT|DELETE /admin/rules[/{id}]`
  (root), DTO `RuleResponse`/`UpsertRuleRequest`.
- `api/admin/actions.rs`: `GET|POST|PUT|DELETE /admin/actions[/{id}]` (аналогично), DTO
  `ActionResponse`/`UpsertActionRequest`.
- `api/admin/rewards.rs`: `GET /admin/rewards` (сессия) — `HelixClient::get_custom_rewards`
  с `GetCustomRewardRequest::broadcaster_id(...)` и `user_token()`; возвращает
  `[{ id, title, cost, is_enabled, is_paused, used_in_rules }]`; ошибки — bare `StatusCode`.
- Подключить в `api/admin.rs` (session_router/root_router), зарегистрировать в
  `lib.rs::MergeSubdocs`; регенерировать `generated/openapi.json` и `packages/api-client`
  (`npm run codegen:rest`).
- Тесты: CRUD, права (root/admin), валидация, список наград (мок).

### 14. Регенерация OpenAPI + api-client (после шага 13)

- Порядок изменён по просьбе: сначала генерим клиент на **уже готовое** API, чтобы фронт
  (шаг 15) писался под актуальный контракт и его можно было сразу проверить.
- `just gen-client` (`cargo run -p gen-openapi` → `apps/backend/generated/openapi.json`,
  затем `swagger-typescript-api` в `packages/api-client`) — новые секции admin rules/actions/
  rewards попадают в `openapi.json` и в клиент.
- Проверка: `apps/frontend` собирается (`npm run check`) с обновлённым клиентом.

### 15. Frontend: `admin/panel/+page.svelte`

- Писать против актуального сгенерированного клиента (шаг 14): новые
  `AdminActionsController` / `AdminRulesController` / `AdminRewardsController`.
- Секция «Действия» (root-only): список (имя, тип, параметры, enabled), форма создания/
  редактирования по типу (для ChatReply — шаблон сообщения; EnqueueRoulette — без параметров).
- Секция «Правила» (root-only): список (имя, триггер, условия, действие, enabled), форма
  (триггер → соответствующие условия: matcher+pattern для чата, reward из
  `GET /admin/rewards` для редимпшена; выбор действия из `/admin/actions`).
- delete с `confirm`, тосты — по паттернам существующих карточек.

### 16. Проверка (перед сдачей)

- Backend: `cargo nextest run --package backend`, `cargo clippy --all-targets`,
  `cargo fmt --check`.
- Frontend: `npm run check`, `npm run lint`, `npm run test:unit -- --run` (в `apps/frontend`).
- OpenAPI/api-client регенерация (шаг 14) не ломает сборку.

---

## Беклог (после MVP)

- Создание наград на Twitch через API (`POST /helix/channel_points/custom_rewards`),
  требует `channel:manage:redemptions`.
- Автофулфилл редимпшенов (`PATCH .../custom_rewards/redemptions`).
- Событийный поток наружу (WS/оверлей) — спроектировать отдельно от шины B, когда
  понадобится (напр., как новые экшены с собственной шиной; сейчас виджет уже читает
  очередь через `/wapi/queue`).
- Новые матчеры/условия, кулдауны и лимиты на юзера, порог стоимости.
- Плагины экшенов без кода (скрипты/UI-редактор).
- Персистенс правил, действий и остальных данных на диск (JSON-репозиторий).
