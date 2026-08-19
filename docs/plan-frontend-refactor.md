# План: рефакторинг фронтенда (ui-kit, layout-ы, lint)

Статус: принят (2026-08-19). Утверждённые решения:

- ui-kit — отдельный workspace-пакет `packages/ui-kit` (`@sapa-tv-ru/ui-kit`)
- миграция на ui-kit — полностью, все существующие страницы
- lint-правило на цвета — «мягкое»: запрещены hex/rgb/hsl/oklch/named-colors, разрешены `transparent` и `currentcolor`

Цели:

1. Выделить переиспользуемые минимальные элементы в ui-kit (кнопки, инпуты, селекты и т.д.) со
   стабильным контрактом — реализация меняется свободно, контракт (props) остаётся.
2. Разделить по layout-ам: основной сайт, админ-панель + док-панель, виджеты.
3. Lint-правило: в `.svelte` запрещён атрибут `style` — только `class`.
4. Lint-правило: в `<style>`-блоках `.svelte` запрещены цвета напрямую — только CSS-переменные.

---

## Шаг 0. Перемещение маршрутов

`src/routes/admin/**` -> `src/routes/(panels)/admin/**`

- В URL группа `(panels)` не влияет: `/admin/*` остаётся как есть.
- Относительные импорты (`./ActionsSection.svelte`, `./RulesSection.svelte`) переезжают вместе с файлами.
- Гварды (redirect на `/admin/login`, `/admin/panel`) не меняются.

## Шаг 1. Layout-и

Целевая схема маршрутов:

```
src/routes/
  +layout.svelte              # только theme.css (уже есть)
  (site)/+layout.svelte       # сайт: SiteNav + fonts + "paper" (уже есть)
  (site)/+page | links | stream
  (panels)/+layout.svelte     # НОВЫЙ: базовые стили панелей
  (panels)/admin/login | creds/callback | panel
  (panels)/dock/
  (widgets)/+layout.svelte    # НОВЫЙ: прозрачный фон, центрирование (OBS-виджет)
  (widgets)/roulette/
```

- Новые файлы стилей:
  - `src/styles/panel.css` — html/body панелей (фон, шрифт, размер), таблицы (`table/th/td`,
    `.table-wrap`, `.actions-cell`), `.mono`, `.visually-hidden`, `.loading`.
  - `src/styles/widget.css` — `html/body` виджета: прозрачный фон, центрирование, шрифт.
- Из admin/login, creds/callback, admin/panel, dock, roulette удалить дубли `:global(html)/:global(body)`.
- `stream` (внутри `(site)`): убрать наслоение на `:global(body)`, перевести на локальный wrapper-класс
  (`height: 100vh; overflow: hidden`), чтобы не ломать layout сайта.
- Табличные `th/td` живут в `panel.css`, а не в компоненте `TableWrap`: в Svelte скопленные стили не
  применяются к слот-контенту родителя.

## Шаг 2. `packages/ui-kit`

Новый workspace-пакет `@sapa-tv-ru/ui-kit`.

- Без build-шага: `exports` с `svelte`-условием на исходные `.svelte` (собирает Vite фронтенда).
- Иконки внутрь не импортируются — прокидываются слотом (нет связки с unplugin-icons).
- Тема — через глобальные CSS-переменные из `theme.css` (импортируется корневым layout-ом).
- deps: `svelte` (peer). Свои `check`/`lint`-скрипты.
- Фронтенд: `"@sapa-tv-ru/ui-kit": "workspace:*"`.

Список компонентов (контракт = props; реализации меняем, контракт стабилен):

