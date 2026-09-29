# Рефакторинг ActionExecutor: платформенные экшены через трейт

Статус: **выполнен** (2026-08-24). Пункт бэклога #3: «Разбить ActionExecutor на 3 компонента».

Дата: 2026-08-24

## Цель

`ActionExecutor` перестаёт знать про конкретные платформы. Взаимодействие с API
платформ уходит в пер-платформенные исполнители за общим трейтом; отсутствие
возможности у платформы — это default-метод трейта, который логирует warning.
Добавление новой платформы = новая реализация трейта + одна строка в реестре.

## Контекст (текущее состояние)

- `ActionExecutor<Q, R, S, U, P, C>` (actions/executor.rs, 362 строки) — консьюмер
  шины B: `run(rx)` → `execute(event)` по `ActionKind`:
  - `NoAction` — пусто;
  - `EnqueueRoulette` — платформенно-агностик (`ensure_user_by_platform` +
    `queue_service.enqueue`);
  - `ChatReply { message_template }` — **жёстко прибит к Twitch**: рендер шаблона,
    затем `twitch_auth.user_token()` → `helix.send_chat_message(broadcaster_id, ...)`
    (executor.rs:96–113). Поля `twitch_auth: Option<...>` и `broadcaster_id`.
- Уже есть прецедент пер-платформенного трейта: `PlatformService`
  (ingress/platform.rs) для ingress-воркеров — но это другой жизненный цикл
  (бесконечный run), переиспользовать его для экшенов нельзя.
- `PlatformId` — закрытый enum (Twitch/Youtube/VK_Video_Live) с `as_name()`.
- Юнит-тесты executor'а собирают настоящий `TwitchAuthService` ради двух тестов
  ChatReply — после рефакторинга им понадобится только фейковый исполнитель.
- `async-trait` в зависимостях нет.

## Целевая архитектура (имена из бэклога)

```
шина B (mpsc<ActionEvent>)
        │
        ▼
ActionExecutor          ← оркестрация: ensure_user, рендер шаблона, диспетчеризация
        │  по kind:
        │   EnqueueRoulette → queue_service (без платформ)
        │   ChatReply       → platform_actions.send_chat_message(event.platform, ...)
        ▼
PlatformActionService   ← матчер: PlatformId → конкретный исполнитель (enum-match)
        │
        ▼
PlatformActionExecutor  ← трейт возможностей (нативные async fn, без dyn)
  ├─ TwitchActionExecutor      — helix.send_chat_message
  └─ отсутствие исполнителя    → warning «platform X does not support Y» + Err
```

Ключевое решение — **default-методы с warning**: трейт объявляет все возможности;
платформа реализует только то, что умеет; базовая реализация пишет
`tracing::warn!(platform, action, "capability not supported")` и возвращает
`Err(ActionError::Unsupported)` — экшен честно падает в существующий warn-лог
исполнителя вместо тихого молчания.

## Диспетчеризация: статическая по enum

Платформы известны на момент компиляции (≤20 штук), на ходу не добавляются ⇒
**без `dyn` и без async-trait**: нативные `async fn` в трейте легальны, реестр
хранит `Option<TwitchActionExecutor>` (и будущие поля на платформу), выбор —
обычный `match` по `PlatformId`. Плюсы: минус зависимость, статический диспатч,
проще тесты (spy реализует трейт напрямую). Цена: добавление платформы трогает
match-руку сервиса — приемлемо при компайл-тайм списке.

## План

### Шаг 1. Контракты: `src/actions/platform.rs`

```rust
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub event_id: String,
    pub user_id: String,
    pub user_name: String,
    pub channel_id: String,     // broadcaster/канал на платформе события
}

pub enum ActionError {
    Unsupported,            // default-метод трейта
    Api(String),
}

pub trait PlatformActionExecutor: Send + Sync {
    fn platform(&self) -> PlatformId;

    async fn send_chat_message(&self, ctx: &ActionContext, text: &str)
        -> Result<(), ActionError>
    {
        tracing::warn!(
            platform = self.platform().as_name(),
            action = "send_chat_message",
            "capability not supported"
        );
        Err(ActionError::Unsupported)
    }

    // будущие возможности добавляются сюда с тем же default-паттерном:
    // async fn add_reward(...) / autofill_redemption(...) ...
}
```

Нативные `async fn` в трейте (edition 2024, без dyn — см. решение выше).
`channel_id`: broadcaster_id сейчас берётся из TwitchConfig; в контексте он нужен
любой платформе (у VK/YouTube свой аналог). Источник значения — конфиг платформы
при создании исполнителя (см. шаг 2), не глобальный TwitchConfig.

### Шаг 2. Twitch-реализация: `src/actions/twitch_executor.rs`

`TwitchActionExecutor<C>::new(config: Arc<TwitchConfig>, auth: Arc<TwitchAuthService<C>>)`:

- `platform()` → `PlatformId::TWITCH`;
- `send_chat_message` — тело текущего executor.rs:97–112 переносится как есть
  (user_token → sender_id → helix.send_chat_message(ctx.channel_id...));
  ошибки мапятся в `ActionError::Api`.

