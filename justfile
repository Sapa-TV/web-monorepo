# Workspace tasks
#   just          — list recipes
#   just run      — run the backend
#   just gen      — generate OpenAPI spec to generated/openapi.json
#   just dylint   — run custom dylint lints (own code only, no deps)

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

# Custom dylint lints, only own code, all targets
dylint:
    cargo dylint --all --no-deps -- --all-targets
