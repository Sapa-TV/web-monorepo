# Builder/new для всех структур: конструкторное создание как единственный путь

Статус: **план на согласование**. Пункт бэклога #1: «Переделать создание структур — все
структуры должны создаваться в одном месте - через методы new\build\builder, для этого
во все добавить не публичное zero-size поле ()».

Дата: 2026-08-24

## Цель и правило

Любая `pub struct` с публичными полями получает приватное zero-size поле `_sealed: ()`,
закрывающее литеральное создание вне модуля определения (и внутри крейта тоже).
Единственный путь создания — конструктор (`new`/`from_*`/семантический конструктор),
объявленный в том же модуле. Структуры, у которых все поля уже приватны (сервисы,
сторы), правилу соответствуют без изменений.

`#[non_exhaustive]` **остаётся везде** как крейт-вайд конвенция (workspace deny
`exhaustive_structs`) и дополняется `_sealed: ()` — он добавляет недостающее:
запрет литерального создания внутри крейта. Конфликт линтов решён глобально:
в `[workspace.lints.clippy]` добавлено `manual_non_exhaustive = "allow"` (линт
предлагает убрать `_sealed` в пользу атрибута, что противоречит цели; в перспективе
его роль возьмёт кастомный sg-lint «структура обязана иметь конструктор»).

Проверено пробником: serde-derive / ToSchema / IntoParams совместимы с `_sealed: ()`.

### Шаблон диффа одной структуры (из пилота Action)

```rust
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct X {
    pub field: ...,
    _sealed: (),
}

impl X {
    #[allow(clippy::too_many_arguments)] // если аргументов много
    pub fn new(/* все pub-поля */) -> Self { ... }
}
```

Нюансы, найденные пилотом:

- Литералы вне модуля → `X::new(...)`.
- **sqlx**: `query_as!(Domain, ...)` несовместим с `_sealed` (макрос строит литерал
  на месте). Паттерн репозиториев: `query!` (compile-time проверка SQL и колонок
  сохраняется) + явный маппинг строки через `X::new(...)` — это и есть «создание
  в одном месте». Если маппинг распухнет от числа колонок — приватная Row-структура
  внутри файла репо под `query_as!`.
- **Structures with `Serialize`**: на `_sealed` обязательно `#[serde(skip)]`,
  иначе поле уедет в JSON ответов (`"_sealed": null`). Проверено на RouletteSlotResponse.
