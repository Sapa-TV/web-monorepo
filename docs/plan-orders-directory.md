# План: справочник заказов (игры, фильмы, VIP)

Источник данных сейчас: Google Sheet «ПлАнИрОвАнИе» (`1Tv9aAfZTM1oGV9Pk-ANSJz--UiR_IObe1GlvkZhLPdk`),
вкладки `Игры`, `Фильмы`, `VIP\Unvip` (плюс `Мерч`, `Доска почета`, `Игры на посмотреть` — вне скопа v1).

## Решения

- **Сайт — источник истины.** Админ редактирует в админке сайта. Google Sheet — только зеркало:
  синк сайт → таблица при изменениях (debounce) и/или раз в сутки + кнопка «синхронизировать сейчас».
- Статус в таблице переделывается с цвета заливки на текстовую колонку / чекбоксы (как в `Мерч`),
  цвета — через условное форматирование на стороне Sheets. Цвета мы не читаем и не пишем.
- Скоп v1: игры + фильмы (публичные страницы) + VIP (только админка).
- VIP: только учёт и напоминания, без авто-выдачи/снятия. Автоматизация (рулетка → запись в список) — отдельным этапом через движок правил.

## Модель данных (миграции 0011+)

Общее: `created_at`, `updated_at`. Порядок строк в таблице = `ORDER BY id`, отдельный `number` не нужен.

**Заказчик** — два поля сразу, чтобы будущая автоматизация не потребовала переделок:

- `customer_name TEXT NOT NULL` — отображаемое имя (денормализовано; в Sheet всё равно уходит строкой);
- `user_id INTEGER REFERENCES users(id) ON DELETE SET NULL` — опциональная связь с `users` (миграция 0007).
  При ручном создании NULL, автоматизация (рулетка) будет резолвить/создавать `users` + `user_platforms`
  и заполнять. При импорте из таблицы можно пробовать матчить по `user_platforms.platform_username`
  (case-insensitive) — отдельная опция импорта, не обязательная в v1.

```sql
-- 0011_game_orders.sql
CREATE TABLE game_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT,                           -- NULL = заказчик ещё не выбрал игру
    customer_name TEXT NOT NULL,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    kind TEXT NOT NULL,                   -- 'stream' | 'playthrough'
    source TEXT NOT NULL DEFAULT 'other', -- 'donate' | 'points' | 'roulette' | 'other'
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'completed' | 'cancelled'
    completed_at TEXT,                    -- date
    comment TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 0012_movie_orders.sql
CREATE TABLE movie_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT,
    customer_name TEXT NOT NULL,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    kind TEXT NOT NULL DEFAULT 'movie',   -- 'movie' | 'series' | 'anime' | 'youtube'
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'watched' | 'cancelled'
    comment TEXT,
    created_at ..., updated_at ...
);

-- 0013_vip_records.sql
CREATE TABLE vip_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_name TEXT NOT NULL,
    user_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
    kind TEXT NOT NULL,                   -- 'vip' | 'unvip'
    roulette_date TEXT NOT NULL,          -- date выигрыша/проигрыша в рулетку
    end_date TEXT NOT NULL,               -- vip: +14 дней, unvip: +7 дней
    status TEXT NOT NULL DEFAULT 'active',-- 'active' | 'done' | 'cancelled'
    note TEXT,
    created_at ..., updated_at ...
);
```

Поле `source` нормализует то, что сейчас смешано в колонке «Стрим \ прохождение»
(`стрим (рулетка)`, `прохождение (донат)`, `стрим (баллы)`). При импорте парсим суффикс.

## Backend

Новые доменные модули по существующему шаблону (trait + service + sqlite + inmemory + parity-тест):

- `src/orders/` — `game.rs`, `movie.rs`, `vip.rs` (repository.rs, service.rs), или три отдельных модуля
  `game_orders/`, `movie_orders/`, `vip/` — по аналогии с `roulette/`, `queue/`.
- `db/sqlite/game_order.rs`, `movie_order.rs`, `vip_record.rs` + inmemory + регистрация в `state.rs` (`AppStateBuilder`) и `db/parity.rs`.
- После запросов — `just sqlx-prepare` (офлайн-метаданные `.sqlx`).

### API

Публичное (`public_router()` в `api.rs`, без авторизации), новый файл `src/api/orders.rs`:

- `GET /api/orders/games?status=&kind=&source=&q=` — поиск `q` по title/customer (SQL `LIKE`), фильтры, сортировка по `number`.
- `GET /api/orders/movies?status=&kind=&q=`.

Админ (`api/admin/orders.rs`, в `session_router()` под `require_admin`):

