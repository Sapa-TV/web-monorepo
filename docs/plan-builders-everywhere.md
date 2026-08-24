# Builder/new для всех структур: конструкторное создание как единственный путь

Статус: **план на согласование**. Пункт бэклога #1: «Переделать создание структур — все
структуры должны создаваться в одном месте - через методы new\build\builder, для этого
во все добавить не публичное zero-size поле ()».

Дата: 2026-08-24

## Цель и правило

Любая `pub struct` с публичными полями получает приватное zero-size поле `_priv: ()`,
закрывающее литеральное создание вне модуля определения. Единственный путь создания —
конструктор (`new`/`from_*`/builder), объявленный в том же модуле. Приватные поля
структур (сервисы, сторы) уже сегодня нельзя собрать литералом — они считаются
соответствующими правилу без изменений.

Проверено пробником: **serde-derive совместим** с `_priv: ()` — сгенерированный код
живёт в модуле структуры и видит приватное поле; `ToSchema`/`IntoParams` тоже.
`#[non_exhaustive]` остаётся (внешние крейты) и дополняет, а не заменяет `_priv`.

## Инвентаризация

Всего `pub struct`: **147**. Разбивка по зонам: api/admin 19, db/sqlite 11,
widget_api/users 10, widget_api/queue 7, user 6, api/session 6, api/admin.rs 5,
session 4, queue/entry 4, config/runtime 4, rules/rule 4, остальные по 1–3.

Категории обработки:

| Категория                                       | Примеры                                              | Действие                                               |
| ----------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------ |
| A. Домен-энтити, `new()` уже есть               | QueueEntry, Action, RouletteSlot, Rarity, User, Rule | добавить `_priv`, литералы заменить на `new`           |
| B. DTO-ответы, собираются From-ом в своём файле | AdminResponse, WidgetAccessKeyResponse…              | добавить `_priv`; From-имплементации остаются на месте |
| C. Request-payloads (Deserialize)               | UpsertRuleRequest…                                   | добавить `_priv`                                       |
| D. Newtype/marker                               | SessionToken(String), StandartRandomProvider         | пропустить                                             |
| E. Сервисы/сторы с приватными полями            | все *Service, Presence, EventIngress                 | пропустить (уже соответствуют)                         |

Точную численность категорий A–C фиксируем на шаге 1 (скрипт-аудит + ручная сверка),
она и станет чек-листом шагов 2+.

## Риски (проверено/митигировано)

- serde + `_priv` — компилируется (пробник в корне крейта прошёл check).
- Тесты, собирающие литералы домен-структур (Action{..}, PlatformEvent через ::new
  уже ок) → переходят на конструкторы; там, где конструктору нужны лишние аргументы
  (created_at/updated_at), добавляется тестовый хелпер в сам модуль типа
  (`#[cfg(test)] pub(crate) fn fixture(...)`) вместо раскрытия полей.
- `Default` выводить массово не будем: явные конструкторы — часть цели.

## Шаги

**Протокол одного микро-шага** (= одна структура, один коммит):
правка структуры (`_priv` + конструктор при отсутствии) → `cargo check --all-targets`
→ минимальные фиксы литералов → `nextest` (полный, ~5с) + clippy + fmt → ревью.
Порядок следования — чек-лист ниже; внутри домена сверху вниз.

1. **Аудит**: скрипт строит чек-лист всех pub struct: определяющий файл,
   наличие `new`, число файлов с литеральным использованием вне модуля.
   Категории D/E (newtype/приватные поля) в чек-лист не попадают.
2. **Пилот: `QueueEntry`** — одна структура, фиксируется шаблон диффа.
   3+. **По чек-листу**, домен за доменом: queue → user/platform → roulette →
   actions/rules → session/admin/config → ingress/state → api → widget_api.
   Если структура тянет за собой правки >3 файлов или конструктор требует
   спорных решений — стоп, обсуждаем до продолжения.

Финальный шаг: снять оставшиеся предупреждения clippy точечно, полный регресс,
итоги в этом документе, бэклог минус пункт.

## Шаг 1 — выполнен: чек-лист структур

Кандидаты: pub struct, у которых есть литеральное использование вне модуля
или отсутствует конструктор. Newtype/marker/уже-соответствующие — исключены
(55 шт.). Колонка «Лит.» — число файлов с `\bИмя\s*{` вне defining-файла;
при правке конкретной структуры сверяемся руками (возможны ложные срабатывания
на impl/тестах).

