#!/usr/bin/env bash
set -euo pipefail

DB="${BACKUP_DB:-/srv/web-app/app/data/server.db}"
DEST="${BACKUP_DIR:-/srv/backups/sapa}"
KEEP_DAYS="${BACKUP_KEEP_DAYS:-14}"

[ -f "$DB" ] || {
  echo "database not found: $DB" >&2
  exit 1
}

install -d -m 750 "$DEST"
umask 077

tmp="$(mktemp /tmp/sapa-db-XXXXXX.db)"
trap 'rm -f "$tmp"' EXIT

sqlite3 "$DB" ".backup '$tmp'"

integrity="$(sqlite3 "$tmp" 'PRAGMA integrity_check;')"
[ "$integrity" = "ok" ] || {
  echo "integrity check failed: $integrity" >&2
  exit 1
}

target="$DEST/sapa-$(date +%F_%H%M%S).db.gz"
gzip -9 -c "$tmp" >"$target.part"
mv "$target.part" "$target"

if [ "$(id -u)" -eq 0 ]; then
  chown "${BACKUP_OWNER:-deploy:deploy}" "$target"
fi

find "$DEST" -maxdepth 1 -type f -name 'sapa-*.db.gz' -mtime +"$KEEP_DAYS" -delete

echo "$target"
