# Ingress: единая точка приёма событий с платформ

## Цель

Обобщённый сервис приёма **любых** событий с платформ: chat message сейчас, позже — подписка / фоллоу / награды за баллы канала и т.д.

- Трейт `PlatformService` — реализуется каждой платформой.
- Центральный сервис принимает нормализованные события от всех адаптеров и публикует их во **внутреннюю** шину; внутренние обработчики (подписчики) их обрабатывают и вызывают другие сервисы.
- Это **внутренняя** шина — НЕ та, что отдаёт события наружу виджету/док-панели (там отдельный `BroadcastEventPublisher`).
- Модель события расширяемая: `PlatformEvent::ChatMessage` — первая реализация, дальше добавляются варианты.
- Реализуется только Twitch (через `twitch_api2`, EventSub WebSocket).

## Контекст (что уже есть в коде)

- События наружу: `broadcast::channel` + trait-публишер (`SpinEventPublisher`/`BroadcastEventPublisher`) — `src/event.rs`, `src/queue/events.rs`.
- **Важно про две шины.** Существующий `BroadcastEventPublisher` — **внешний**: он отдаёт события наружу (виджету, док-панели и т.д., через WS). Новый `EventIngress` — **внутренний**: платформы (твитч/вк/ютуб) шлют события во **внутреннюю шину**, которую потребляет внутренний обработчик, а он уже вызывает другие сервисы (queue, users и т.д.). Это две разные шины с разными потребителями, не путать.
- `UniAppState<Q,R,U,P,S>` — generic, `AppStateBuilder` собирает всё в `src/state.rs`.
- `Platform`/`PlatformRepository` в `src/platform.rs` — это **DB-концепция** (id+name, засеяны twitch/youtube/vk_video_live). Новый трейт `PlatformService` — про **ингейшн**, не путать.
- Конвенции: `#[non_exhaustive]`, `thiserror` в `src/error/*`, `clippy::unwrap_used` deny, edition 2024, тесты в `#[cfg(test)]` под каждым модулем.

## Новые модули

```
src/ingress/                 ← новый домен (приём событий с платформ)
  event.rs                   ← PlatformKind, PlatformEvent, PlatformEventPayload, ChatMessage
  platform.rs                ← trait PlatformService (+ EventSink)
  service.rs                 ← EventIngress (внутренняя шина) + LoggingHandler (заглушка-потребитель)
  twitch.rs                  ← TwitchPlatformService (twitch_api2 EventSub WS)
src/error/ingress.rs         ← IngressError (PlatformError)
```

`main.rs`: `mod ingress;`, `src/error.rs`: `pub mod ingress;`.

## 1. Модель — `ingress/event.rs`

Единый «конверт» события: платформа + время + payload.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlatformKind { Twitch, YouTube, VkVideoLive }

