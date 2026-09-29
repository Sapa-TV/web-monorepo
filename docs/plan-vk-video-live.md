# Интеграция VK Video Live

Статус: **план на согласование**. Пункт бэклога #1.
Дата: 2026-08-26

## Контекст

VK Video Live (бывш. VK Play Live, live.vkvideo.ru) — третья платформа,
`PlatformId::VK_VIDEO_LIVE = 3` уже существует и засеяна в БД
(`migrations/0003_platforms_and_credentials.sql`), поведения нет нигде.
Эталон интеграции — Twitch: ingress через EventSub WS, OAuth-креды в
`platform_credentials`, админский connect-flow, экшен send_chat_message.

Публичный REST+WS API у платформы есть (неофициально подтверждён SDK):
OAuth2 (code → access/refresh, revoke, scopes incl. channel_points),
`GET channel/{name}` со статусом стрима и адресом chat-websocket
(Centrifugo), каталоги каналов. Официальная документация — за JS-стеной
dev.vk.com, прямые swagger-URL не отвечают.

## Что уже готово в коде (переиспользуем)

- Абстракция ingress: `trait PlatformService { platform(); run(sink, shutdown) }`
  (`ingress/platform.rs`) — единственный контракт новой платформы.
- Конвейер событий: `EventIngress` (dedup + broadcast), sealed
  `PlatformEvent::{chat_message, reward_redemption}`.
- Креды: одна opaque-строка на платформу (`PlatformCredentialRepository`,
  для Twitch — refresh token) + lifecycle/restart через
  `PlatformCredentialService`, супервизор стартует/останавливает ingress по
  появлению/отзыву кредов (`ingress/supervisor.rs`).
- Экшены: `trait PlatformActionExecutor` + dispatch в
  `actions/service.rs` (сейчас match только по TWITCH).
- Frontend-паттерн: `TwitchPlatformCard.svelte` + popup-callback +
  регенерируемый api-client.

## Исследование (2026-08-26, завершено)

Официальная документация: dev.live.vkvideo.ru (контент через
`api.live.vkvideo.ru/devconsole/v1/doc/page/{id}`, YAML в base64).

### OAuth (подтверждено доками + SDK)

- Authorize: `https://auth.live.vkvideo.ru/app/oauth2/authorize`
  (`client_id`, `redirect_uri`, `response_type=code`, `scope` через запятую, `state`).
- Token: `POST https://api.live.vkvideo.ru/oauth/server/token`,
  Basic `client_id:secret`, form-urlencoded; grant_types
  `authorization_code` / `refresh_token` / `client_credentials`.
  Ответ: `access_token`, `refresh_token`, `expires_in`, `token_type=Bearer`.
- Revoke: `POST /oauth/server/revoke`.
- Приложение регистрируется в кабинете разработчика на dev-портале.

### REST (всё `https://api.live.vkvideo.ru`, Bearer access_token)

- `GET /v1/channel?channel_url=` → `channel.id` (числовой),
  `channel.status`, `web_socket_channels.{chat,info,channel_points,...}`,
  `stream.{status,started_at,viewers,title,category}`.
- `GET /v1/chat/messages?channel_url&limit≤200` — история для бэкфилла.
- `POST /v1/chat/message/send?channel_url&stream_id`, тело
  `{parts:[{text:{content}}]}`; scope `chat:message:send`; ошибки
  `send_too_fast` / `same_message` / `message_too_long`. На сайте бот-лимит
  10 сек между сообщениями.
- Есть также `chat/member(s)`, `chat/settings`, `channel_points` (доки есть —
  задел под RewardRedemption позже).

### WebSocket (Centrifugo v4, JSON, `cf_protocol_version=v2`)

- `GET /v1/websocket/token` (Bearer, доступно и app-auth) → connection JWT.
- Connect: `wss://pubsub.live.vkvideo.ru/connection/websocket?format=json&cf_protocol_version=v2`.
- Подписки публичные без токенов: `channel-chat:{id}` (чтение чата),
  `channel-info:{id}` (статус стрима); limited-каналы — через
  `GET /v1/websocket/subscription_token?channels=`.
