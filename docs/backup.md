# Backups

The production state is a single SQLite database:
`/srv/web-app/app/data/server.db` (WAL mode). Everything else lives in git, so
only this file is backed up.

## Where copies live

| Location                                   | Retention | Encrypted   |
| ------------------------------------------ | --------- | ----------- |
| VPS `/srv/backups/sapa`                    | 14 days   | no          |
| Local workstation                          | 30 days   | no          |
| GitHub Actions artifact `sapa-db-<run id>` | 30 days   | yes (`age`) |

The nightly snapshot is taken by `deploy/backup/backup-db.sh` through
`sapa-db-backup.timer` (04:00 server time, catches up after downtime). The
script uses `sqlite3 .backup`, so concurrent writes are safe and the backend
stays up. Copies whose `PRAGMA integrity_check` does not return `ok` are never
kept.

The units live in `deploy/backup/` and are installed once as root:

```sh
install -m 644 deploy/backup/sapa-db-backup.{service,timer} /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now sapa-db-backup.timer
```

Every deploy also takes a snapshot before the new image starts
(`deploy-backend.sh`).

## GitHub artifact

Artifacts expire and are readable by any GitHub account, because the repository
is public. They are therefore encrypted with the `age` recipient from
`.sops.yaml` before upload. The private key stays on the workstation:
`%APPDATA%\sops\age\keys.txt`.

Download and decrypt:

```sh
gh run download -n sapa-db-<run id>
age -d -i "$APPDATA/sops/age/keys.txt" -o sapa-db.db.gz sapa-db.db.gz.age
gzip -d sapa-db.db.gz
```

The repository's scheduled workflows are disabled after 60 days without
repository activity, so re-enable the workflow if it stops running.

## Restore

```sh
cd /srv/web-app/app
docker compose stop backend
cp /srv/backups/sapa/sapa-YYYY-MM-DD_HHMMSS.db.gz /tmp/restore.db.gz
gzip -d /tmp/restore.db.gz
rm -f data/server.db data/server.db-wal data/server.db-shm
mv /tmp/restore.db data/server.db
chown 1000:1000 data/server.db
docker compose up -d backend
curl -fs http://127.0.0.1:3000/api/health
```

Migrations run at startup, so an older dump is upgraded automatically. A dump
made by a newer image cannot be used with an older image.
