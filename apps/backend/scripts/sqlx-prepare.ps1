# Regenerates sqlx offline metadata (.sqlx) after query or migration changes.
# Rule: changed a query or a migration -> run this -> commit updated .sqlx.
$ErrorActionPreference = "Stop"

$backendDir = Join-Path $PSScriptRoot ".."
$env:DATABASE_URL = "sqlite:data/query-check.db"

Push-Location $backendDir
try {
    sqlx database create
    if ($LASTEXITCODE -ne 0) { throw "sqlx database create failed" }

    sqlx migrate run
    if ($LASTEXITCODE -ne 0) { throw "sqlx migrate run failed" }

    cargo sqlx prepare -- --all-targets
    if ($LASTEXITCODE -ne 0) { throw "cargo sqlx prepare failed" }
} finally {
    Pop-Location
}
