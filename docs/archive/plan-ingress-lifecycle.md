# План: lifecycle-менеджер ingress-сервисов (start/restart/stop по credentials)

> Статус: **выполнено**.

## Результаты

- **Шаг 1–2.** `PlatformCredentialService<C>` (`src/platform.rs`) — единая точка записи кредов с сигналом
  `watch::Sender<u64>` (ревизия) + `IngressSupervisor<C>` (`src/ingress/supervisor.rs`) с матрицей reconcile
  `(configured, running)` → старт/стоп/рестарт/no-op; стартовый reconcile подхватывает креды из репо после рестарта процесса.
- **Шаг 3.** `UniAppState.credentials: Arc<PlatformCredentialService<C>>` (`src/state.rs`), создаётся один раз в `build()`.
- **Шаг 4.** `AdminAuthService` (`src/admin/auth.rs`) — `complete()`/`revoke_ingress_credentials()`/`is_ingress_credentials_configured()`
  через `PlatformCredentialService` (`save_credential`/`clear_credential`/`load_credential`, с сигналом).
- **Шаг 5.** `TwitchAuthService` (`src/ingress/twitch_auth.rs`) — `Mutex<Option<UserToken>>` удалён; `user_token()` минтит токен
  из репо при каждом вызове; `persist_rotated()` → `save_rotated` (без сигнала, ротация не перезапускает ингресс).
- **Шаг 6.** `src/main.rs` — ad-hoc спавн заменён на `IngressSupervisor` + match-фабрика `build_ingress` (без `Box<dyn>`),
  реестр платформ `[TWITCH]` / `[]` по наличию twitch-конфига.
- **Шаг 7.** `src/test_fixtures.rs` — под новые сигнатуры (ломающего не было), добавлен `save_twitch_credentials`-хелпер.
- **Шаг 8.** Верификация: `cargo nextest run --package backend` 185/185 зелёный, `cargo clippy --all-targets` чисто, `cargo fmt --check` ок.

---

Статус: **запланировано / не начато**. Запись для реализации.

Дата: 2026-08-17

## Назначение

Единый механизм запуска/перезапуска/остановки ingress-сервисов, реагирующий на
credentials: **первое получение** → запуск, **замена** → перезапуск, **отзыв** → остановка.
Работает и после перезапуска процесса (persistентные креды в репо подхватываются на старте).
Универсален по `PlatformId`: будущие платформы (youtube, vk_video_live) получают ту же семантику бесплатно.

## Контекст (текущее состояние)

- Ingress стартует один раз в `apps/backend/src/main.rs:51-67`: если есть `TwitchConfig` → спавнится
  `TwitchPlatformService::run(sink)`. Если креды ещё не сохранены, задача сразу падает в `run()`
  (`user_token()` → `PlatformError::Auth`), перезапуска по появлению credentials нет.
- Креды пишутся напрямую в `PlatformCredentialRepository` в двух местах:
  - `AdminAuthService::complete` (`apps/backend/src/admin/auth.rs:110`) — **получение/замена**;
  - `AdminAuthService::revoke_ingress_credentials` (`apps/backend/src/admin/auth.rs:175`) — **отзыв**;
  - фоново `TwitchAuthService::persist_rotated` (`apps/backend/src/ingress/twitch_auth.rs:91`) — **ротация** refresh-token.
- `TwitchAuthService` кэширует `UserToken` в `Mutex<Option<UserToken>>` (`ingress/twitch_auth.rs:20`).
  После ре-авторизации в репо новый refresh-token, но работающая задача держит **старый кэш** —
  замена чинится только «смертью» инстанса (новый экземпляр на каждом (ре)старте).
- `UniAppState` уже generic над `C: PlatformCredentialRepository` (`apps/backend/src/state.rs`).
- Конвенции: `#[non_exhaustive]`, generic `<R>` у сервисов, RPITIT в трейтах, без `Box<dyn>`/векторов
  (см. `docs/archive/plan-ingress.md`), `thiserror` в `src/error/*`, тесты `#[cfg(test)]` под модулем,
  `cargo clippy`/`cargo test`/`cargo fmt`.

## Решение (обзор)

Вводим `PlatformCredentialService<C>` — единую точку записи credentials, которая владеет сигналом
жизненного цикла (`tokio::sync::watch`). Центральный `IngressSupervisor<C>` слушает сигнал и
reconcile'ит желаемое состояние per-platform: старт/стоп/рестарт. Сервисы строит match-фабрика в
`main.rs` (по конвенции репо — без `dyn`). Фоновая ротация (`save_rotated`) сигнал **не** бьёт —
политика «ротация = рестарт или нет» выбирается адаптером платформы выбором `save_credential` vs `save_rotated`.
Параллельно убираем `Mutex`-кэш токена в `TwitchAuthService` (самовосстановление замены на реконнекте).