### Шаг 3. Матчер: `src/actions/service.rs`

```rust
#[non_exhaustive] // нет — сервис собирается builder'ом; поля приватные
pub struct PlatformActionService {
    twitch: Option<TwitchActionExecutor>,
    // youtube: Option<YouTubeActionExecutor>,   ← будущее добавляется сюда + match-рука
}

impl PlatformActionService {
    pub fn new() -> Self;                                       // все поля None
    pub fn with_twitch(mut self, e: TwitchActionExecutor) -> Self;
    pub fn is_empty(&self) -> bool;

    pub async fn send_chat_message(
        &self, platform: PlatformId, ctx: &ActionContext, text: &str,
    ) -> Result<(), ActionError> {
        match platform {
            PlatformId::TWITCH => match &self.twitch {
                Some(e) => e.send_chat_message(ctx, text).await,
                None => unsupported(platform, "send_chat_message"),   // warn + Err
            },
            other => unsupported(other, "send_chat_message"),
        }
    }
}
```

- Регистрация в main.rs/runtime.rs: если twitch сконфигурирован — `with_twitch`,
  иначе сервис пуст (все вызовы уйдут в warning-ветку).
- Тесты: незарегистрированная платформа/пустой сервис → `Unsupported` + ветка
  warning; зарегистрированная Twitch — вызов доходит до исполнителя.

### Шаг 4. Перевод ActionExecutor

- Поля `twitch_auth`, `broadcaster_id` удаляются; вместо них
  `platform_actions: Arc<PlatformActionService>` (генерик `C` у executor'а
  исчезает — она уходит в TwitchActionExecutor).
- `ChatReply`: render(template, ctx) → `platform_actions.send_chat_message(
event.platform, &action_ctx, &text).await`; ошибка логируется существующим
  образом (task survives — семантика сохраняется).
- `runtime.rs start_rule_pipeline`: собрать сервис из twitch_config и передать в
  ActionExecutor::new; сигнатура main.rs не меняется.

### Шаг 5. Тесты

- **Юниты executor** упрощаются: вместо `TwitchAuthService` — spy-исполнитель,
  реализующий `PlatformActionExecutor` (записывает вызовы send_chat_message);
  сервис собирается с ним; проверяем, что ChatReply вызывает исполнителя платформы
  события и передаёт отрендеренный текст ({username} и т.п.); путь без регистрации
  → ошибка Unsupported.
- **Юниты TwitchActionExecutor**: без сети — маппинг ошибок auth в ActionError::Api
  (токена нет → Api), happy-path не тестируем (нужен live Twitch).
- **Юниты PlatformActionService**: пустой сервис → Unsupported; with_twitch → вызов
  доходит.
- Юзкейс `rules_pipeline` остаётся зелёным (EnqueueRoulette путь не меняется).

### Шаг 6. Финализация

```sh
cargo check --package backend
cargo nextest run --package backend
cargo clippy --all-targets && cargo fmt --check
```

Ручной smoke: сообщение в чате с правилом ChatReply — ответ появляется
(требует живого Twitch, вне CI).

## Осознанно вне скоупа

- Реальные AddReward/autofill — методы появятся в трейте с default-warning,
  реализации придут вместе с пунктами «админка наград»/«автофулфилл».
- YouTube/VK исполнители — вызовы уйдут в warning-ветку сервиса до появления
  их ingress.
- Вынос `ensure_user` из executor'а — он платформенно-агностичен и остаётся частью
  оркестрации.

## Оценка

3–4.5 ч: контракты ~0.5 ч, twitch-реализация ~1 ч, сервис ~0.5 ч, перевод
executor + runtime ~0.75–1 ч, тесты ~0.75 ч, регресс ~0.25 ч.

## Итог (отличия от плана)

Выполнено полностью; nextest 372 passed (+5 новых), clippy/fmt чисто.
Отклонения:

- **Диспетчеризация статическая** (решение на ревью плана): без `dyn` и
  async-trait; `PlatformActionService<T>` дженерик по исполнителю, выбор —
  `match` по `PlatformId`.
- Следствие: `ActionExecutor` сохранил параметр-дженерик — теперь это
  `T: PlatformActionExecutor` вместо `C: PlatformCredentialRepository`.
  Платформенной логики в нём по-прежнему нет; тесты получили чистый spy.
- `ExecutorError::Chat(String)` заменён на `Platform(#[from] ActionError)`.
- **`channel_id` в ActionContext пока заполняется пустой строкой**: Twitch-
  исполнитель берёт broadcaster из своего конфига (`TwitchConfig.broadcaster_id`),
  поле — задел под платформы, где target живёт в событии.
- Юниты twitch_executor добавлены сразу в шаге 2 (не 5): маппинг отсутствия
  токена в `ActionError::Api` без сети.
- Тесты executor переведены на in-memory фикстуру `test_state_inmemory()`
  вместо sqlite `test_state_with`; spy сделан клонируемым через Arc-inner,
  чтобы страница/тест держали хэндл к записанным вызовам после переезда
  исполнителя в сервис.
- Ручной smoke с живым Twitch остаётся за эксплуатацией.