impl PlatformKind {
    pub fn as_name(&self) -> &'static str { ... }  // "twitch" | "youtube" | "vk_video_live"
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct PlatformEvent {
    pub platform: PlatformKind,
    pub sent_at: DateTime<Utc>,
    pub payload: PlatformEventPayload,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlatformEventPayload {
    ChatMessage(ChatMessage),
    // позже:
    // Subscription { .. }, Follow { .. }, ChannelPointsReward { .. }, ...
}

#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ChatMessage {
    pub user_id: String,      // platform-scoped user id
    pub user_name: String,
    pub text: String,
}
```

- `PlatformEvent` — единый конверт; расширение = новый вариант `PlatformEventPayload`.
- `platform`/`sent_at` вынесены в конверт, **не** дублируются внутри `ChatMessage` (адаптер знает платформу).
- `sent_at` — `Utc::now()` на ингресте (в payload twitch нет таймстампа сообщения).
- `PlatformKind::as_name()` — мост к существующей привязке юзеров (UserService резолвит по имени платформы).

## 2. Платформенный трейт — `ingress/platform.rs`

```rust
pub type EventSink = tokio::sync::mpsc::Sender<PlatformEvent>;

pub trait PlatformService: Send + Sync {
    fn kind(&self) -> PlatformKind;
    async fn run(&self, sink: EventSink) -> Result<(), PlatformError>;
}
```

**Без `dyn` и без вектора.** Платформ всего ~3 (twitch/youtube/vk_video_live), возможно ещё пара — динамическая диспетчеризация и `Vec<Box<dyn PlatformService>>` тут не нужны. Адаптеры **не хранятся** в `EventIngress`: каждый адаптер конструируется и спавнится отдельным `tokio::spawn` в `main`, получая общий `EventSink` (клоны mpsc-sender). Поэтому:

- трейт использует **нативный** `async fn` в трейте (edition 2024, RPITIT) — никакого `async-trait` и dyn-совместимости не требуется;
- `EventSink` — конкретный mpsc-sender (YAGNI).

Адаптер переводит сырое платформенное событие в нормализованный `PlatformEvent` и пушит в `EventSink`.

## 3. Внутренняя шина — `ingress/service.rs`

`EventIngress` — **внутренняя шина**: принимает события со всех платформ в `sink` и перекладывает их во внутренний `broadcast`, откуда их потребляют внутренние сервисы:

```rust
pub struct EventIngress {
    sink: mpsc::Sender<PlatformEvent>,            // раздаётся адаптерам (вход)
    out: broadcast::Sender<Arc<PlatformEvent>>,   // внутренняя шина (внутрь, не наружу)
}

impl EventIngress {
    pub fn new() -> Self;                            // спавнит pump-loop
    pub fn sink(&self) -> EventSink;                 // клон для каждого адаптера
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<PlatformEvent>>;
    pub async fn publish(&self, event: PlatformEvent) -> Result<(), PlatformError>;  // тест-хук
}
```

Поток: `TwitchPlatformService` (и позже youtube/vk) → пушат `PlatformEvent` в `sink` → pump-loop перекладывает их во **внутреннюю** `out`-шину → внутренние сервисы подписаны (`subscribe()`) и обрабатывают; по необходимости они вызывают другие сервисы (queue, users и т.д.).

- `new()` создаёт канал `(sink, rx)` и спавнит **pump-loop**: `rx` → broadcast внутренней шины. Ошибка broadcast при «нет подписчиков» — warn, не fatal (как в `event.rs`).
- Адаптеры спавнятся **вне** `EventIngress` (в `main`): `tokio::spawn(async move { adapter.run(ingress.sink()).await })`, упавший адаптер логируется.
- Потребители — внутренние сервисы, подписанные через `subscribe()`. **Заглушка** на этом этапе — `LoggingHandler`: отдельная задача, подписывается на шину и пишет `tracing::info!` (без логики). Точка расширения: сюда подключаются команды→очередь, фоллоу→баллы и т.д., фильтруя `payload` по типу.
- Внутренняя шина НЕ отдаёт события наружу — это не `BroadcastEventPublisher` (тот для виджета/док-панели).

## 4. Twitch-адаптер — `ingress/twitch.rs`

- Тип `TwitchPlatformService`; поля `config` (client_id/access_token/broadcaster_id).
- EventSub **WebSocket**: connect → Welcome (session_id) → подписка `channel.chat.message` (TransportMethod::WebSocket) → `Payload::parse` → нотификация `ChannelChatMessageV1`.
- **Чистый маппинг** `PlatformEvent::from_twitch(notif)` → `PlatformEvent { platform: Twitch, payload: ChatMessage{ chatter_user_id, chatter_user_name, message.text } }` — отдельная функция без сети, юнит-тестируется; `message_id` — для будущего дедупа.
- Reconnect/retry — минимальный (лог + выход), отдельной задачей позже.
- WS-слой: `tokio-tungstenite` (новая dep) **или** companion-крейт — уточняется при реализации, зависит от уже проверенного теста.

## 5. Конфиг

В `Config` — опциональный блок (`#[derive(Clone, Deserialize)] #[non_exhaustive]`):

```rust
pub struct TwitchConfig {
    pub client_id: String,
    pub access_token: String,
    pub broadcaster_id: String,
}
// Config { ..., pub twitch: Option<TwitchConfig> }  // default None
```

`Option`, чтобы тесты и запуск без твича не падали. Креды — env/config.toml (`TWITCH_CLIENT_ID`, `TWITCH_ACCESS_TOKEN`, `TWITCH_BROADCASTER_ID`).

## 6. Связывание в State

- `UniAppState` + `pub ingress: Arc<EventIngress>` (конкретный тип, не generic).
- `AppStateBuilder::build` создаёт `EventIngress::new()` — без адаптеров → тесты (`test_state`) не коннектятся к сети.
- В `main.rs` после `build`: спавн адаптера и заглушки-потребителя:
  ```rust
  tokio::spawn(spawn_logging_handler(state.ingress.subscribe()));       // заглушка
  if let Some(tw) = &state.config.twitch {
      let adapter = TwitchPlatformService::new(tw.clone());
      tokio::spawn(async move { if let Err(e) = adapter.run(state.ingress.sink()).await { tracing::error!(...); } });
  }
  ```

## 7. Dependencies

- `twitch_api2` с фичами `["eventsub", "helix", "client", "twitch_oauth2"]` (в `apps/backend/Cargo.toml`); `tokio-tungstenite` — на подтверждение. `async-trait` **не нужен** (нативный `async fn` в трейте).

## 8. Тесты

- `event.rs`: сериализация `PlatformKind`/`PlatformEvent` roundtrip, tag-based `PlatformEventPayload`.
- `service.rs`: `publish` → подписчик получает; без подписчиков — нет паники.
- `platform.rs`/`twitch.rs`: `StubPlatformService`, пишущий в sink → доставка в шину (без сети); для twitch — только чистый маппинг из нотификации (фикстура).
- Проверка: `cargo clippy` (bacon), `cargo test`, `cargo fmt`.

## Открытые вопросы

- WS-библиотека для EventSub-коннекта (tokio-tungstenite vs twitch_eventsub) — зависит от уже проверенного теста.
- Нужен ли дедуп по `message_id` сейчас или позже.
- Форма будущих вариантов `PlatformEventPayload` (`Subscription`, `Follow`, `ChannelPointsReward`) — их DTO пока не проектируем, только модель-скелет.