**Почему `watch`, а не broadcast/Notify:** `watch` всегда хранит последнее значение — супервизор
никогда не пропустит событие, даже если занят reconcile'ом. `Notify` теряет уведомления в гонке
`notify_waiters`/`notified`, `broadcast` может lag'ол (кейс уже виден в `EventIngress`).
Payload не нужен — только «желаемое состояние изменилось», значение супервизор читает сам из репо.

## Шаги

### Шаг 1. `src/platform.rs`: `PlatformCredentialService<C>`

Новая прослойка над `PlatformCredentialRepository` (закрывает open-question из
`docs/archive/plan-platform-credential.md`). Владеет репо + `watch::Sender<u64>` (монотонная ревизия).

```rust
#[non_exhaustive]
pub struct PlatformCredentialService<C: PlatformCredentialRepository> {
    repo: Arc<C>,
    lifecycle: watch::Sender<u64>,
}

impl<C: PlatformCredentialRepository> PlatformCredentialService<C> {
    pub fn new(repo: Arc<C>) -> Result<Self, RepositoryError>;   // initial state из репо
    pub async fn load_credential(&self, p: PlatformId) -> Result<Option<String>, RepositoryError>;
    pub async fn save_credential(&self, p: PlatformId, c: &str) -> Result<(), RepositoryError>; // bump
    pub async fn save_rotated(&self, p: PlatformId, c: &str) -> Result<(), RepositoryError>;    // no bump
    pub async fn clear_credential(&self, p: PlatformId) -> Result<(), RepositoryError>;          // bump
    pub fn subscribe_lifecycle(&self) -> watch::Receiver<u64>;
}
```

- `save_credential` — админ. получение/замена; `save_rotated` — фоновая ротация (без сигнала);
  `clear_credential` — отзыв. `save_rotated` НЕ бьёт сигнал. Кто какой путь использует — решает
  адаптер платформы (для Twitch: `persist_rotated` → `save_rotated`, без рестарта).
- Тесты: save/clear бодут `watch` (ревизия растёт), `save_rotated` — нет; значение пишется в репо.

Done, когда: `cargo test` зелёный, новые юнит-тесты в модуле.

### Шаг 2. `src/ingress/supervisor.rs` (новый): `IngressSupervisor<C>`

```rust
#[non_exhaustive]
pub struct IngressSupervisor<C: PlatformCredentialRepository> {
    credentials: Arc<PlatformCredentialService<C>>,
    sink: EventSink,
    platforms: &'static [PlatformId],
    spawn: fn(PlatformId, Arc<PlatformCredentialService<C>>, EventSink) -> Option<JoinHandle<()>>,
}
```

Одна задача: `loop { rx.changed().await; reconcile().await; }`; карта `HashMap<PlatformId, AbortHandle>`.

```
reconcile: configured = credentials.load(p).await?.is_some()
    (true,  not running) => spawn             // старт
    (false, running)     => abort             // отзыв
    (true,  running)     => abort + spawn     // замена
    (false, idle)        => no-op
```

- Первый reconcile сразу после старта покрывает «креды уже были в репо, процесс перезапустился».
- `spawn` — сгенерированный `fn`-указатель (match в `main.rs`); каждый вызов строит **новый**
  экземпляр сервиса → кэш токена/конфиг умирают вместе с задачей.
- Тесты на stub-фабрике (запись стартов/остановок в общий state): переходы `None→start`,
  `Some→None→stop`, `Some→new→restart`, `save_rotated` без restart, стартовый reconcile поднимает уже
  сохранённую крепу.

Done, когда: `cargo test` зелёный, тесты покрывают матрицу переходов.

### Шаг 3. `src/state.rs`: проводка `PlatformCredentialService` в `UniAppState`

- Поле `pub credentials: Arc<PlatformCredentialService<C>>` в `UniAppState` (тип-параметр `C` уже есть),
  клон в `impl Clone`.
- `AppStateBuilder::build()` создаёт `Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>>`
  один раз из `self.credentials_repo`, кладёт в state и отдаёт в `AdminAuthService`.

Done, когда: рефакторинг без изменения публичного API роутеров; тесты `test_state` зелёные.

### Шаг 4. `src/admin/auth.rs`: `AdminAuthService` на `PlatformCredentialService`

- ctor: `AdminAuthService::new(config, credentials: Arc<PlatformCredentialService<C>>)`.
- `complete()` (админ. получение/замена) → `credentials.save_credential(PlatformId::TWITCH, ...)`.
- `revoke_ingress_credentials()` → `credentials.clear_credential(PlatformId::TWITCH)`.
- `is_ingress_credentials_configured()` → `credentials.load_credential(...)`.
- Тесты `auth.rs` — обновить на новый ctor.

