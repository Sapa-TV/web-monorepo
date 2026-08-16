# Задачи

- [ ] Переделать на axum-extra (cookie) `let mut cookie = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");`
- [ ] Перенести все const в config, для части сделать default значения
- [ ] Разделить config - сейчас куча всего лежит в common
- [x] перейти с .env ACCESS_KEY на генерируемый при старте и с root ендпойнотом для замены ключа
- [x] посмотреть конфиги - зачем нужен refresh token в .env config? он будет храниться в db repo
- [x] посмотреть current_refresh_token - почему ошибка Auth вместо Fail-Fast при загрузке конфигурации во время инициализации
- [x] apps/backend/src/ingress/twitch_auth.rs - сейчас сохраняется в файл, надо переделать на inmemory repo.
      c трейтом и инмемори реализацией, позже это будет храниться в sqlite db
- [x] Объединить .env / .env.example (сейчас два набора: корневой — дев (читает dotenvy при cargo run), deploy/.env + deploy/.env.example — прод/sops-раундтрип).
      Реализован вариант A: все env-файлы в корне — .env (прод, gitignored, источник для sops), .env.sops (шифрованный, tracked), .env.dev (дев, gitignored),
      .env.example (единый шаблон). justfile sops-раундтрип: .env -> .env.sops. scp в deploy-backend.yml и path_regex в .sops.yaml обновлены на корневые файлы.
      Удалены deploy/.env.example и deploy/.env. VPS (deploy-backend.sh, docker-compose.yml env_file: .env) не менялся. .env* добавлены в .dockerignore.
- [ ] Бекенд выдаёт query-параметры в openapi.json как `in: path` (у `GET /api/queue` status/limit/cursor, у `GET /api/users` platform/platform_user_id и т.п.).
      Из-за этого в сгенерированном клиенте они объявлены аргументами, но в запрос не попадают.
      Починить генерацию спеки на бекенде (стоит пометить: поправить utoipa/introspection, чтобы параметры были `in: query`), затем выполнить `just gen-client`.
      Пока обходимся передачей query вручную: `api.api.<method>(args, { query: { status: "Pending" } })`
- [ ] `packages/api-client/src/base-client.ts`: добавить `credentials?: RequestCredentials` в `ApiConfig` и прокидывать в fetch — фронту нужен `credentials: "include"` (куки сессии)
- [ ] `packages/api-client/src/base-client.ts`: добавить `headers?: Record<string, string>` (default headers) в `ApiConfig` — чтобы `Authorization: Bearer <pak>` задавался в `createApi` один раз, а не в каждом вызове
- [ ] `apps/frontend/src/lib/api.ts`: убрать дубли `QueueEntry`/`QueueStats`, re-export из `@sapa-tv-ru/api-client` (интерфейсы `QueueEntryResponse`/`QueueStats` уже в generated); `apiFetch`/`WS_URL` пока оставить
- [ ] `apps/frontend/src/routes/(panels)/dock/+page.svelte`: заменить `apiFetch` на методы `api.api.*` (`list`, `stats`, `dequeueNext`, `complete`, `cancel`, `enqueueAnonymous`); 401-логику `setKeyState` перевести на проверку `HttpError.status === 401` через `Result.match`
- [ ] `apps/frontend/src/routes/(widgets)/roulette/+page.svelte`: заменить `apiFetch` на `api.api.complete` (и остальные используемые методы) с обработкой `Result`
- [ ] после пунктов выше: удалить `apiFetch`/`API_BASE` из `apps/frontend/src/lib/api.ts` (останется `api`, `WS_URL`)
- [ ] Зашивать commit/GIT_SHA в финальную сборку фронта (env при vite build / build.json в статике) — чтобы знать, какой билд фронта на сервере
- [ ] Добавить юз-кейс/e2e тесты (сейчас межмодульные сценарии вроде «ротация PAK → старый ключ на widget-эндпоинте не работает» покрыты только unit-тестами)
- [ ] Переделать создание структур - все структуры должны создаваться в одном месте - через методы new\build\builder, для этого во все добавить не публичное zero-size поле ()
