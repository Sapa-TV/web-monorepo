# new_returns_self

Custom dylint lint for the `backend` workspace.

### What it does

Requires that a `fn new` constructor return `Self` directly.

### Why is this bad?

`clippy::new_ret_no_self` allows `Result<Self, _>`, `Option<Self>`, etc. This workspace
wants a stricter contract: a `new` must return exactly `Self` — nothing wrapped
(`Option`, `Result`, `Box`, `Arc`, `impl Trait`) and no unit `()`.

### Known problems

- Only checks `ImplItem` (`fn new` inside an `impl`). Trait methods named `new`
  (`check_trait_item`) are not yet covered.
- Windows: the `dylint-link` rustup component is unavailable for the pinned
  nightly, so `dylint-link` is installed via `cargo install dylint-link` instead.

### Example

```rust
struct Foo;

impl Foo {
    fn new() -> Self {
        Self
    }
}
```