#!/usr/bin/env bash
# Transactional backend deploy, run on the VPS via: ssh ... "bash -s" < deploy/scripts/deploy-backend.sh
# Expects: current dir = /srv/web-app/app, sops + docker available, GHCR login already done.
set -euo pipefail

APP_DIR=/srv/web-app/app
IMAGE=ghcr.io/sapa-tv/backend

cd "$APP_DIR"

# -- Config -------------------------------------------------------------------
# Regenerate .env from encrypted source (fail hard if sops is missing/broken)
sops -d --input-type dotenv --output-type dotenv .env.sops > .env
chmod 600 .env

# -- Remember current image ---------------------------------------------------
# Whatever is actually running now (resolved image ID) becomes the rollback target.
OLD_IMAGE=""
if container_id="$(docker compose ps -q backend)"; then
  OLD_IMAGE=$(docker inspect --format '{{.Image}}' "$container_id" 2>/dev/null || true)
fi

# -- Apply new image ----------------------------------------------------------
docker compose pull
docker compose up -d

# -- Verify -------------------------------------------------------------------
healthcheck() {
  for i in $(seq 1 30); do
    if curl -fs http://127.0.0.1:3000/health >/dev/null; then
      return 0
    fi
    sleep 5
  done
  return 1
}

if healthcheck; then
  echo "Backend is healthy"
  exit 0
fi

# -- Rollback -----------------------------------------------------------------
echo "New backend did not become healthy, rolling back" >&2
if [ -n "$OLD_IMAGE" ]; then
  docker tag "$OLD_IMAGE" "$IMAGE:latest"
  docker compose up -d --force-recreate
  if healthcheck; then
    echo "Rolled back to previous image ($OLD_IMAGE)" >&2
    exit 1
  fi
fi

echo "Rollback also failed, backend left in unknown state" >&2
exit 1