Done, когда: `cargo test` и `cargo clippy` без предупреждений.

### Шаг 5. `src/ingress/twitch_auth.rs`: сервис + убрать кэш токена

- ctor на `Arc<PlatformCredentialService<C>>` вместо `Arc<C>`.
- `persist_rotated()` → `credentials.save_rotated(PlatformId::TWITCH, ...)`.
- **Убрать `Mutex<Option<UserToken>>`**: `user_token()` при каждом вызове читает refresh-token из репо
  и минтит `UserToken::from_refresh_token` заново. Замена чинится самим ингрессом на следующем
  реконнекте (EventSub шлёт `Revocation` → `consume_loop` выходит → `run()` зовёт `user_token()` →
  свежий токен), даже если супервизорный рестарт запаздает.
- Цена: один лишний mint на реконнект (редкие дисконнекты) — незначимо.
- Тесты: обновить ctor; проверить, что `user_token()` читает изменившуюся крепу.

Done, когда: `cargo test`, `cargo clippy` чисто.

### Шаг 6. `src/main.rs`: супервизор вместо ad-hoc спawn

Заменить блок `main.rs:51-70`:

```rust
fn build_ingress(
    platform: PlatformId,
    credentials: Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>>,
    sink: EventSink,
) -> Option<tokio::task::JoinHandle<()>> {
    match platform {
        PlatformId::TWITCH if twitch_config.is_some() => Some(tokio::spawn(async move {
            let service = TwitchPlatformService::new(Arc::clone(&twitch_config), credentials);
            if let Err(e) = service.run(sink).await {
                tracing::error!("{} ingress stopped: {e}", service.platform().as_name());
            }
        })),
        _ => None,
    }
}

let platforms: &'static [PlatformId] = if config_store.twitch().is_some() { &[PlatformId::TWITCH] } else { &[] };
let supervisor = IngressSupervisor::new(Arc::clone(&state.credentials), state.ingress.sink(), platforms, build_ingress);
tokio::spawn(supervisor.run());
```

- Сохранить флаг `config_store.twitch().is_none()` → «twitch config NOT configured» + пустой реестр.
- Новые типы подключить в `src/ingress.rs` / `lib.rs` при необходимости.

Done, когда: `cargo build`, ручная проверка старта без кредов (ingress не спавнится, нет ошибки в логах).

### Шаг 7. `src/test_fixtures.rs`: коректировки под ctor

- `test_fixtures.rs` и все тестовые сборки state под новые сигнатуры; при желании — тестовый
  helper для «креды сохранены» в state.

Done, когда: полный `cargo test` зелёный.

### Шаг 8. Верификация вручную (e2e)

1. `apps/backend`: `cargo test`, `cargo clippy --all-targets` (deny: `exhaustive_structs`, `new_ret_no_self`), `cargo fmt`.
2. Без кредов: панель — «не авторизовано», ingress не спавнится (нет ошибок в логах).
3. «Авторизовать» → супервизор стартует Twitch ingress; событие `channel.chat.message` доходит до шины
   (лог `ingress event received`).
4. «Отозвать» → ingress останавливается (задача abort).
5. Ре-авторизация → ingress перезапускается, работает с новыми кредами (кэш предыдущей генерации не влияет).
6. Перезапуск процесса с сохранёнными кредами → ingress поднимается сам (reconcile на старте).

## Расширяемость (youtube / vk_video_live)

- `PlatformId`, `PlatformCredentialService`, супервизор уже generic — новых платформ механизм не трогает.
- Для новой платформы: реализовать `PlatformService` (+ её OAuth-админ-флоу в стиле `api/admin/twitch.rs`),
  добавить `PlatformId` в реестр `platforms` и один match-arm в `build_ingress`.
- Политика «ротация = рестарт или нет» — выбор адаптера: `save_credential` (рестарт) vs `save_rotated` (без).

## Открытые вопросы

- Нужно ли супервизору отдавать статус наружу (per-platform `running: bool`) — отдельным API/WS-событием;
  пока не требует, `GET /api/admin/ingress/credentials` остаётся как «настроено/нет».
- Graceful shutdown ингресса на рестарте: `abort()` рвёт задачу резко (Twitch сам почистит подписку по
  таймауту). Если понадобится мягкое завершение — `tokio-util::sync::CancellationToken` в `PlatformService::run`.
- Sqlite-версия `platform_credentials`: опционально поле `version` для сравнения поколений — на заметку,
  сейчас сигнал `watch` покрывает потребность.