- CRUD: `POST/PUT/DELETE /api/admin/orders/games[/:id]`, то же для `movies` и `vip`.
- `POST /api/admin/orders/sync` — принудительный синк в Google Sheet.
- Ответы на utoipa-аннотациях, DTO с `#[non_exhaustive]` + `_sealed`, контроль ast-grep-правил
  (no control flow в api/, лимит строк хендлера).

### Google Sheets синк (write-only)

- Крейт: `reqwest` уже есть; OAuth2 service-account JWT делаем вручную (JWT + `jsonwebtoken`/`rsa` либо
  готовый `yup-oauth2`) — выбрать `yup-oauth2`, если не тянет лишнего, иначе самописный JWT (ES256 не нужен, RS256).
- Конфиг: `GOOGLE__SERVICE_ACCOUNT_KEY_PATH` (json ключ в sops-секретах), `GOOGLE__SPREADSHEET_ID`.
  Таблицу расшарить на email сервисного аккаунта с правами редактора.
- Сервис `sheets_sync/`: полная перезапись диапазона вкладки (`values:batchUpdate` / `values.update`)
  по снапшоту из БД (`ORDER BY id`). Триггеры:
  - debounce (например 30 сек) после любого CRUD через событие в `event.rs`-шину или прямой вызов из сервиса,
  - раз в сутки фоновой задачей по паттерну `start_background_tasks` (`main.rs:187-204`),
  - вручную через `POST /api/admin/orders/sync`.
- ID таблицы храним в конфиге (см. импорт ниже) — синк и импорт используют один и тот же spreadsheet.

### Импорт текущих данных

Через сайт, не CLI: на админ-странице поле «Ссылка на Google-таблицу» + кнопка «Импортировать».

- Backend: `POST /api/admin/orders/import { spreadsheet_url }` — парсит ID из ссылки, читает вкладки
  `Игры`, `Фильмы`, `VIP\Unvip` через Sheets API (`values.batchGet`; service account имеет и read-доступ,
  раз уж выдаём write). Парсинг строк → вставка в БД (пропуск пустых строк; дедупликация по
  `(customer_name, title)` если импорт повторный). Ответ — отчёт: сколько импортировано/пропущено по каждой вкладке.
- Ссылка сохраняется в конфиг (`ConfigRepository`/runtime config), дальше используется синком.
- Статусы из цветов заливки API вернёт только если запрашивать `sheets.get` с форматированием —
  не делаем: статусы проставляются текстовой колонкой при переделке таблицы, до импорта.

## Frontend

Публичные страницы в `routes/(site)/`:

- `/games` — таблица заказов игр: поиск по названию/заказчику, фильтры (статус, стрим/прохождение,
  источник), сортировка. Объём ~230 записей — грузим всё одним запросом, фильтруем на клиенте.
- `/movies` — аналогично (статус, тип: фильм/сериал/аниме/ютуб).

Админка `routes/(panels)/admin/panel/orders/+page.svelte` (guard наследуется от layout):

- вкладки «Игры» / «Фильмы» / «VIP»;
- таблицы с inline-редактированием/формами на `@sapa-tv-ru/ui-kit` (по образцу `AdminsCard.svelte`);
- блок «Google Sheets»: поле ссылки на таблицу, кнопки «Импортировать» и «Синхронизировать сейчас»,
  индикатор последнего синка;
- VIP-вкладка: блок «Активные VIP» (с подсветкой истекающих ≤ 3 дней), «Ожидают возврата VIP» (unvip-записи),
  история. Напоминания чисто визуальные, без авто-действий.

Навигация: пункт в `SidebarMenu.svelte` (админка) и в `SiteNav` (публичные страницы).

После бэкенда: `just gen-client` — типы и методы появятся в `@sapa-tv-ru/api-client`.

## Автоматизация (позже, отдельный план)

- Новый action движка правил: `add_order_entry { list: games|movies, customer, source: 'roulette' }` —
  по событию `PlatformEvent::reward_redemption` создаёт запись с `title = NULL` (заказчик выберет позже,
  админ заполнит). VIP: `add_vip_record { kind: vip|unvip }`.
- Завязано на backlog-пункты «Админка управления наградами» и «Новые матчеры/экшены».

## Порядок работ

1. Миграции 0011–0013 + доменные модули + репозитории + parity-тесты.
2. Публичные GET-эндпоинты + тесты.
3. Админский CRUD + тесты.
4. `just gen-client`, публичные страницы `/games`, `/movies`.
5. Админ-страница `admin/panel/orders` (игры, фильмы, VIP).
6. Sheets-клиент (service account) + импорт на сайте (ссылка + кнопка, отчёт).
7. Sheets-синк сайт → таблица (debounce + daily + manual).

Проверки: `cargo nextest run --package backend`, `cargo clippy --all-targets`, `cargo fmt --check`,
фронт — vitest/playwright по необходимости.
