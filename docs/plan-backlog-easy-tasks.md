# План: лёгкие задачи из backlog.md

Статус: **план утверждён**, не начат.

Дата: 2026-08-21

## Классификация беклога

### Лёгкие

| Задача                                                                            | Оценка  |
| --------------------------------------------------------------------------------- | ------- |
| `base-client.ts`: `credentials?` в `ApiConfig`                                    | ~15 мин |
| `base-client.ts`: `headers?` (default headers)                                    | ~15 мин |
| `api.ts`: дедуп `QueueEntry`/`QueueStats` → re-export                             | ~10 мин |
| Бекенд: root не может удалить сам себя (`removeAdmin`)                            | ~30 мин |
| GIT_SHA в сборку фронта                                                           | ~30 мин |
| axum-extra cookie вместо ручного `format!`                                        | ~1 ч    |
| Админка: подтверждение обновления access_key (диалог в ui-kit, без старого ключа) | ~30 мин |

### Средние

- Перенос `apiFetch` на `api.api.*` в dock (~583 стр.) и roulette (~428 стр.) — механика, но объёмно; после них удаляется `apiFetch`.
- Consts → config с дефолтами (~20 констант по бекенду).
- Разделение config на модули.
- Линтер на длину api-хендлеров.
- Админку разбить на страницы + навигация.
- Поиск по username при добавлении админа (нужен helix-lookup эндпоинт).

### Сложные

utoipa `in: query` баг генерации спеки · e2e-тесты · builder/new для всех структур · редизайн потока добавления twitch credentials · переработка обработки ошибок (throw только в низах + статус-коды) · унификация стилей · Ingress hot-reload credentials · CancellationToken в supervisor · рефакторинг `ActionExecutor` · WS-статусы активных панелей · streamerbot-подобный редактор правил · Twitch rewards API + автофулфилл · событийный поток наружу · матчеры/кулдауны · no-code плагины · sqlite-персистентность.

## Решения

- Подтверждение ротации access_key — **кастомный диалог в ui-kit**, старый ключ не показываем.
- Миграция dock/roulette на `api.api.*` включена в этот заход (без неё не удалить `apiFetch`).
- Throw-логику `creds.ts` не трогаем — это отдельная «сложная» задача про обработку ошибок.

## Этапы

### Этап A — api-client и типы

1. ✅ Готово (2026-08-21). `packages/api-client/src/base-client.ts`: добавить `credentials?: RequestCredentials` и `headers?: Record<string, string>` в `ApiConfig`; сохранить как поля `HttpClient`, мерджить с per-request headers (default первыми), `credentials ?? "include"` (сейчас `"include"` захардкожен в fetch-options). Также расширены `CreateApiOptions`/`createApi`.
2. `apps/frontend/src/lib/api.ts`: удалить локальные интерфейсы `QueueEntry`/`QueueStats`, re-export типов из `@sapa-tv-ru/api-client`; поправить импорты по фронту.

### Этап B — бекенд

3. `remove_admin`: отклонять удаление собственного аккаунта (сравнение id сессии и цели), 40x + тест.
4. Заменить ручные cookie-строки (`format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax")`) на `axum_extra::cookie::Cookie` builder в auth/session + прогнать тесты auth.

### Этап C — фронт-мелочи

5. GIT_SHA: в `vite.config` через `define` инжектить `git rev-parse --short HEAD`, показать версию в UI.
6. `ConfirmDialog.svelte` в `packages/ui-kit/src` + подключить в `AccessKeyCard.svelte` вместо `window.confirm`.

### Этап D — миграция на `api.api.*` и удаление `apiFetch`

7. `apps/frontend/src/routes/(panels)/dock/+page.svelte`: методы `list`, `stats`, `dequeueNext`, `complete`, `cancel`, `enqueueAnonymous` через `api.api.*`; query передавать вручную (`{ query: {...} }`) до починки utoipa-спеки; 401-логика `setKeyState` → проверка `HttpError.status === 401` через `Result.match`.
8. `apps/frontend/src/routes/(widgets)/roulette/+page.svelte`: то же для используемых методов (`complete`, чтение очереди).
9. `lib/admin/session.ts` + `lib/admin/creds.ts`: перевести на `api.api.*`, обновить моки в `session.test.ts`/`creds.test.ts`.
10. Удалить `apiFetch`/`API_BASE` из `lib/api.ts` (остаются `api`, `WS_URL`, `WAPI_BASE`).

Примечание: `apiFetch` используется не только в dock/roulette, но и в `lib/admin/session.ts`/`creds.ts` — поэтому шаг 9 обязателен для шага 10.

## Проверка

- Фронт: `pnpm vitest run` + сборка (`pnpm build`).
- Бекенд: `cargo nextest run --package backend`, `cargo clippy --all-targets`, `cargo fmt`.
