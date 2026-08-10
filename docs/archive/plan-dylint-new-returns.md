# Dylint: линт `new_returns_self` (new возвращает только чистый `Self`)

## Цель

Запретить любые возвраты `fn new`, кроме прямого `-> Self`. Отклонять:
`Option<Self>`, `Result<Self, _>`, `Box<Self>`, `Arc<Self>`, `impl Trait`, неявный
возврат (`-> ()`).

`clippy::new_ret_no_self` такого не закрывает (он разрешает `Result<Self, _>` и др.) —
поэтому добавляем **пользовательский dylint-линт**.

Подключение в коде — `cfg_attr + deny` (т.к. имя custom-линта неизвестно
rustc, когда dylint не запущен).

## Исследование (факты на момент составления плана)

- Toolchain: `nightly-1.97.0` (`2026-05-14`, MSVC, default). Компоненты `rustc-dev`,
  `dylint-link` и бинарники `cargo-dylint` **не установлены**; `rust-toolchain.toml`
  отсутствует.
- Все 25 `fn new` в кодовой базе уже возвращают чистый `Self` → линт нигде не
  сработает, build останется успешным; правило работает как защита на будущее.
- Сборка: `edition 2024`, `[workspace.lints.clippy]` в корневом `Cargo.toml`,
  `[lints] workspace = true` в `apps/backend/Cargo.toml`, `clippy.toml` (msrv 1.96,
  allow-unwrap-in-tests).
- Как работает dylint: custom-линт = отдельная `cdylib`-крейт с
  `rustc::LateLintPass`; регистрируется в `[workspace.metadata.dylint]`; запуск
  `cargo dylint --all`; нужен `[workspace.lints.rust.unexpected_cfgs]` с
  `check-cfg = ["cfg(dylint_lib, values(any()))"]`.

## Новые/изменённые файлы

```
rust-toolchain.toml                        ← pin nightly + rustc-dev, dylint-link
lints/new_returns_self/                    ← отдельный workspace с крейтом линта
  Cargo.toml
  src/lib.rs                               ← declare_lint! + LateLintPass
  ui/...                                   ← ui_test кейсы (ok / не ok)
Cargo.toml (корень)                        ← [workspace.metadata.dylint] + unexpected_cfgs
apps/backend/src/main.rs                    ← #![cfg_attr(dylint_lib=.., deny(..))]
```

## Шаг 1: Toolchain

`rust-toolchain.toml` (закреплённая версия для воспроизводимости):

```toml
[toolchain]
channel = "nightly-2026-05-14"
components = ["rustc-dev", "dylint-link", "rust-src"]
profile = "minimal"
```

Команды:

```sh
rustup install nightly-2026-05-14 --profile minimal --component rustc-dev --component dylint-link
rustup override set nightly-2026-05-14   # если не хотим ставить default
cargo install cargo-dylint
```

Примечание: версия `cargo-dylint` / `dylint_linting` должна совпасть с закреплённым
nightly (иначе расхождение по rustc-интернам). При конфликте — скорректировать pin.
`rustc-dev` — объёмная загрузка (сотни МБ).

## Шаг 2: Крейт линта `lints/new_returns_self/`

Отдельный workspace (свой `[workspace]`), **не** member `apps/backend`, чтобы
`rustc_private` / `rustc-dev` не попадали в сборку приложения. Вложенный workspace
автоматически исключается из корневого.

`Cargo.toml`:

- `crate-type = ["cdylib"]`
- `rustc_private = true`
- deps: `dylint_linting`, `rustc_driver` (через `dylint_library!`), обёртки из
  `rustc_*`/`clippy_utils` при необходимости
- `[package.metadata.rust-analyzer] rustc_private = true`
- `[package.metadata.dylint]` / lints от dylint

`src/lib.rs` (скелет):

```rust
extern crate rustc::driver::rustc_driver;

dylint_linting::declare_lint!(
    LateLintPass,             // pass kind
    CheckNewReturnsSelf,      // name-of-pass-fn struct
    "new_returns_self",       // lint name
    "`new` must return `Self` directly (no Option/Result/Box/Arc/impl)",
);
```

