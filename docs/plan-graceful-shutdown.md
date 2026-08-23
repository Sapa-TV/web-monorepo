# Graceful shutdown через CancellationToken (бэклог #4)

Статус: **план на согласование**. Пункт бэклога #4: «Supervisor - running platforms останавливаются
через handle.abort(), исследовать возможность их остановки через CancelationToken».

Дата: 2026-08-23

## Вердикт

**Делать стоит, но в скоупе «supervisor + фоновые таски», а не «весь мир».**
Аборт в supervisor — реальная проблема (обрыв WS без close-frame и убийство in-flight Helix-запросов
при ротации/отзыве креденшелов). Остальное — дешёвая гигиена. termicy не берём.

## Текущее состояние

- HTTP server — уже graceful: `with_graceful_shutdown(shutdown_signal())`
  (main.rs:137). ✓
- Supervisor → ingress — `AbortHandle::abort()` на revoke/replace кредов
  (supervisor.rs:75,93). Обрыв WS без close frame; in-flight
  `create_eventsub_subscription` / `send_chat_message` убиваются посреди запроса.
- Ingress run-loop — бесконечный reconnect с голым `sleep(delay)` (twitch.rs:169).
  Неканселируем изнутри; при shutdown задача умирает вместе с рантаймом.
- Фоновые таски (`queue_timeout_task`, `queue_purge_task`, `session_prune_task`) —
  `loop { sleep; ... }` без остановки (main.rs:181–208). По данным ок (операции
  идемпотентны), грязно по семантике.
- Rule pipeline (`engine.run`, `executor.run` над mpsc) — не останавливается
  (runtime.rs:27–28); in-flight экшены теряются при Ctrl+C (например,
  недоставленный chat-reply).

Важный факт о деплое: backend живёт в Linux-Docker (`deploy/docker-compose.yml`),
`docker stop` шлёт SIGTERM и через 10 сек — SIGKILL. Сейчас за 10 секунд успевает
только drain HTTP-коннектов; ingress и фоновые таски обрываются мгновенно.

Что НЕ является проблемой: потеря данных. Все записи идут через sqlite-транзакции,
EventSub-подписки Twitch прибирает сам после разрыва WS-сессии. Речь только про
чистоту остановки и корректность соединений.

## Нужно ли это?

Да, по убывающей ценности:

1. **Supervisor (ядро задачи)** — стоп/рестарт ингресса сейчас происходит в момент,
   когда пользователь меняет креденшелы в админке. Abort() может разорвать
   наполовину выполненный Helix-вызов. Кооперативная отмена с таймаутом и abort
   как последним рубежом — правильный паттерн.
2. **Фоновые таски + pipeline** — дешёво (по паре строк на таск), даёт «настоящий»
   graceful: сначала гасим ingress и пайплайн, потом drain HTTP.
3. **TwitchPlatformService** — отменяемый reconnect-sleep и отправка close-frame.
   Без этого пункта отмена supervisor'а всё равно упирается в голый `sleep(delay)`.

## termicy — исследование

Crate существует: v0.1.0, опубликован 2026-08-11 (12 дней назад), автор один
(pr0n1x), **17 скачиваний всего**, 145 строк кода, docs.rs не настроен.

Что даёт:

- `CancellationListener` — capability-трейт над `tokio_util::sync::CancellationToken`:
  наблюдать отмену и плодить child-токены можно, `.cancel()` — нельзя. Защита от
  случайной отмены чужого скоупа на уровне типов.
- `Cancellable::until_cancelled` — cancel-biased гонка future против отмены.
- `Termination` — Unix-слушатель SIGINT/SIGTERM/SIGQUIT/SIGHUP, драйвит root-токен.

Почему **не берём**:

- Зрелость: v0.1.0, нулевая адаптация (17 загрузок), один релиз. Supply-chain риск
  ради 145 LOC.
- `termination` модуль **Unix-only** — на dev-машине (Windows) бесполезен; у нас уже
  есть кроссплатформенный `shutdown_signal()` (ctrl_c + SIGTERM).
