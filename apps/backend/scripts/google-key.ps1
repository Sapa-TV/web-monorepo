# Prints GOOGLE_SERVICE_ACCOUNT_KEY_BASE64=<base64 of the key file> for .env.
# Usage: just google-key [path]   (default: secrets/google-sa.json)
param(
    [string]$Path = "secrets/google-sa.json"
)

$ErrorActionPreference = "Stop"

if (!(Test-Path $Path)) { throw "file not found: $Path" }
$b64 = [Convert]::ToBase64String([IO.File]::ReadAllBytes((Resolve-Path $Path)))
Write-Output "GOOGLE_SERVICE_ACCOUNT_KEY_BASE64=$b64"