- События: `{"push":{"channel":...,"pub":{"data":{"type":...,"data":...}}}}`.
  Чат: `type:"message"` (легаси) и `"message_v8"`; текст в блоках
  `data[].content` — JSON-строка вида `[\"ку\",\"unstyled\",[]]`
  (нулевой элемент = текст). Маппинг: event_id=`data.id`,
  sent_at=`data.createdAt`, user_id=`author.id`, user_name=`author.nick`.

### Вывод

Все неизвестные закрыты, блокеров нет. Отправка сообщений официально
доступна (scope `chat:message:send`) — экшен send_chat_message реализуем
через `POST /v1/chat/message/send`.

## Решения (приняты)

- Connect-flow: тот же root админ на той же странице админки, где
  добавляются twitch credentials — карточка VK рядом с карточкой Twitch
  (`VkPlatformCard` по образцу `TwitchPlatformCard`).
- Креды: храним refresh token; для простоты рядом access token и время
  истечения — всё в одной opaque-строке существующей колонки
  `platform_credentials[3]` (JSON-блоб), схема БД не меняется.
  Структуру блоба знает только `VkAuthService`.
- `StreamStatus` глобальный флаг не трогаем (пер-платформенный статус —
  отдельная задача, не блокирует).
- Именование: платформа пишется целиком — env `VK_VIDEO_LIVE__*`,
  секция `[vk_video_live]` в config.toml, файл `config/vk_video_live.rs`,
  тип `VkVideoLiveConfig` (VK — отдельная платформа, может добавляться
  отдельно; не сокращаем до `vk`).

### Redirect-схема (платформенно-Scoped, мигрируем и Twitch)

Redirect_uri указывает на страницы фронтенда (SPA), которые дергают
backend API с code/state. Пути разделяем по платформам:

- `/login-callback/twitch` — вход на сайт (не только админы) и админка
- `/creds-callback/twitch` — подключение бот-кредов канала
- `/login-callback/vk-video-live` — вход через VK (пригодится для чата
  сайта; регистрируем в приложении VK сразу)
- `/creds-callback/vk-video-live` — подключение кредов VK

Фронтенд: один generic callback-компонент с параметром платформы вместо
двух отдельных страниц; старые `/admin/login` и `/admin/creds/callback`
уходят. Обновить зарегистрированные redirect_uri: в приложении VK
(точное совпадение до символа) и в Twitch console для twitch-пары.
Env: `TWITCH__REDIRECT_URI`, `TWITCH__CREDENTIALS_REDIRECT_URI`,
`VK_VIDEO_LIVE__REDIRECT_URI`, `VK_VIDEO_LIVE__CREDENTIALS_REDIRECT_URI`.

Backend API при этом остаётся раздельным: `/auth/{platform}/callback`,
`/admin/{platform}/auth/callback`.

### Отдельный крейт для VK

Протокольная логика VK — в отдельном lib-крейте `crates/vk-video-live`
(workspace member), бэкенд только склеивает:

- `auth.rs` — OAuth exchange/refresh/revoke, типы токенов;
- `api.rs` — REST клиент: `/v1/channel`, `/v1/chat/messages`,
  `/v1/chat/message/send`, `/v1/websocket/token`;
- `pubsub.rs` — Centrifugo v4 JSON-клиент (connect/subscribe/push,
  ping 25s, reconnect с backoff);
- `events.rs` — типизированные события чата + парсинг текстовых блоков;
- `error.rs`.

В бэкенде остаются: `config/vk_video_live.rs`, `ingress/vk_video_live.rs`
(PlatformService), `ingress/vk_video_live_auth.rs` (жизненный цикл кредов),
`api/admin/vk_video_live.rs`, `actions/vk_video_live_executor.rs`. Крейт
тестируется на фикстурных фреймах изолированно от бэкенда.

Примечание: манифесты правит пользователь вручную:

1. корневой `Cargo.toml` → `[workspace]` → `members`: добавить
   `"crates/vk-video-live"`;
2. `apps/backend/Cargo.toml` → `[dependencies]`: добавить
   `vk-video-live = { path = "../../crates/vk-video-live" }`.

