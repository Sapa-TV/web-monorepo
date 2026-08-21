# План: средние задачи из backlog.md

Статус: **утверждён**, не начат.

Дата: 2026-08-21

Решения:
- M2 — делаем доменное разделение `RuntimeConfig`.
- M3 — линтер на **ast-grep** (не dylint).
- M4 — включаем в этот заход.
- M5 — пункт беклога удаляем без работы (уже сделано).

## Задачи

### M1. Consts → один файл по скоупам

✅ Готово (2026-08-21). Решение изменено в ходе работы: вместо расползания по StaticConfig/RuntimeConfig/TwitchConfig — редкоменяемые tunables собраны в одном файле `apps/backend/src/consts.rs`, разбитом на модули-скоупы (`actions`, `ingress`, `queue`, `session`). Перенесены: `BUS_CAPACITY`, `CHANNEL_CAPACITY`, `DEDUP_WINDOW`, `TWITCH_EVENTSUB_WS_URL`, `TWITCH_RECONNECT_{INITIAL,MAX}_DELAY`, `MAX_PAGE_LIMIT`, `LOGIN_TICKET_TTL`. Протокольные cookie-имена и `INGRESS_SCOPES` остались на местах. Проверки: clippy/fmt чисто, nextest 268/268.

### M2. Доменное разделение RuntimeConfig

✅ Готово (2026-08-21). Плоский `RuntimeConfig` разбит на доменные секции `QueueRuntimeConfig` (`default_limit`, `retention_secs`, `cleanup_interval_secs`), `SessionRuntimeConfig` (`ttl_secs`, `cleanup_interval_secs`), `RouletteRuntimeConfig` (`timeout_secs`) + `widget_access_key`. Внешний формат config-файла/.env сохранён плоским: `RawConfig` маппит ключи в секции в `split()`. Дефолты значений вынесены в `consts.rs` (`queue::*`, `session::*`, `roulette::TIMEOUT_SECS`, `server::PORT`). Проверки: clippy/fmt чисто, nextest 269/269.

### M3. Линтер: максимальная длина api-хендлеров

✅ Готово (2026-08-21). Правило `max-handler-lines` в `.sg/rules/` (ast-grep, `function_item` после атрибута `utoipa::path`) + скрипт-обёртка `tools/check-handler-lines.mjs`: порог **20 строк тела функции** (сигнатура с экстракторами не считается). Попутно ужаты хендлеры-нарушители:
- `rewards.rs::list_rewards` — поход в Twitch API вынесен в `TwitchAuthService::custom_rewards`, ссылки правил — в `RuleService::referenced_reward_ids`;
- `session.rs::twitch_login_callback/create_session/get_me/logout` — переход на `CookieJar` (axum-extra) + `auth_cookie`, маппинг DTO через `From`, обмен кода и выпуск тикета объединены в `SessionService::exchange_login`; `SessionService` теперь сам держит `AdminService` (`login()` без внешнего аргумента);
- `widget_api/users.rs::link_platform/update_platform_username/delete_platform` — общий хвост `user_json`.
Проверки: скрипт OK, ast-grep test 3/3, clippy/fmt чисто, nextest 269/269.

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
