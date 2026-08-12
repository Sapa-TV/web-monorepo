# Задачи

- [ ] Переделать на axum-extra (cookie) `let mut cookie = format!("{name}={value}; Path=/; HttpOnly; SameSite=Lax");`
- [ ] Перенести все const в config, для части сделать default значения
- [ ] Разделить config - сейчас куча всего лежит в common
- [ ] перейти с .env ACCESS_KEY на генерируемый при старте и с root ендпойнотом для замены ключа
- [ ] посмотреть конфиги - зачем нужен refresh token в .env config? он будет храниться в db repo
- [ ] apps/backend/src/ingress/twitch_auth.rs - сейчас сохраняется в файл, надо переделать на inmemory repo.
      c трейтом и инмемори реализацией, позже это будет храниться в sqlite db
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
