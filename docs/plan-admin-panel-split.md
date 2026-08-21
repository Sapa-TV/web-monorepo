# Админ-панель: разделение на подстраницы с боковым меню

Статус: **план готов**, не начат.

Дата: 2026-08-21

## Контекст (текущее состояние)

`/admin/panel` — монолитная страница (`apps/frontend/src/routes/(panels)/admin/panel/+page.svelte`, ~505 строк):

- Guard `guardAdmin()` вызывается в `onMount` страницы.
- Шапка: заголовок, root-badge, кнопка «Выйти».
- Секции подряд: Access key (widget access key), Администраторы, Twitch credentials, `ActionsSection.svelte`, `RulesSection.svelte`.
- Одна страница грузит всё сразу: WAK + админы + creds + actions + rules.
- Widget access key в коде именуется с префиксом `Pak` (`loadPak`, `rotatePak`, `copyPak`) — исторически неверное имя.

На `/admin/panel` ссылаются `login` и `links` через `resolve("admin/panel")`.

## Задача

Разделить панель на отдельные страницы: слева вертикальное меню, справа контент выбранного раздела. Разделы:

- **Платформы** — Twitch-авторизация (дефолтная страница; позже добавятся другие платформы).
- **Виджеты** — access key и ссылки на док-панель/виджет.
- **Действия и триггеры** — actions + rules.
- **Админы** — root-only контент.
- Позже: **Рулетка** — редактирование roulette slots.

Решения (согласованы):

- Реальные подроуты SvelteKit (deep-linking, кнопка «назад»).
- `/admin/panel` без подраздела = страница «Платформы».
- Twitch credentials живут в «Платформах»; слово «ингейш» в UI убираем.
- Root-only разделы видны всем в меню, но не-root видит заглушку «нет доступа».
- Переименовать всё `Pak*` → `Wak*` (widget access key): `loadPak` → `loadWak`, `rotatePak` → `rotateWak`, `copyPak` → `copyWak`.
- Фронтенд по возможности разбивать на минимальные компоненты (одна ответственность на компонент): шапка, пункт меню, меню и т.д. — вместо крупных монолитных файлов. Новые страницы/секции собирать из маленьких компонентов.
- Компоненты живут в `src/lib/components/`, разложены по папкам по смыслу, а не рядом с роутами:
  - `components/admin/` — каркас админ-панели (`PanelHeader`, `SidebarMenu`, `SidebarItem`);
  - `components/admin/platforms/` — карточки платформ (`TwitchPlatformCard`, позже YouTube/Kick);
  - `components/admin/widgets/` — виджеты (`AccessKeyCard`, `WidgetLinksCard`);
  - `components/admin/actions/` — действия и триггеры (`ActionsSection`, `RulesSection`).

## Целевая структура

```
apps/frontend/src/routes/(panels)/admin/panel/
├── +layout.svelte          ← НОВЫЙ: guard + шапка + вертикальное меню + контент
├── +page.svelte            ← ПЕРЕПИСАТЬ: «Платформы» (дефолт) — Twitch-авторизация
├── widgets/+page.svelte    ← НОВЫЙ: access key, ссылки на док-панель/виджет
├── admins/+page.svelte     ← НОВЫЙ: админы (root-only контент)
├── actions/+page.svelte    ← НОВЫЙ: действия + триггеры
├── ActionsSection.svelte   ← без изменений
└── RulesSection.svelte     ← без изменений
```

Позже: `roulette/+page.svelte` — меню расширится одним пунктом.

## План реализации

### 1. Общее состояние панели — `src/lib/admin/panel-state.svelte.ts`

Runes-модуль с полями `loaded`, `isRoot`. Заполняется guard'ом в layout; дочерние страницы читают `isRoot` без повторных запросов к API.

### 2. `+layout.svelte` панели (разбит на минимальные компоненты)

- `guardAdmin()` в `onMount` (логика из текущей страницы): `NotLoggedIn` → редирект на логин, `NotAdmin` → на главную; иначе заполнить panel-state.
- Компоненты в `src/lib/components/admin/`:
  - `PanelHeader.svelte` — заголовок «Админ-панель», root-badge, кнопка «Выйти»;
  - `SidebarMenu.svelte` — список разделов, определение активного пункта по `page.url.pathname`;
  - `SidebarItem.svelte` — один пункт меню (иконка, подпись, href, active).
- Пункты меню: **Платформы**, **Виджеты**, **Действия и триггеры**, **Админы** с lucide-иконками (`~icons/lucide/*`); ссылки через `resolve()` по образцу остального кода.
- Grid: `grid-template-columns: 220px 1fr`; на узких экранах (~720px) меню складывается в горизонтальную полосу сверху (media query).
- Контент рендерится только после проверки доступа («Проверка доступа...» до этого).

### 3. `+page.svelte` → «Платформы» (дефолт)

- Карточка Twitch: статус-бейдж (авторизовано/не авторизовано), кнопка «Авторизовать» (попап + polling переносятся как есть), «Отозвать».
- Переименовать «ингейш»: хинт — «Учётка, от имени которой бекенд ходит в Twitch (стрим-статус, чтение чата)», подтверждение отзыва — «Отозвать Twitch credentials? Интеграция с Twitch перестанет работать.»
- Структура карточек позволит добавить YouTube/Kick позже.
- Сюда переезжают: `loadCreds`, `startCredsPoll`, `authorizeTwitch`, `revokeCreds`, очистка poll-таймера в `onDestroy`.

### 4. `widgets/+page.svelte`

- Access key: `Code` + копирование, ротация ключа.
- Ссылки: копирование ссылок на док-панель и виджет (`linkFor`/`copyLink`).
- Сюда переезжают (с переименованием Pak → Wak): `loadWak`, `rotateWak`, `copyWak`, `linkFor`, `copyLink`, состояние `copied`/таймер фидбека.

### 5. `admins/+page.svelte`

- Если не root — `Alert` «Раздел доступен только root-админам.» (пункт меню при этом виден всем).
- Иначе форма добавления + таблица (переезжают `loadAdmins`, `addAdmin`, `removeAdmin`).

### 6. `actions/+page.svelte`

Просто рендерит существующие `<ActionsSection />` и `<RulesSection />` — компоненты самодостаточны (сами грузят данные), не меняются.

### 7. Проверка

- `pnpm --filter frontend check` (svelte-check).
- `pnpm --filter frontend lint`.
- Юнит-тесты (`session.test.ts`, `creds.test.ts`) не затрагиваются.
- Ручная проверка: переходы по пунктам меню, прямой заход по URL на каждый раздел, редиректы guard'а, ссылки со страниц login/links (продолжают работать — `/admin/panel` остаётся дефолтом).

## Бонус эффекта

Точечная загрузка данных: каждая страница грузит только своё вместо текущего «всё сразу».
