# TS: ошибки со статус-кодами + запрет try/catch/throw

Статус: **план на ревью**, не начато. Закрывает пункты бэклога «обработка ошибок — try/catch/throw только в низкоуровневых» и «у ошибок должна быть статус кода».

Дата: 2026-08-23

## Контекст (текущее состояние)

- `packages/api-client` **уже возвращает neverthrow**: `ResultAsync<T, E | HttpError<E> | TimeoutError | NetworkError | ParseError>`;
  `HttpError` несёт `status` — транспортная задача «ошибки со статус-кодами» решена.
- Нарушения живут слоем выше:
  - `src/lib/admin/creds.ts`, `session.ts` — конвертируют `Result` в `throw new Error(<UI-текст по-русски>)`,
    вручную матчат `err.status === 401/403/400`;
  - Svelte-компоненты (Actions/Rules/AdminsCard/TwitchPlatformCard/RaritiesCard/RouletteSlotsCard/
    AccessKeyCard/WidgetLinksCard/SiteNav/+page/+layout) — паттерн `if (res.isErr()) throw res.error`
    - `try { … } catch { setError }`.
- Инфраструктура линтов готова: кастомный пакет `packages/custom-eslint-rules`
  (правила `no-color-literals`, `props-inline-type`; пример подключения по files-glob в
  `apps/frontend/eslint.config.js`). Пакет `api-client` отдельным eslint-конфигом не покрывается
  (там только codegen-скрипт) — значит «разрешено в api-client» выполняется автоматически.

## Решения

1. **Запрет через кастомное ESLint-правило** `@sapa-tv-ru/custom-eslint-rules/no-exceptions`:
   репортит `ThrowStatement`, `TryStatement`, `CatchClause` и `.catch(` на промисах
   (MemberExpression с именем `catch` — иначе лазейка через `load().catch(...)`).
   Работает и в `.svelte` script-блоках (механика как у `colorLiterals`).
2. **Разрешённая папка** — `apps/frontend/src/lib/internal/`: единственное место в frontend,
   где try/catch/throw разрешены (тонкие адаптеры над api-клиентом). Отключение правила точечным
   files-glob в eslint.config.js. Пакет api-client вне зоны действия конфига frontend — тоже разрешён.
3. **Ошибки с человеко-читаемым дискриминантом**: `ApiErrorKind` — const-object enum
   (паттерн как `GuardStatus`), см. «Таксономию». Никаких выдуманных псевдо-HTTP кодов для
   не-HTTP семей. Никаких человекочитаемых текстов внутри ошибки.
4. **UI-тексты живут в UI**: компоненты/страницы матчат `kind` и держат свои тексты
   (как сейчас делает creds.ts, но наверху).
5. **Вся остальная работа — neverthrow**: компоненты перестают эскалировать `throw res.error`,
   используют `mapErr`/`andThen`; там, где нужна плоская последовательность вызовов —
   `safeTry`/генераторы neverthrow (v8 поддерживает) вместо try/catch.
6. **Тесты**: vitest-тесты переписываются на ассерты по `result.isErr()` + `error.kind`;
   `expect(…).rejects.toThrow(/текст/)` исчезают вместе с throws. Правило действует и на test-файлы;
   playwright e2e (`tests/**`) — off (там асерты чужого кода).

## Таксономия ошибок

```ts
// src/lib/internal/api-error.ts
export const ApiErrorKind = {
	// HTTP-семья: нормализация статус-диапазонами
	BadRequest: "bad_request",
	Unauthorized: "unauthorized",
	Forbidden: "forbidden",
	NotFound: "not_found",
	Conflict: "conflict",
	RateLimited: "rate_limited",
	Server: "server",        // любые 5xx
	HttpOther: "http_other", // прочие статусы вне таблицы
	// не-HTTP семьи — свои имена, никаких выдуманных цифр
	Timeout: "timeout",
	Network: "network",
	Parse: "parse",
} as const;

export type ApiErrorKind = (typeof ApiErrorKind)[keyof typeof ApiErrorKind];

export class ApiError extends Error {
	constructor(
		readonly kind: ApiErrorKind,
		readonly status?: number, // исходный HTTP-статус; только у http-семьи
		readonly body?: unknown,
	) {
		super(`api error: ${kind}${status ? ` (${status})` : ""}`);
	}
}
```

