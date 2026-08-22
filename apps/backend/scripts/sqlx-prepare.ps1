# Regenerates sqlx offline metadata (.sqlx) after query or migration changes.
# Rule: changed a query or a migration -> run `just sqlx-prepare` -> commit updated .sqlx.
$ErrorActionPreference = "Stop"

$backendDir = Join-Path $PSScriptRoot ".."
$repoDir = Join-Path (Join-Path $backendDir "..") ".."
$dbPath = Join-Path $repoDir "data\server.db"
New-Item -ItemType Directory -Force -Path (Split-Path $dbPath) | Out-Null
# Same DB the app uses via .env; absolute because query! macros expand under a
# rustc whose cwd is the workspace root.
$env:DATABASE_URL = "sqlite:" + ($dbPath -replace '\\', '/') + "?mode=rwc"
Remove-Item Env:SQLX_OFFLINE -ErrorAction SilentlyContinue

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
