# Workspace tasks
#   just          — list recipes
#   just run      — run the backend
#   just gen      — generate OpenAPI spec to generated/openapi.json
#   just lint     — run custom ast-grep lints (own code only)
#   just lint-test — run ast-grep rule tests (CI-ready)
#   just astg-update — refresh ast-grep test snapshots after editing rules/cases
#   just astg-one   — run one ast-grep rule test suite (id = rule id)
#   just astg-inspect — show why files/rules are scanned or skipped
#   just astg-pat   — check an ast-grep pattern against Rust code from stdin

set dotenv-load

# Default: show recipes
default:
    @just --list

# Run the backend (port from config.toml / PORT env, default 3000)
run:
    cargo run -p backend

# Generate OpenAPI spec file
gen path='generated/openapi.json':
    cargo run -p gen-openapi -- {{path}}

# Generate OpenAPI spec + TS REST client for @sapa-tv-ru/api-client
gen-client:
    just gen
    pnpm --dir packages/api-client exec swagger-typescript-api generate --path ../../generated/openapi.json --output ./generated --name Api --templates ./templates --modular --clean-output

# Custom ast-grep lints, only own code, all targets
lint:
    pnpm exec ast-grep scan

# Run ast-grep rule test suites
lint-test:
    pnpm exec ast-grep test

# Refresh committed ast-grep rule test snapshots after editing rules/cases
astg-update:
    pnpm exec ast-grep test -U

# Run a single ast-grep rule test suite, e.g. `just astg-one no-control-flow-in-api`
astg-one id:
    pnpm exec ast-grep test -f {{id}}

# Show why ast-grep scans or skips each file/rule (debug `files`/`ignores` scoping)
astg-inspect:
    pnpm exec ast-grep scan --inspect entity

# Check an ast-grep pattern against Rust code from stdin, e.g. `code | just astg-pat 'if $C { $$$B }'`
astg-pat pattern:
    pnpm exec ast-grep run -p '{{pattern}}' -l rust --stdin

# Encrypt deploy/.env -> deploy/.env.sops (needs sops + age, uses deploy/.sops.yaml)
encrypt-env:
    sops -e --input-type dotenv --output-type dotenv deploy/.env > deploy/.env.sops

# Decrypt deploy/.env.sops -> deploy/.env (local inspection only, file is gitignored)
decrypt-env:
    sops -d --input-type dotenv --output-type dotenv deploy/.env.sops > deploy/.env
