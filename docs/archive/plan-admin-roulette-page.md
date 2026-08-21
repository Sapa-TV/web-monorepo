# Админка: страница «Рулетка» — редактирование roulette slots

Статус: **реализовано** (шаги 1–8).

Дата: 2026-08-21

## Контекст (текущее состояние)

- Слоты и редкости живут в бекенде: `roulette_slots` (id, name, rarity_id, weight, action) + `rarities` (id, name, display_name, image, color; 5 предзаполненных: common→mythical).
- CRUD слотов/редкостей сейчас торчит в `widget_api` под `/wapi` с защитой Bearer widget access key (`apps/backend/src/widget_api/roulette_slots.rs`, `rarities.rs`).
- Во фронте `/wapi/slots` и `/wapi/rarities` никто не вызывает (виджет получает данные через WS-события, dock их не трогает).
- Сгенерированный клиент имеет класс `Wapi` с методами slots/rarities, но он не экспортируется из `@sapa-tv-ru/api-client`.
- Меню админки уже рассчитано на пункт «Рулетка» (`docs/plan-admin-panel-split.md`).

## Задача

Добавить в админку страницу «Рулетка» с редактированием слотов рулетки. Решения (согласованы):

- CRUD переносится в `/api/admin/roulette/*` под админ-сессию (require_admin) — страница доступна всем админам.
- `/wapi/slots` и `/wapi/rarities` остаются **только для чтения** (GET); create/update/delete из wapi убираются.
- Редактируются и слоты, и редкости; редкости — редкий функционал, прячутся под accordion/spoiler.
- В таблице слотов — колонка «Шанс %» (weight / сумма весов).

## План реализации

### 1. Бекенд: новый `apps/backend/src/api/admin/roulette.rs`

По образцу `api/admin/actions.rs`:

- Перенести DTO и хендлеры из `widget_api/roulette_slots.rs` и `widget_api/rarities.rs`.
- Слоты: `GET/POST /admin/roulette/slots`, `PUT/DELETE /admin/roulette/slots/{id}`.
- Редкости: `GET/POST /admin/roulette/rarities`, `PUT/DELETE /admin/roulette/rarities/{id}`.
- Всё в `session_router()` → require_admin (root_router не нужен).
- Свой `AdminRouletteApiDoc` (utoipa OpenApi).

### 2. Бекенд: роутинг + OpenAPI

- `api/admin.rs`: `session_router().merge(roulette::session_router())`.
- `lib.rs` (`MergeSubdocs`): убрать `SlotsApiDoc`/`RaritiesApiDoc` из widget-ветки, добавить `AdminRouletteApiDoc` в main-ветку. Теги `slots`/`rarities` сохраняются.

### 3. Бекенд: wapi → read-only

- `widget_api/roulette_slots.rs`: оставить только `list_slots` (GET /wapi/slots); удалить create/update/delete хендлеры, request-типы и роуты.
- `widget_api/rarities.rs`: оставить только `list_rarities`; аналогично удалить остальное.
- Тест: новые admin-эндпоинты требуют сессию (по образцу `actions_require_admin_session_cookie`).
- Проверка: `cargo nextest run --package backend`.

### 4. Регенерация клиента

`just gen-client` — в `Api.ts` появятся типизированные методы CRUD для `/api/admin/roulette/*`, в `Wapi.ts` останутся только list-методы.

### 5. Фронтенд: меню

`SidebarMenu.svelte` — пункт **Рулетка** (`${panelBase}/roulette`), иконка lucide `loader-pinwheel`.

### 6. Фронтенд: страница слотов

- `routes/(panels)/admin/panel/roulette/+page.svelte` — title «Sapa TV | Рулетка», рендер `<RouletteSlotsCard />`. Доступ всем админам (guard уже в layout).
- `components/admin/roulette/RouletteSlotsCard.svelte` — самодостаточная карточка:
  - таблица: имя, редкость (цветная точка + display_name), вес, шанс % (weight/Σ весов; при Σ=0 — «—»), действие, edit/delete;
  - создание/редактирование инлайн-формой по образцу `ActionsSection`.
- `components/admin/roulette/RouletteSlotForm.svelte` — поля: name (required), rarity (Select: display_name + цветной dot), weight (число ≥ 0), action (строка).

### 7. Фронтенд: редкости под спойлером

`components/admin/roulette/RaritiesCard.svelte` — нативный `<details>` в стилях проекта (ui-kit не расширяем); внутри таблица (display_name, name, color-свотч, image) + форма create/edit.

### 8. Проверка

- `cargo nextest run --package backend`.
- `pnpm --filter frontend check` + `pnpm --filter frontend lint`.
- Ручная проверка: пункт меню «Рулетка», CRUD слотов, спойлер редкостей, колонка шанса.

## Примечания

- `action` у слота — свободная строка, бекенд её нигде не исполняет; в UI это просто текстовое поле «Действие».
- Компоненты кладём в `src/lib/components/admin/roulette/` (конвенция из плана разделения админки).