| Компонент   | Покрывает сейчас                                                            | Варианты / props                                                                                               |
| ----------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `Button`    | `.btn`, `.btn--primary/sm/danger/icon/complete/cancel/twitch`, `.tape`      | `variant` (default/primary/danger/brand), `size` (sm/md), pass-through (`type`, `disabled`, `onclick`, aria-*) |
| `Input`     | `.inline-form input`, `.field input`                                        | `value` (bindable), placeholder/type, pass-through                                                             |
| `Select`    | `.field select` (kind, trigger, matcher, actionId, rewardId)                | `value` (bindable), children = options                                                                         |
| `Checkbox`  | `.check` (enabled/Включено)                                                 | `checked` (bindable), children = label                                                                         |
| `Field`     | `.field` (label + control)                                                  | `label`, children = control                                                                                    |
| `Badge`     | `status-badge.*`, `key-badge.*`, `status-pill.*`, `badge--root`, `conn-dot` | `tone`, `dot` (bool), children                                                                                 |
| `Alert`     | `.alert--error/ok`                                                          | `tone` (error/success), default `role="alert"`                                                                 |
| `Card`      | `.card` (админ-секции)                                                      | children                                                                                                       |
| `Section`   | `.section` + `.section-title` (админка, dock)                               | `title`, children                                                                                              |
| `TableWrap` | `.table-wrap` (обёртка таблицы; стили th/td в panel.css)                    | children = `<table>...`                                                                                        |
| `Code`      | `.mono`, `.key-value`                                                       | `block` (bool), children                                                                                       |
| `AuthCard`  | login + creds/callback карточка                                             | `title`, `subtitle`, children, `error`                                                                         |

Миграция (всё сразу):

- admin/panel, ActionsSection, RulesSection, dock, login, creds/callback -> ui-kit.
- Лендинг: `.tape` x4 -> `Button variant="brand"` (читает `--brand`/`--brand-ink`, задаются на родителе).
- Дублированные стили из файлов удаляются.

## Шаг 3. Lint-правила

Проектируется и пишется на этом шаге (не раньше). ESLint flat config.

1. **`no-inline-styles`** — включаем готовое правило `svelte/no-inline-styles` (error):
   запрещает `style="..."` атрибуты и `style:`-директивы в `.svelte`, только `class`.

2. **`no-color-literals`** — кастомное правило (error) для `**/*.svelte`:
   - источник AST: `svelte-eslint-parser` + `parserServices.getStyleContext()` (postcss AST `<style>`-блока);
   - идём по `decl`-узлам, ищем цветовые литералы в значении, включая аргументы `color-mix()`,
     градиенты, `box-shadow`, и `var(--x, <fallback>)`;
   - мягкая политика: запрещены hex, rgb/rgba/hsl/hsla/oklch/oklab/lab/lch/color()/hwb, named-colors;
     разрешены `transparent` и `currentcolor`;
   - применяется к любому `.svelte` (панель + ui-kit).

Размещение правил: единый источник — `packages/ui-kit/lint/rules/`.
Подключается и в `apps/frontend/eslint.config.js`, и в `eslint.config.js` ui-kit
(чтобы правило покрывало оба пакета). ui-kit получает свой `lint`-скрипт
(в turbo `lint` уже цепляет `^lint`).

### Чистка текущих цветов (в этом же шаге)

Хардкод-цвета в `.svelte` (~50 мест) переносим в CSS-переменные `theme.css`:

- `roulette/+page.svelte` — `oklch(...)`, `#f2e9dc`, `#f6efe5`, `#d9cfbf`, `rgba(...)` (шрифт/фон/бордер/тени) -> новые виджет-токены (`--widget-bg`, `--widget-ink`, `--widget-accent`, ...);
- `stream/+page.svelte` — `background-color: #000`, fallback-и `var(--x, #...)` -> токены или просто переменные без fallback;
- fallback-и `var(--error, #e64040)`, `var(--primary, #ffb01e)`, `var(--twitch-brand, #9146ff)` (SiteNav, DonateToggle, login) — убрать fallback, оставить `var(--x)` (theme.css всегда загружен);
- `admin/*` и `dock` — fallback-и и редкие литералы;
- добавить недостающие токены в `theme.css` (виджетные, тени, оверлей-скан).

## Шаг 4. Проверка

- `pnpm --filter frontend check`
- `pnpm --filter frontend lint`
- `pnpm --filter frontend test:unit` (+ e2e при необходимости)
- `pnpm --filter frontend build`
- `pnpm --filter @sapa-tv-ru/ui-kit lint`
- Смоук: лендинг, /links, /stream, /dock, /roulette, /admin/login, /admin/panel, creds/callback.