- Дубликаты DTO между api/** и widget_api/** (одинаковые имена, разные типы):
  сводятся в один тип рядом с доменом (`roulette/dto.rs`), оба роутера импортируют;
  OpenAPI при этом байт-в-байт стабилен.
- Struct-update синтаксис (`..base`) не пересекает модульную границу (E0451) —
  разворачивается в явные присваивания или заменяется конструктором.
- Для контекст-структур с `Default` добавляются семантические конструкторы
  (`EventContext::chat(...)`, `::reward(...)`), использующие `..Self::default()`
  внутри своего модуля.
- Тестовые литералы → `new` либо локальный fixture-хелпер.
- `derive(Default)` совместим с `_sealed` (`()` тоже `Default`) — для маленьких
  Copy-типов достаточно позиционного `new(a, b)`, serde-skip не нужен, если
  структура сама не сериализуется.
- Если конструктор сбрасывал `created_at` на now при апдейте — это пре-существующий
  баг, фиксируется отдельным пунктом бэклога, в рефакторинге поведение сохраняется.

## Инвентаризация (аудит выполнен)

Всего `pub struct`: **147**. После классификации:

| Категория                                         | Кол-во | Действие                                         |
| ------------------------------------------------- | ------ | ------------------------------------------------ |
| Newtype/unit/уже с приватными полями              | ~55    | пропустить                                       |
| Запечатать — pub-поля, но литералов пока нет (B0) | ~10    | только `_sealed`, по одной за шаг вперемешку с A |
| Запечатать — с внешними литералами                | **24** | таблица A ниже, по структуре за шаг              |
| Bulk-DTO без литералов                            | **52** | только `_sealed` в своём файле, сплошной проход  |

«Литералов пока нет» ≠ «соответствует правилу»: `_sealed` гарантирует
конструкторный путь навсегда, поэтому pub-поля структур без текущих литералов
тоже запечатываются.

### B0. Только `_sealed` — pub-поля без внешних литералов (~10)

~~QueueEntry · QueueStats~~ — готово (конструкторы `new` уже были, добавлен `_sealed`,
один чанк с QueuePage). ~~Rarity~~ — готово (поля pub(crate), `new` был, добавлен
`_sealed`). Остались: RouletteSlot · PlatformEvent · ChatMessage ·
RewardRedemption · Platform · StreamStatus — идут вперемешку со списком A внутри
того же домена.

## Шаги

**Протокол микро-шага** (= одна структура, один коммит): правка структуры
(`_sealed` + конструктор) → `cargo check --all-targets` → фиксы литералов →
nextest + clippy + fmt → ревью. Порядок — список A, затем B0 (внутри доменов),
затем bulk B.

1. ~~Аудит~~ — выполнен, результат ниже.
2. ~~Пилот~~ — выполнен: `Action` (+ попутно `EventContext`), шаблон диффа выше.
   3+. По списку A, затем bulk B.

Финал: снять точечные allow, полный регресс, итоги здесь, бэклог минус пункт.

### A. По одной структуре за шаг (24)

| Структура             | Файл                    | Внешние литералы                                                   |
| --------------------- | ----------------------- | ------------------------------------------------------------------ |
| ActionContext         | actions/platform.rs     | executor, twitch_executor, service                                 |
| Admin                 | admin.rs                | db/inmemory_admin, db/sqlite/admin                                 |
| RarityResponse        | api/admin/roulette.rs   | widget_api/rarities                                                |
| RouletteSlotResponse  | api/admin/roulette.rs   | widget_api/roulette_slots                                          |
| StreamStatusResponse  | api/stream.rs           | widget_api/stream                                                  |
| QueueRuntimeConfig    | config/runtime.rs       | config/static_config                                               |
| RouletteRuntimeConfig | config/runtime.rs       | config/static_config                                               |
| RuntimeConfig         | config/runtime.rs       | config/static_config                                               |
| SessionRuntimeConfig  | config/runtime.rs       | config/static_config                                               |
| StaticConfig          | config/static_config.rs | store, admin/twitch, admin/rewards                                 |
| TwitchConfig          | config/twitch.rs        | admin/auth, ingress/twitch, ingress/twitch_auth, api/admin/rewards |
| ApiError              | error/api.rs            | error/{admin,config,actions,rules,user}                            |
| Presence              | presence.rs             | exempt: все поля приватные                                         |
| PresenceGuard         | presence.rs             | exempt: все поля приватные (вне исходного списка)                  |
| PresenceSnapshot      | presence.rs             | готово: `new(dock, widget)`; литералы в тестах presence/ws заменены |
| QueuePage             | queue/entry.rs          | готово: `new(entries, next_cursor)`; литерал в service заменён     |
| MessageConditions     | rules/rule.rs           | sqlite/rule, parity, service, api, inmemory                        |
| RewardConditions      | rules/rule.rs           | engine, inmemory, sqlite, service, parity, api                     |
| Rule                  | rules/rule.rs           | те же + api                                                        |
| LoginTicket           | session.rs              | sqlite/session, service, inmemory, parity                          |
| Session               | session.rs              | те же                                                              |
| ResolvedUserPlatform  | user.rs                 | user/service                                                       |
| UserPlatform          | user.rs                 | db/inmemory_user, db/sqlite/user                                   |
| User                  | user.rs                 | db/inmemory_user, db/sqlite/user                                   |
| UserView              | user.rs                 | user/service, widget_api/users                                     |

### B. Bulk-DTO (52), сплошной проход

api/admin.rs (5) · api/admin/actions.rs (3) · api/admin/ingress.rs (1) ·
api/admin/rewards.rs (1) · api/admin/roulette.rs (5) · api/admin/rules.rs (3) ·
api/admin/twitch.rs (6) · api/session.rs (5) · widget_api/queue.rs (7) ·
widget_api/stream.rs (2) · widget_api/users.rs (11)

## Верификация после каждого шага

```sh
cargo check --package backend --all-targets
cargo nextest run --package backend
cargo clippy --all-targets && cargo fmt --check --package backend
```

## Осознанно вне скоупа

- Браузерный e2e/playwright трек.
- Builder-библиотеки (derive_builder и пр.) — ручные конструкторы достаточны.
- Изменение публичных JSON API — контракты DTO не меняются, только способ создания.
- Массовое снятие `#[non_exhaustive]` со структур вне чек-листа — по мере касания.

## Оценка

После аудита: **24 микро-шага категории A** (~3–5 мин каждый с учётом фикс-апов)

- **52 bulk-DTO** (~1–2 ч сплошняком) + финал ≈ **4–6 ч**, верхняя граница исходной
  оценки бэклога подтверждена.