Логика в `check_impl_item`:

- если имя ассоциированной fn == `new`:
  - `FnRetTy::DefaultReturn` → выдаём предупреждение;
  - `FnRetTy::Return(ty)` → `ty.kind` == `TyKind::Path(QPath::Resolved(None, path))`,
    где последний сегмент — `Self` (или `Res::SelfTy`) → ок; **всё остальное**
    (`Option<Self>`, `Result<Self,_>`, `Box<Self>`, `Arc<Self>`, `impl Trait`) → `span_lint`.
- emit: `cx.span_lint("new_returns_self", impl_item.span, "`new`must return`Self` directly")`.

Каркас: `cargo dylint new new_returns_self`, затем заменить код. Добавить `ui_test`
кейсы (валидный `-> Self` и невалидные обёртки).

## Шаг 3: Регистрация (корневой `Cargo.toml`)

```toml
[workspace.metadata.dylint]
libraries = [{ path = "lints", pattern = "*" }]

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ["cfg(dylint_lib, values(any()))"]
```

`[workspace.lints.rust.unexpected_cfgs]` — снимает предупреждения про неизвестный
`cfg(dylint_lib)` у `cfg_attr`-allow/deny в коде. `apps/backend/Cargo.toml` уже
`[lints] workspace = true` → наследует.

## Шаг 4: Подключение в коде

`apps/backend/src/main.rs` (корень bin), самая верхняя строка:

```rust
#![cfg_attr(dylint_lib = "new_returns_self", deny(new_returns_self))]
```

В обычной сборке (без dylint) имя линта не известно rustc, `cfg_attr` убирается в 0 —
никаких unknown-lint ошибок.

## Шаг 5: Запуск и CI

- Локально: `cargo dylint --all` (в корне workspace).
- CI (опционально): шаг с `cargo install cargo-dylint` + `cargo dylint --all`;
  кешировать `target/dylint/`, `~/.dylint_drivers/`, `~/.rustup/toolchains/`.
- (Опционально) rust-analyzer: `check.overrideCommand: ["cargo","dylint","--all",...]` в настройках VS Code.

## Открытые вопросы

- Pin: `nightly-2026-05-14` соответствует установленной. Если позже захочется другой
  nightly — обновить и pin, и версию `cargo-dylint`/`dylint_linting`.
- Обрабатывать ли trait fn (`fn new` в трейтах), или только импл.
- Нужен ли аналог для `with_*` / `build` / `try_new` — пока только `new`.
- Добавлять ли CI-шаг сейчас или отдельно.

## Изменения при реализации

### Инструментарий

- `dylint-link` ставится `cargo install dylint-link` (v6.0.3): rustup-компонента для
  этого nightly недоступна. В `rust-toolchain.toml` компоненты только `rustc-dev`, `rust-src`.

### Крейт линта

- Pin внутри крейта выровнен на `nightly-2026-05-14` (шаблон генерил другой).
- `clippy_utils` из каркаса убран: его git-rev не компилируется с pin-версией, линт его не использует.
- API rustc: `Res::SelfTy` → `Res::SelfTyParam`/`Res::SelfTyAlias`;
  `Res` из `rustc_hir::def::Res`.
- Эмиссия: `span_lint` удалён → `emit_span_lint` + `DiagDecorator`.

### UI-тест

- Написан вручную; для `impl Debug` нужен `#[derive(Debug)]` (иначе E0277 до линта).
- `--bless` через `dylint_testing::ui_test` не работает — `.stderr` собран из фактического вывода.

### Запуск

- `cargo dylint --all` линтит и зависимости; для своего workspace:
  `cargo dylint --all --no-deps -- --all-targets`.
- deny проверен end-to-end временным `fn new() -> Option<Self>` (отклоняется), probe удалён,
  `cargo dylint --all --no-deps -- --all-targets` → 0; `cargo clippy -p backend` зелёный.