| Структура                  | Файл                     | new | Лит. |
| -------------------------- | ------------------------ | --- | ---- |
| Action                     | actions/action.rs        | ✓   | 7    |
| ActionId                   | actions/action.rs        | ✓   | 2    |
| EventContext               | actions/action.rs        | ✓   | 1    |
| ActionEvent                | actions/event.rs         | —   | 1    |
| ActionContext              | actions/platform.rs      | —   | 3    |
| Admin                      | admin.rs                 | —   | 2    |
| RarityResponse             | api/admin/roulette.rs    | —   | 1    |
| RouletteSlotResponse       | api/admin/roulette.rs    | —   | 1    |
| StreamStatusResponse       | api/stream.rs            | —   | 1    |
| QueueRuntimeConfig         | config/runtime.rs        | —   | 1    |
| RouletteRuntimeConfig      | config/runtime.rs        | —   | 1    |
| RuntimeConfig              | config/runtime.rs        | —   | 1    |
| SessionRuntimeConfig       | config/runtime.rs        | —   | 1    |
| StaticConfig               | config/static_config.rs  | —   | 3    |
| SharedSettings             | config/store.rs          | ✓   | 1    |
| TwitchConfig               | config/twitch.rs         | —   | 7    |
| ApiError                   | error/api.rs             | ✓   | 8    |
| ChatMessage                | ingress/event.rs         | —   | 0    |
| PlatformEvent              | ingress/event.rs         | —   | 1    |
| RewardRedemption           | ingress/event.rs         | —   | 0    |
| InMemoryPlatformRepository | db/inmemory_platform.rs  | —   | 0    |
| Platform                   | platform.rs              | ✓   | 1    |
| PlatformId                 | platform.rs              | ✓   | 3    |
| Presence                   | presence.rs              | ✓   | 1    |
| PresenceSnapshot           | presence.rs              | ✓   | 1    |
| QueueEntryId               | queue/entry.rs           | ✓   | 1    |
| QueuePage                  | queue/entry.rs           | ✓   | 1    |
| RouletteSlot               | roulette/slot_service.rs | ✓   | 1    |
| MessageConditions          | rules/rule.rs            | ✓   | 6    |
| RewardConditions           | rules/rule.rs            | ✓   | 5    |
| Rule                       | rules/rule.rs            | ✓   | 6    |
| LoginTicket                | session.rs               | ✓   | 4    |
| Session                    | session.rs               | ✓   | 4    |
| UniStateParams             | state.rs                 | ✓   | 1    |
| ResolvedUserPlatform       | user.rs                  | ✓   | 1    |
| User                       | user.rs                  | ✓   | 3    |
| UserId                     | user.rs                  | ✓   | 1    |
| UserPlatform               | user.rs                  | ✓   | 2    |
| UserView                   | user.rs                  | ✓   | 2    |
| ApiDoc                     | lib.rs                   | —   | 0    |

Однострочные DTO без литералов и без new (категория C, 52 шт.) — идут после
таблицы сплошным проходом по файлам api/** и widget_api/**:
AddAdminRequest, AdminResponse, PresenceResponse, TwitchIdParam,
WidgetAccessKeyResponse, ActionIdParam, ActionResponse, UpsertActionRequest,
IngressCredentialsResponse, RewardResponse, RarityIdParam, SlotIdParam,
UpsertRarityRequest, UpsertRouletteSlotRequest, RuleIdParam, RuleResponse,
UpsertRuleRequest, TwitchAuthCallbackQuery, TwitchAuthCallbackResponse,
TwitchAuthStartResponse, TwitchUserResponse, TwitchUserSearchQuery,
CreateSessionRequest, SessionResponse, TwitchLoginCallbackQuery,
TwitchLoginCallbackResponse, TwitchLoginStartResponse, Unauthorized,
StreamStatusResponse(wapi), AnonymousEnqueueRequest, EnqueueRequest, ListQuery,
NextResponse, QueueEntryResponse, QueueIdParam, QueueListResponse,
SetStreamStatusRequest, CreateUserRequest, FindUserQuery, LinkPlatformRequest,
PlatformNameParam, PlatformResponse, UpdatePlatformRequest, UpdateUserRequest,
UserIdParam, UserPlatformResponse, UserResponse.

## Верификация после каждого шага

```sh
cargo check --package backend --all-targets
cargo nextest run --package backend
cargo clippy --all-targets && cargo fmt --check --package backend
```

Дополнительно на шаге 2 (пилот): grep-контроль отсутствия литералов
`QueueEntry {` вне defining-модуля.

## Осознанно вне скоупа

- Браузерный e2e/playwright трек.
- Введение builder-библиотек (derive_builder и пр.) — ручные конструкторы достаточны.
- Изменение публичных API JSON — контракты DTO не меняются, только внутренний
  способ создания.

## Оценка

Уточняется на шаге 1. Исходно из бэклога 3–6 ч выглядит оптимистично при ~90+
структурах категорий A–C; ожидаемый диапазон после аудита — **6–9 ч** суммарно
(шаги 3–8 по 0.5–1.5 ч каждый). Пилот покажет реальную скорость на структуру.