### Что продумать заранее (чек-лист до шага 1)

- **CSRF/state обобщение**: `AdminAuthService` твичевский (один
  pending_csrf store) — ключом сделать платформу, билдеры authorize-URL
  per-platform, не сломав twitch flow.
- **Refresh-цикл**: refresh по `expires_at` с запасом и при 401;
  обновлённый блоб через `save_rotated` (без рестарта ingress);
  `save_credential` только на connect/revoke.
- **Блоб кредов**: `{access_token, refresh_token, expires_at,
user{id,nick}, channel{id,url}}` — канал резолвится при connect
  (`GET /v1/channel` по channel_url авторизованного юзера) и хранится,
  чтобы ingress не ходил в REST на старте лишний раз.
- **Отправка чата**: официальный `POST /v1/chat/message/send` требует
  `stream_id` (берём из `GET /v1/channel`); офлайн → ActionError;
  сериализация отправок с интервалом ≥10s, при `send_too_fast` — дроп
  с логом.
- **Дедуп**: числовые id сообщений VK в существующее окно dedup ложатся
  как есть.
- **Статус стрима**: не трогаем (ручной глобальный флаг); подписка на
  `channel-info:{id}` — позже.
- **Ошибки**: `PlatformError::TwitchApi` обобщить (добавить VkApi или
  PlatformApi).
- **Статус кредов в админке**: `GET /admin/ingress/credentials`
  сейчас twitch-only — расширить до per-platform (OpenAPI + api-client
  перегенерация).
- **Фичефлаг**: конфиг VK отсутствует → платформа не стартует и в
  списке супервизора её нет (симметрично twitch).

## Шаги реализации (по образцу twitch, шаг = коммит)

0. Redirect-миграция: новые фронтовые маршруты `/login-callback/{platform}`,
   `/creds-callback/{platform}` (generic-компонент), обновить env и
   зарегистрированные URI в Twitch console и приложении VK, api-client
   регенерация. Twitch flow должен остаться зелёным.
1. Крейт `crates/vk-video-live`: скелет + `auth.rs` (OAuth по докам) +
   `error.rs`. Юнит-тесты на построение запросов/парсинг ответов.
2. Крейт: `api.rs` (channel, chat history/send, ws token) + `events.rs`
   (парсинг message/message_v8 на фикстурных фреймах из devtools).
3. Крейт: `pubsub.rs` (centrifugo v4: connect/subscribe/push/ping,
   reconnect backoff). Тесты на фреймах.
4. Бэкенд: `config/vk_video_live.rs` (`VkVideoLiveConfig { client_id,
client_secret, redirect_uri, credentials_redirect_uri }`) + поле
   `vk_video_live` в `StaticConfig` (+ env пример без секретов).
5. Бэкенд: `ingress/vk_video_live_auth.rs` (блоб кредов, refresh-цикл) +
   `ingress/vk_video_live.rs` (PlatformService поверх крейта). Регистрация
   в `main.rs` (список платформ + build_ingress).
6. Бэкенд: обобщение CSRF/state в `AdminAuthService` под платформы;
   `api/admin/vk_video_live.rs` (auth + callback); per-platform статус
   кредов в `/admin/ingress/credentials`.
7. Бэкенд: `actions/vk_video_live_executor.rs` (send_chat_message с
   интервалом ≥10s) + dispatch в `actions/service.rs`; обобщение
   `PlatformError`.
8. Фронтенд: `VkPlatformCard.svelte`, брендовый цвет, регенерация
   api-client и openapi.json.
9. Финал: полный регресс (nextest, clippy, fmt, openapi чистый).

## Риски

- Часть API полуофициальная (сокет-события могут пополняться/меняться);
  парсинг событий делаем устойчивым к неизвестным `type`.
- Лимит отправки чата ~10 сек/сообщение (send_too_fast) — в экшене
  предусматриваем очередь/дроп с логом.
- Ребрендинг доменов продолжается — все URL в константах одного модуля.

Оценка: исследование завершено (0 ч остатка), реализация 4–6 ч.