- нормализация из клиентских ошибок живёт в internal: точные статусы 400/401/403/404/409/429 →
  одноимённые kind; 5xx → Server; прочие → HttpOther; `TimeoutError` → Timeout;
  `NetworkError` → Network; `ParseError` → Parse;
- сырой `status` сохраняется в ошибке (логи, редкие точные матчи), но потребители матчат `kind`;
- UI-тексты компонентов ключуются от `kind`.

Отброшенные варианты: плоские цифры 504/0/597 для не-HTTP семей (504 — семантика gateway,
0 — наследие XHR, 597 — магия без смысла); двухуровневый `kind: http|timeout|…` + status
(двойной матч в компонентах); класс на каждую семью (дублирует api-client).

## Шаги

1. **Правило линта**
   - `packages/custom-eslint-rules/rules/no-exceptions.js` + экспорт в index.js;
   - RuleTester-тесты правила (позитив/негатив, включая .svelte-имитацию и `.catch(`);
   - подключение в `apps/frontend/eslint.config.js`: error на `src/**`, off на `src/lib/internal/**`
     и `tests/**` (playwright);
   - прогон `pnpm --dir apps/frontend exec eslint .` — зафиксировать текущий список нарушений
     (ожидаемо ~13 файлов) как чек-лист шага 4.
2. **Internal зона**
   - `src/lib/internal/api-error.ts`: `ApiErrorKind`, `ApiError`, функция
     `normalizeApiError(err: unknown): ApiError`;
   - юнит-тесты нормализации (vitest, без throw — правило уже действует).
3. **Переписывание lib/admin**
   - `creds.ts` → `completeCredsAuth(): Promise<Result<TwitchAuthCallbackResponse, ApiError>>`
     (или `ResultAsync`); то же для `startLogin`/`completeLogin`/guard в `session.ts`;
   - `creds.test.ts`/`session.test.ts` — ассерты по `kind` (Unauthorized/BadRequest), никаких `/текст/`;
   - UI-тексты («Нет доступа: сессия истекла…», «попробуй ещё раз») переносятся в страницу логина,
     которая матчит `kind`.
4. **Компоненты** — по списку из прогона линта:
   - убрать `if (res.isErr()) throw res.error;` + try/catch + `.catch(...)`;
   - `setError(err: unknown)` остаётся точкой форматирования: `instanceof ApiError` → текст по
     kind, иначе строковое представление;
   - цепочки вызовов → `safeTry` либо последовательные `mapErr`.
5. **Финализация**
   - `pnpm --dir apps/frontend lint && pnpm --dir apps/frontend check && pnpm --dir apps/frontend test:unit`;
   - бэклог: отметить оба пункта выполненными.

## Definition of Done

- eslint-правило активно, `lint` зелёный при нуле нарушений вне internal;
- в `src` (вне internal) нет ни одного throw/try/catch/.catch( — проверяется правилом,
  не ревью-глазами;
- все сетевые ошибки несут `kind` из `ApiErrorKind`; сырой `status` доступен для http-семьи;
- creds/session тесты ассертят `kind`, а не тексты;
- `check` (svelte-check) и `test:unit` зелёные.

## Риски / открытые вопросы

- `safeTry`-генераторы могут быть незнакомы — если покажутся перебором, альтернатива: локальный
  хелпер `pipe(result, andThen, …)`; стиль выбрать на первом компоненте;
- правило не ловит `reject(new ...)` внутри executor'а промиса — приемлемо: reject вне internal
  бессмысленен без try-границы