- Всё содержимое тривиально заменяется: `tokio-util::sync::CancellationToken`
  (+ ~20 строк локального конвенционного правила «cancel down, never up») +
  существующий обработчик сигналов.

**Что берём у termicy бесплатно — конвенцию** (описана в его lib.rs, годная):
один root-токен на процесс; отменяет его ровно один владелец (signal listener);
отмена каскадит вниз и никогда вверх; никаких голых `sleep()` в циклах — всегда
гонка с `cancelled()`; родитель ждёт дочерние таски перед выходом; сервер —
`with_graceful_shutdown(token.cancelled())`. Если termicy созреет (>0.1, тысячи
загрузок) — переезд будет механическим, API повторяет tokio-util.

## План

### Шаг 1. Зависимость и root-токен

- `tokio-util = { workspace = true, features = ["rt"] }` в workspace + backend
  (`sync` фича для CancellationToken входит в default; проверить минимальный набор).
- `main.rs`: root `CancellationToken`; сигнал → `token.cancel()`; axum получает
  `with_graceful_shutdown(token.clone().cancelled_owned())` вместо `shutdown_signal()`.
  Порядок остановки: cancel root → supervisor гасит ingress → pipeline/фоновые
  выходят → drain HTTP параллельно → main ждёт serve.

### Шаг 2. PlatformService и фабрика получают токен

- Фабрика в `IngressSupervisor::run`/`reconcile`: сигнатура замыкания получает
  `CancellationToken` (child от root). `main.rs build_ingress` прокидывает его
  в `TwitchPlatformService::run(sink, token)`.

### Шаг 3. Supervisor: cancel → wait(timeout) → abort

- `running: HashMap<PlatformId, (JoinHandle<()>, CancellationToken)>`.
- Стоп/рестарт: `token.cancel()` → `tokio::time::timeout(GRACEFUL_STOP, handle)`
  → если истёк, `handle.abort()` + warn-лог.
- Сам `run`: `select!` между `lifecycle.changed()` и `token.cancelled()`.

### Шаг 4. TwitchPlatformService: кооперативная отмена

- `consume_loop`: гонка `ws.next()` против `token.cancelled()`; при отмене —
  отправить WS close-frame и выйти. Подписки не удаляем вручную: Twitch чистит их
  сам при закрытии сессии (проверено доками EventSub).
- `run`: различать «отменено» и «ошибка»: отмена → `Ok(())` без reconnect;
  reconnect-`sleep(delay)` тоже под `select!` с токеном.

### Шаг 5. Фоновые таски и rule pipeline

- `queue_timeout_task` / `queue_purge_task` / `session_prune_task`:
  `select! { _ = token.cancelled() => break, _ = sleep(...) => ... }`.
- `engine.run` / `executor.run`: гонка `rx.recv()` против `cancelled()`;
  канал закрывается, недоставленные события осознанно бросаем (они не персистятся).
- `start_rule_pipeline(state, token)`.

### Шаг 6. Тесты

- Supervisor-тесты: stub-фабрика теперь получает токен — проверяем, что stop
  каскадит child-токен (stub держит `pending::<()>().until_cancelled(&token)`),
  `wait_for_finished` остаётся.
- Новый тест: повторная отмена/старт после рестарта не течёт по токенам.
- Существующие executor/queue тесты не затрагиваются (сервисам токен не нужен —
  он только у долгоживущих циклов).

### Шаг 7. Проверка

```sh
cargo check --package backend
cargo nextest run --package backend
cargo clippy --all-targets && cargo fmt --check
```

Плюс ручной smoke: `docker stop` / Ctrl+C — в логах видим «ingress stopped» до
выхода процесса, без SIGKILL-таймаута.

## Оценка

3–5 ч: шаги 1–2 ~0.5 ч, шаг 3 ~1 ч, шаг 4 ~1–1.5 ч, шаг 5 ~0.5–1 ч,
шаги 6–7 ~0.5–1 ч.
