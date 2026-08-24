# Сторы логики докпанели и виджета (.svelte.ts) + юнит-тесты

Статус: **выполнен** (2026-08-24). Пункт бэклога #3: «Логика докпанели и виджета в .svelte.ts сторы + юнит-тесты».

Дата: 2026-08-24

## Зачем

После разбиения #3 страницы докпанели (~330 строк) и рулетки (~220 строк) стали
композицией «разметка + стили + логика». Логика (ws-reconnect, обработка сообщений,
машина фаз рулетки с таймерами, действия над очередью) не покрыта тестами вообще —
её нельзя вызвать без монтирования компонента. Вынос в `.svelte.ts` сторы даёт:

- юнит-тесты через обычный vitest (server-проект, node) без браузера;
- страницу-композицию: инстанс стора + разметка;
- единый паттерн на будущее (конвенция `*.svelte.ts` уже есть: `panel-state.svelte.ts`).

## Контекст (текущее состояние)

- Vitest: два проекта — `client` (playwright-browser, `*.svelte.test.ts`) и `server`
  (node, `*.test.ts`). SvelteKit-плагин компилирует руны в `.svelte.ts` в обоих.
- Конвенция моков: `vi.mock("#lib/api", () => ({...}))` + neverthrow `okAsync/errAsync`
  (см. `session.test.ts`).
- Докпанель: состояние очереди, ws с экспоненциальным backoff и обработкой
  auth/presence/spin-сообщений, поллинг каждые 10с, события-лог (50 последних).
- Рулетка: машина фаз idle→spinning→completed/error→idle, авто-complete через 10с,
  возврат в idle через 4с, ws c retry, бейджи ключа/связи.

## План

### Шаг 1. DockStore (`src/lib/dock/dock-store.svelte.ts`)

```ts
export class DockStore {
	entries = $state<QueueEntry[]>([]);
	connState = $state<"connected" | "disconnected">("disconnected");
	// ... nextUser, dequeueLabel, keyState, widgetOnline, events
	active = $derived(/* Pending|Spinning|Error */);
	done = $derived(/* Completed|Cancelled */);

	constructor(private accessKey: string, private opts?: {
		pollIntervalMs?: number;   // дефолт 10_000; в тестах — маленький
		websocket?: typeof WebSocket; // инъекция фейка в тестах
	}) {}

	start() { /* loadAll + setInterval + connectWs */ }
	stop() { /* clearInterval, ws.close() */ }

	async loadAll(): Promise<void>
	async dequeueNext(): Promise<void>
	async complete(id: number): Promise<void>
	async cancel(id: number): Promise<void>
	async enqueue(name: string): Promise<boolean>  // true = добавлен (страница чистит поле)
}
```

- Весь ws-цикл (backoff, auth handshake, presence, spin-сообщения) — внутри стора.
- `addEvent` — приватный, лимит 50 сохраняется.
- Страница `dock/+page.svelte`: `const dock = new DockStore(key)` + `onMount(dock.start)`
  + разметка читает `dock.*`. Ожидаемо ~150 строк.

### Шаг 2. Тесты DockStore (`dock-store.svelte.test.ts`, client-проект)

- Моки: `vi.mock("#lib/api")` (WS_URL + wapi), класс `FakeWebSocket` (ловит `onmessage`,
  позволяет дёргать `onopen/onclose/message`), `vi.useFakeTimers`.
- Кейсы: loadAll заполняет derived; 401 → keyState bad/missing; dequeue/complete/cancel
  пишут событие и перезагружают; enqueue гвардит пустое имя; ws open→auth отправлен;
  presence обновляет widgetOnline; spin_started → событие + перезагрузка; close →
  reconnect с backoff (fake timers), auth_err останавливает ретраи.

### Шаг 3. RouletteStore (`src/lib/widgets/roulette-store.svelte.ts`)

```ts
export class RouletteStore {
	phase = $state<Phase>("idle");
	stateLabel/idleText/conn/badge/spin = $state(...);

	constructor(private accessKey: string, private opts?: { websocket?, onCompleteAuto?: ... }) {}

	start(); stop();
	handleMessage(msg: WsMessage): void     // switch по типу сообщения
	private setSpinning/setCompleted/setSpinError/setIdle/failAuth
}
```

- Авто-complete таймер остаётся в сторе, вызов `wapi.complete` мокается в тестах.
- Страница рулетки: ws-сокет остаётся в странице ИЛИ переезжает в стор — решить по
  месту; предпочтительно в стор (тогда страница ~80 строк: доступ к ключу, старт, рендер).

### Шаг 4. Тесты RouletteStore

- Фазовая машина: spinning → (10с, fake timers) авто-complete → completed → (4с) idle;
  spin_error чужого entry_id не сбрасывает текущий спин; failAuth → phase=denied +
  бейджи; handleMessage прокидывает все типы сообщений.

### Шаг 5. Перевод страниц и проверка

```sh
pnpm --filter frontend check && pnpm --filter frontend test:unit -- --run
pnpm --filter frontend lint
```

Ручной smoke: докпанель (dequeue/complete/cancel/enqueue, бейджи, лог),
виджет (спин по ws-событию из админки).

## Осознанно вне скоупа

- Вынос логики остальных страниц (site/admin cards) — у них логика тоньше, нужда меньше.
- Общий базовый класс для ws-reconnect двух сторов: сначала продублируем честно,
  обобщим при третьем использовании.

## Оценка

2–3 ч: DockStore ~1 ч, RouletteStore ~0.5–1 ч, тесты ~0.5–1 ч (кейсы описаны,
механика моков уже отработана в session.test.ts).

## Итог (отличия от плана)

Выполнено полностью; vitest 53 passed (28→53), svelte-check/eslint/lint чисто.
Итоговый размер: dock/+page 332→144 строк, roulette/+page 219→24. Отклонения:

- **DockStore**: добавлены `dequeueBusy`/`enqueueBusy` — в плане не фигурировали,
  но кнопки Dequeue/Добавить держат disabled-состояния; `enqueue(name)` возвращает
  `boolean`, чтобы страница чистила поле только при успехе (прежнее поведение).
- **ws рулетки переехал в стор целиком** (план предлагал «решить по месту») —
  страница осталась чистым рендером в 24 строки.
- Тесты гоняются в server-проекте (node), не в browser-mode: рунам `.svelte.ts`
  браузер не нужен, быстрее и без playwright.
- Грабли тестирования сторов (задокументированы паттернами в тестах):
  незавершённый `loadAll()` из `start()` резолвится после `mockReset` следующего
  теста → хелпер `startStore()` флашит микротаски через fake timers; дефолтные
  реализации моков ставятся до старта.
- Линты внесли правки: `_`-префиксы для неиспользуемых аргументов даже в типах,
  константа вместо литерала 401, `SvelteDate` вместо `new Date()` внутри реактивного
  класса (`svelte/prefer-svelte-reactivity`).
- Ручной smoke докпанели/виджета остаётся за эксплуатацией (логика покрыта
  юнит-тестами на все описанные в плане кейсы).
