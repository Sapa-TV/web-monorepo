# Workspace tasks
#   just          — list recipes
#   just run      — run the backend
#   just gen      — generate OpenAPI spec to generated/openapi.json
#   just lint     — run custom ast-grep lints (own code only)
#   just lint-test — run ast-grep rule tests

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

# Custom ast-grep lints, only own code, all targets
lint:
    pnpm exec ast-grep scan

# Run ast-grep rule test suites
lint-test:
    pnpm exec ast-grep test

# Encrypt deploy/.env -> deploy/.env.sops (needs sops + age, uses deploy/.sops.yaml)
encrypt-env:
    sops -e --input-type dotenv --output-type dotenv deploy/.env > deploy/.env.sops

# Decrypt deploy/.env.sops -> deploy/.env (local inspection only, file is gitignored)
decrypt-env:
    sops -d --input-type dotenv --output-type dotenv deploy/.env.sops > deploy/.env
