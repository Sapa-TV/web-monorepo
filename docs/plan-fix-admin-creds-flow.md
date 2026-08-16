# Fix: получение Twitch credentials на /admin/panel

Статус: **запланировано / не начато**. Запись для реализации.

Дата: 2026-08-16

## Контекст (текущее состояние)

На странице `/admin/panel` блок «Twitch credentials» не работает: после клика «Авторизовать»
открывается пустая вкладка + отдельная вкладка с Twitch, после подтверждения доступа статус
остаётся «не авторизовано».

Причины (в порядке влияния):

1. **Один `redirect_uri` на два OAuth-потока** — `apps/backend/src/admin/auth.rs` строит URL как
   для логина (`start_login`, `auth.rs:69-71`), так и для credentials (`start`, `auth.rs:65-67`)
   из одного `twitch.redirect_uri` (`auth.rs:75` и `auth.rs:128`). Логину нужно возвращаться на SPA
   `/admin/login`, а credentials-потоку — на бекенд `/api/admin/twitch/auth/callback`, где
   refresh token сохраняется (`AdminAuthService::complete`, `auth.rs:92-106`). При текущем
   `TWITCH__REDIRECT_URI=http://localhost:5173/admin/login` попап после авторизации приходит на
   страницу логина, которая разменивает code по логин-пути (`session.ts:completeLogin`) — refresh
   token **не сохраняется**, поллинг `/api/admin/ingress/credentials` вечно отдаёт `configured:false`.
2. **Баг попапа** — `window.open("", "_blank", "noopener")` (`panel/+page.svelte:149`) при
   `noopener` возвращает `null` (по спеке), `win.location.assign` не выполняется, остаётся пустая
   вкладка, а `window.open(auth_url, ...)` на `:154` открывает Twitch вторым окном.
3. **Затирание сессии** — попап попадает на `/admin/login`, чей `onMount` прогоняет полный логин
   бот-аккаунта и перезаписывает `sapa_session` root-админа → дальше панель получает 403.

Затронутые файлы:

- `apps/backend/src/admin/auth.rs` — общий redirect + размен токена.
- `apps/backend/src/config/twitch.rs` — единственная `redirect_uri`.
- `apps/backend/src/config/store.rs`, `apps/backend/src/ingress/twitch_auth.rs` — литералы `TwitchConfig` в тестах.
- `apps/frontend/src/routes/admin/panel/+page.svelte` — попап и поллинг.
- `apps/frontend/src/lib/admin/session.ts` — общая логика логина.

## Решение (обзор)

Вводим **отдельный** `TWITCH__CREDENTIALS_REDIRECT_URI` для credentials-потока: Twitch
возвращает попап на новую SPA-страницу `/admin/creds/callback`, которая выдёргивает `code`/`state`,
зовёт бекенд `/api/admin/twitch/auth/callback` (persist refresh token) и сама закрывает окно.
Панель как и раньше поллит `GET /api/admin/ingress/credentials`.

## Изменения в Twitch Dev Console и .env (сделать в первую очередь)

Twitch Dev Console — приложение (app), вкладка «Redirect URIs»:

- Оставить `http://localhost:5173/admin/login` (логин-поток).
- **Добавить** `http://localhost:5173/admin/creds/callback` (credentials-поток).
- В проде это `https://<домен>/admin/creds/callback` и `https://<домен>/admin/login`.

`.env` (и шаблон `.env.example`):

- Добавить переменную `TWITCH__CREDENTIALS_REDIRECT_URI`.
- Dev-значение по умолчанию: `TWITCH__CREDENTIALS_REDIRECT_URI=http://localhost:5173/admin/creds/callback`.
- Переменная **обязательна** (fail-fast): если не задана или пустая, конфиг падает при старте бэкенда —
  никаких «бэкенд без credentials» в рантайме. Это стандартный для конфига подход в этом репозитории
  (см. другие обязательные поля `TwitchConfig`).

## Шаги

### Шаг 1. Backend: поле `credentials_redirect_uri` в конфиге

Файлы: `apps/backend/src/config/twitch.rs`, `apps/backend/src/config/store.rs` (литерал в тесте), `apps/backend/src/ingress/twitch_auth.rs` (литерал в тесте).

- В `TwitchConfig` добавить `pub credentials_redirect_uri: String` (обязательное поле).
- В `build()` принять параметр и включить в список обязательных полей (fail-fast на пустое значение).
- В `Deserialize` поле обязательное (без `#[serde(default)]`), пробросить в `build`.
- Обновить `twitch_json()` (добавить поле) и тесты: отсутствие/пустое значение → ошибка
  `MissingField { field: "credentials_redirect_uri" }`.
