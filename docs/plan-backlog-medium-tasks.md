# План: средние задачи из backlog.md

Статус: **утверждён**, не начат.

Дата: 2026-08-21

Решения:
- M2 — делаем доменное разделение `RuntimeConfig`.
- M3 — линтер на **ast-grep** (не dylint).
- M4 — включаем в этот заход.
- M5 — пункт беклога удаляем без работы (уже сделано).

## Задачи

### M1. Consts → config с дефолтами

Аудит констант бекенда: настраиваемые (tunables) переезжают в конфиг с default-значениями, протокольные остаются `const`.

Кандидаты на перенос:
- `runtime.rs`: `ACTION_BUS_CAPACITY`
- `queue/service.rs`: `MAX_QUEUE_PAGE_LIMIT`
- `session/service.rs`: `LOGIN_TICKET_TTL`
- `ingress/service.rs`: `CHANNEL_CAPACITY`, `DEDUP_WINDOW`
- `ingress/twitch.rs`: `INITIAL_RECONNECT_DELAY`, `MAX_RECONNECT_DELAY`

Остаются const (протокольные): `SESSION_COOKIE`, `LOGIN_COOKIE`, `EVENTSUB_WS_URL`, `INGRESS_SCOPES`, platform id.

Шаги:
1. Добавить поля в `StaticConfig` (+ `RawConfig`/`Default`) — значения уровня процесса; TTL/делэи можно в static, т.к. runtime-секция для мутируемого.
2. Заменить использования на чтение из конфига (через `AppState`).
3. Тесты: дефолты не меняют поведение существующих тестов.

### M2. Доменное разделение RuntimeConfig

Файлы config уже разделены (static/runtime/twitch/repository/store), «common» не существует — пункт частично устарел. Оставшаяся работа: разбить плоский `RuntimeConfig` (roulette_timeout + retention + queue_* + session_ttl в одной структуре) на доменные секции: `QueueConfig`, `SessionConfig`, `RouletteConfig`.

Шаги:
1. Создать секционные структуры с `Default` из текущих значений.
2. `RuntimeConfig` становится композицией секций; сериализация сохраняет совместимость ключей (`queue_default_limit` и т.д.) либо мигрирует на вложенные ключи — проверить `store.rs`/persisted settings.
3. Обновить `StaticConfig::split`, `validate`, тесты, места чтения (`state.rs`, сервисы).

### M3. Линтер: максимальная длина api-хендлеров

Инструмент: **ast-grep** (`@ast-grep/cli` уже в devDeps корня).

Правило: функция с атрибутом `#[utoipa::path]`, тело длиннее **20 строк** → ошибка.

Замер (2026-08-21): 32 хендлера, при пороге 20 укладываются 28/32 (88%). Нарушители:
- `rewards.rs`: `list_rewards` (41)
- `session.rs`: `create_session` (37), `twitch_login_callback` (35)
- `session.rs`: `get_me` (21)

Шаги:
1. Добавить правило в `sgconfig.yml` (создать при отсутствии); ограничение длины — через служебный скрипт-обёртку, если чистый YAML этого не умеет.
2. Прогнать по `apps/backend/src/api/**`.
3. Отдельным проходом вынести инлайн-логику четырёх нарушителей в сервисы (куки, twitch-config, листинг наград).

### M4. Поиск администратора по username

Цель: при добавлении админа вводишь username — twitch_id и display_name подставляются сами.

Бекенд:
1. Новый эндпоинт `GET /api/admin/twitch/users?query=<login>` (root-guard): через `TwitchAuthService::helix()` → `get_users` (по login), вернуть `[{id, login, display_name}]`.
2. utoipa-схема + `gen-client`.

Фронт (`AdminsCard.svelte`):
3. Поле поиска с дебаунсом вместо ручного ввода Twitch ID; результат — список кандидатов, клик заполняет форму.
4. Fallback: ручной ввод ID остаётся (если twitch не настроен).

### M5. Админка: страницы и навигация — уже сделано

✅ Готово (2026-08-21). Пункт удалён из backlog.md без работы: отдельные роуты `admin/panel/{actions,admins,roulette,widgets}` + платформы на главной, `SidebarMenu`, layout-guard существуют.

## Порядок

M5 (удаление пункта) → M1 → M2 (после решения) → M3 → M4.

Проверка после каждого шага: `cargo nextest run --package backend`, `cargo clippy --all-targets`, `cargo fmt`; фронт — `pnpm --filter frontend run lint`, `run check`, vitest.
