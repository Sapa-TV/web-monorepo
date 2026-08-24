# Юз-кейс/e2e тесты бэка: межмодульные сценарии

Статус: **выполнен** (2026-08-24). Пункт бэклога #1: «Добавить юз-кейс/e2e тесты
(сейчас межмодульные сценарии вроде «ротация PAK → старый ключ на widget-эндпоинте
не работает» покрыты только unit-тестами)».

Дата: 2026-08-24

## Что понимаем под e2e

**Не** браузерные тесты (у фронта есть пустой playwright-config — это отдельный трек),
а in-process сценарии бэкенда: реальный axum-роутер целиком (`api::router` +
`widget_api::router`), sqlite, реальные сервисы и пайплайн правил — без сети и Twitch.
Именно такие тесты ловят то, что не видят юниты: middleware-обвязку, пути роутинга,
сериализацию DTO на стыке модулей, порядок подписок broadcast-каналов.

## Контекст (текущее состояние)

- Фикстуры `test_fixtures.rs`: `test_state()` (sqlite), `test_router(state)`
  (полный роутер), `session_cookie()` (login-ticket → сессионная cookie),
  `api_path()`. Всё под `#[cfg(test)]` → недоступно интеграционным тестам из `tests/`,
  поэтому сценариевый модуль живёт **внутри крейта**: `src/usecases/mod.rs`
  (+ подпапки по доменам).
- Инъекция платформенных событий: `EventIngress::publish()` уже существует
  (`#[allow(dead_code)]` ждёт именно таких тестов): dedup → broadcast → RuleEngine.
- `runtime::start_rule_pipeline(state)` поднимает связку RuleEngine + ActionExecutor
  над AppState.
- WAK-middleware `require_key` сравнивает Bearer с `config.widget_access_key()`;
  ротация — `POST /api/admin/widget-access-key`.
- wapi: `/wapi/queue` CRUD, `/wapi/queue/anonymous`, `/wapi/queue/next`,
  `/queue/{id}/complete|cancel`, `/queue/stats`; админка: `/admin/actions`,
  `/admin/rules`, `/admin/rewards`, `/admin/roulette/slots|rarities`.

## Инфраструктура (шаг 1)

В `src/usecases/mod.rs`:

- `async fn json_request(app, method, uri, headers, body) -> Response` — убрать
  копипасту ручной сборки `Request` из существующих api-тестов;
- `async fn wait_until(pred, timeout)` — polling с интервалом 10мс: пайплайн правил
  асинхронный, ассерты не должны спать фиксированное время;
- `fn bearer(key)` / переиспользование `session_cookie()`.
- Модули-сценарии: `wak_rotation.rs`, `roulette_flow.rs`, `rules_pipeline.rs`,
  `reward_redemption.rs`, `sessions.rs`.

## Сценарии

### S1. Ротация widget access key (заголовочный кейс)

1. Админ-сессия; GET текущего ключа → k1.
2. `GET /wapi/queue` с Bearer k1 → 200.
3. `POST /admin/widget-access-key` → k2.
4. Запрос с k1 → **401**, с k2 → 200; GET ключа отдаёт k2.
5. Повторная ротация: k2 тоже перестаёт работать (ключ одноразовый на поколение).

### S2. Полный цикл очереди глазами док-оператора

1. Через админ-API создать rarity → slot (вес 100).
2. Анонимный зритель: `POST /wapi/queue/anonymous {name}` → entry Pending;
   повтор с другим именем мапится на того же guest-юзера.
3. `POST /wapi/queue/next` → Spinning + слот; второй параллельный next → 409.
4. `POST /queue/{id}/complete` → Completed; stats показывают ожидаемые счётчики.
5. Второй entry → cancel вместо complete.

### S3. Правило → экшен → очередь (главный межмодульный)

1. `start_rule_pipeline(state)`.
2. Через API создать action EnqueueRoulette и правило
   chat_message / equals «!spin» → этот action.
3. Подписки готовы → `ingress.publish(chat_message "!spin", user u1)`.
4. `wait_until(в /wapi/queue появился entry юзера u1)`.
5. Дедуп: тот же event_id повторно — второй записи нет.
6. Правило выключить (PATCH enabled=false) → новое сообщение не создаёт запись.
7. Чужое сообщение («!raffle») не триггерит.

### S4. Награда → правило

1. Создать награду через `/admin/rewards`, правило trigger=reward_redemption,
   reward_id=<id> → action ChatReply (или EnqueueRoulette).
2. `publish(reward_redemption c этим reward_id)` → эффект правила наступил
   (запись в очереди / проверяемое действие).

### S5. Жизненный цикл сессии

1. Login-ticket → обмен на cookie (фикстура session_cookie уже делает это —
   сценарий прогоняет вручную через два запроса).
2. Админские руты доступны с cookie, без — 401, у не-админа — 403.
3. Logout → cookie больше не работает.

## Осознанно вне скоупа

- Twitch OAuth/callback и ingress twitch-воркер (нужен mock Twitch API — отдельный пункт).
- Реальный WebSocket-хендшейк в сценариях: tower oneshot не прокидывает upgrade;
  ws-авторизация покрыта юнитами `handle_message`/middleware.
- Браузерный playwright-трек фронта (конфиг есть, тестов нет — отдельный пункт бэклога
  при необходимости).

## Проверка

```sh
cargo nextest run --package backend -E 'test(usecases)'
cargo nextest run --package backend        # регресс целиком
cargo clippy --all-targets && cargo fmt --check
```

## Оценка

4–6 ч: инфраструктура ~0.5 ч, S1+S2 ~1.5 ч, S3 ~1–1.5 ч, S4 ~0.5–1 ч,
S5 ~0.5 ч, полировка/регресс ~0.5 ч.

## Итог (отличия от плана)

Выполнены все сценарии; nextest 367 passed (6 юзкейсов), clippy/fmt чисто.
Отклонения:

- **Файлы без mod.rs** — `src/usecases.rs` + `usecases/*.rs` (2024-style),
  замечание с ревью шага 1.
- **request_json ставит `Content-Type: application/json`** при наличии тела:
  баг найден сценарием S2 — без заголовка POST падал с 415 (S1 не ловил,
  rotate не читает тело).
- `start_rule_pipeline` после работы graceful-shutdown требует
  `CancellationToken` — в сценариях передаётся свежий токен.
- Обновление правил — **PUT**, не PATCH: добавлен хелпер `put_json`,
  а запланированный `patch_json` удалён за ненадобностью.
- **S4 скорректирован**: `/admin/rewards` — список наград из Twitch (нужен живой
  Twitch и валидный токен), локально награды не создаются. Правило матчит
  произвольный строковый `reward_id`, поэтому сценарий использует фиксированный id
  + негативный кейс чужой награды вместо создания награды через API.
- **S5 расширен**: проверено тело `/sessions/me`, одноразовость login-ticket
  (повторный обмен → 400), logout без сессии → 401.
- Сценарий S1 дополнен третьей ротацией: каждое поколение ключей аннулирует все
  предыдущие (k3 живёт, k2 умирает).
- Инфраструктура: `wait_until` polling, `json_request/get/post/put` хелперы,
  временный `allow(dead_code)` снят после наполнения сценариями.