- Добавить `credentials_redirect_uri: String` в структуры-литералы в тестах `store.rs`,
  `ingress/twitch_auth.rs`, `admin/auth.rs` и JSON в тестах `static_config.rs`.

Done, когда: `cargo test` в `apps/backend` зелёный.

### Шаг 2. Backend: разнесение redirect по потокам

Файл: `apps/backend/src/admin/auth.rs`.

- `start_with_scopes(scopes, redirect_uri: &str)` — параметризовать по redirect.
- `start()` → credentials redirect (`NotConfigured`, если `None`).
- `start_login()` → прежний `redirect_uri`.
- `exchange(code, state, redirect_uri)` — параметризовать.
- `complete()` → credentials redirect; `complete_login()` → прежний redirect.
- В `test_config()` добавить `credentials_redirect_uri: Some(...)`.
- Новые юнит-тесты:
  - `start()` кладёт `redirect_uri=<credentials>` в auth URL; `start_login()` — `<login>`.
  - `start()` → `NotConfigured`, если `credentials_redirect_uri = None`.

Done, когда: `cargo test` и `cargo clippy` в `apps/backend` без предупреждений.

### Шаг 3. Backend (опц.): лог

Файл: `apps/backend/src/main.rs` (`main.rs:56-60`).

- Добавить `credentials_redirect_uri` в строку «twitch config ready».

### Шаг 4. Frontend: lib-хелпер для credentials-callback

Файлы: новый `apps/frontend/src/lib/admin/creds.ts`, новый `apps/frontend/src/lib/admin/creds.test.ts`.

- `completeCredsAuth(code, state)` → `api.twitchAuthCallback(code, state)` (генерённый клиент уже есть
  в `packages/api-client/generated/Api.ts:137-147`, `credentials:"include"` в `base-client.ts:88`).
- Упаковать ошибки в понятные сообщения (401/403 → «сессия истекла, перелогинься»; 400 → «не настроен»).
- Юнит-тесты в стиле `session.test.ts`.

Done, когда: `pnpm test` зелёный.

### Шаг 5. Frontend: страница-колбэк `/admin/creds/callback`

Новый файл: `apps/frontend/src/routes/admin/creds/callback/+page.svelte`.

- `onMount`: читает `code`/`state` и `error`/`error_description` (отказ юзера тоже приходит сюда).
- `error` → показать текст; `code+state` → `completeCredsAuth(...)`.
- Успех → «Twitch credentials авторизованы», `if (window.opener) window.close()`.
- Ошибка → сообщение, окно не закрывать (чтобы было что прочитать).
- Стилизуется как login-страница (переиспользовать токены и `.btn--twitch`).

Done, когда: страница открывается по URL `.../admin/creds/callback?code=x&state=y` и закрывает попап.

### Шаг 6. Frontend: починить попап и поллинг в панели

Файл: `apps/frontend/src/routes/admin/panel/+page.svelte` (`authorizeTwitch` :146-162, поллинг :131-144).

- `authorizeTwitch()`:
  - попап без `noopener`: `window.open("", "sapa_twitch_auth", "popup,width=560,height=720")` (синхронно в обработчике клика, чтобы не блокировался);
  - `startTwitchAuth()` упал → `win?.close()` + показать ошибку;
  - `win` не null → `win.location.assign(auth_url)`, иначе fallback `window.open(auth_url, "sapa_twitch_auth")`;
  - сообщение «Авторизуйся во всплывающем окне — статус обновится сам».
- Поллинг:
  - лимит по времени (например ~5 мин) — останавливаться;
  - ошибки не глотать: 401/403 → стоп + «сессия истекла»; остальные — продолжать с попытками, после лимита — показать ошибку.

Done, когда: `pnpm test`, typecheck и lint без предупреждений.

### Шаг 7. Шаблон конфига

Файл: `.env.example` (и актуальный `.env` у разработчика).

- Добавить `TWITCH__CREDENTIALS_REDIRECT_URI` + комментарий: «зарегистрировать в Twitch Dev Console».
  (Если guard на чтение `.env*` не даст отредактировать — прислать пользователю строку для ручной вставки.)

### Шаг 8. Проверка вручную (e2e)

1. `apps/backend`: `cargo test`, `cargo clippy`.
2. `apps/frontend`: `pnpm test` + typecheck/lint.
3. Поднять backend + frontend, `.env` с новым `TWITCH__CREDENTIALS_REDIRECT_URI`.
4. `/admin/login` → `/admin/panel`.
5. «Авторизовать» → появляется одно окно с Twitch (пустой вкладки нет) → подтвердить.
6. Попап сам закрывается, на панели статус → «авторизовано», панель остаётся root-доступом.
7. «Отозвать» → статус «не авторизовано».
8. Регрессия логина: выйти → `/admin/login` → войти через Twitch (работает как раньше).
