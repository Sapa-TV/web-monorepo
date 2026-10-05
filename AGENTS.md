# Monorepo AGENTS.md

## Тесты

Запускаем тесты через `cargo nextest` (установлен; в nextest есть дефолтные
таймауты, `cargo test` может висеть бесконечно и ломает процесс работы).

```sh
# backend
cargo nextest run --package backend

# конкретный модуль/фильтр (например, platform::)
cargo nextest run --package backend -E 'test(platform::)'

# с повышенным таймаутом на тест (сек), например для тестов с реальным I/O
cargo nextest run --package backend --test-threads=1 --slow-timeout=600s
```

Полезные флаги:

- `--test-threads=1` — последовательный запуск (для диагностики
  зависаний/гонок).
- `--no-capture` / `-n` — показать вывод тестов.
- `--slow-timeout=<dur>` — превышение таймаута считает тест медленным (и
  форсирует вывод).
- `--fail-fast`/`--no-fail-fast` — остановиться/не останавливаться при первом
  падении.
- `-E 'test(<выражение>)'` — фильтр по имени теста (contains).

Проверка компиляции короче: `cargo check --package backend`. Линт/мойка:
`cargo clippy --all-targets` и `cargo fmt --check`.

## Запуск dev

- `pnpm dev` — обычный запуск (backend + frontend, логи от `info`).
- `pnpm dev:debug` — то же самое, но backend с `RUST_LOG=debug` (переменная
  подмешивается из `.env.debug` через dotenv; backend читает `RUST_LOG` в
  `main.rs` через `EnvFilter`).
- Backend дублирует логи в файл: `logs/backend.log.<дата>` (rolling по дням,
  tracing-appender, настраивается в `main.rs`). Консоль и файл с разными
  фильтрами: консоль — `RUST_LOG` (дефолт `info`), файл — `RUST_LOG_FILE`
  (дефолт `debug`).

## Комментарии

- Комментарии в коде почти не нужны — не пиши их без необходимости.
- Если комментарий всё-таки нужен, пиши его только на английском.
- Русские комментарии в коде не использовать: кириллица в комментариях ломается
  из-за кодировки.
