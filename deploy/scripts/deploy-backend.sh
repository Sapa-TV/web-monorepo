#!/usr/bin/env bash
# Transactional backend deploy, run on the VPS via: ssh ... "bash -s" < deploy/scripts/deploy-backend.sh
# Expects: current dir = /srv/web-app/app, sops + docker available, GHCR login already done.
set -euo pipefail

APP_DIR=/srv/web-app/app
IMAGE=ghcr.io/sapa-tv/backend
BASE_IMAGE="$IMAGE:latest"
HEALTH_URL=http://127.0.0.1:3000/api/health
SMOKE_NAME=backend-smoke
SMOKE_PORT=3001
SMOKE_URL=http://127.0.0.1:$SMOKE_PORT/api/health
SMOKE_TRIES=30
SMOKE_INTERVAL=5

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

# -- Helpers ------------------------------------------------------------------
cid_primary() {
  docker compose ps -aq backend 2>/dev/null | head -n1
}

cid_smoke() {
  docker ps -aq --filter "name=^${SMOKE_NAME}$" 2>/dev/null | head -n1
}

# Result codes: 0 = healthy, 1 = container errored, 2 = healthy timeout.
wait_healthy() {
  local url="$1" resolver="$2"
  local cid state
  for _ in $(seq 1 $SMOKE_TRIES); do
    cid="$($resolver)"
    if [ -n "$cid" ]; then
      state="$(docker inspect --format '{{.State.Status}}' "$cid" 2>/dev/null || true)"
      case "$state" in
        exited | dead | restarting | paused)
          echo "ERROR: container '$cid' is $state (exit code $(docker inspect --format '{{.State.ExitCode}}' "$cid" 2>/dev/null || echo n/a))" >&2
          return 1
          ;;
      esac
    fi
    if curl -fs --max-time 3 "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep $SMOKE_INTERVAL
  done
  echo "TIMEOUT: container stayed alive but $url did not respond within $((SMOKE_TRIES * SMOKE_INTERVAL))s" >&2
  return 2
}

dump_container_diagnostics() {
  local cid="$1"
  echo "--- container list ---"
  docker compose ps -a || true
  if [ -n "$cid" ]; then
    echo "--- container state ($cid) ---"
    docker inspect --format 'status={{.State.Status}} exit_code={{.State.ExitCode}} error={{.State.Error}} image={{.Image}}' "$cid" 2>/dev/null || true
    echo "--- container logs (tail 200) ---"
    docker logs --tail=200 "$cid" 2>&1 || true
  fi
}

cleanup_smoke() {
  docker rm -f "$SMOKE_NAME" >/dev/null 2>&1 || true
}
trap cleanup_smoke EXIT

# -- Pull new image -----------------------------------------------------------
docker compose pull

# -- Smoke test new image before touching the running container ---------------
cleanup_smoke
docker run -d --name "$SMOKE_NAME" \
  --env-file .env \
  -v "$APP_DIR/config.toml:/app/config.toml:ro" \
  -v "$APP_DIR/data:/app/data" \
  -p "127.0.0.1:$SMOKE_PORT:3000" \
  "$BASE_IMAGE" \
  >/dev/null

smoke_rc=0
wait_healthy "$SMOKE_URL" cid_smoke || smoke_rc=$?
echo "--- $SMOKE_NAME logs (tail 200) ---"
docker logs --tail=200 "$SMOKE_NAME" 2>&1 || true
cleanup_smoke
if [ "$smoke_rc" -ne 0 ]; then
  echo "ABORT: new image failed smoke test (rc=$smoke_rc), production container untouched" >&2
  exit "$smoke_rc"
fi
echo "Smoke test passed"

# -- Apply new image ----------------------------------------------------------
docker compose up -d

health_rc=0
wait_healthy "$HEALTH_URL" cid_primary || health_rc=$?
if [ "$health_rc" -eq 0 ]; then
  echo "Backend is healthy"
  exit 0
fi

echo "--- failed new container diagnostics ---"
dump_container_diagnostics "$(cid_primary)"
echo "New backend did not become healthy, rolling back" >&2

# -- Rollback -----------------------------------------------------------------
if [ -n "$OLD_IMAGE" ]; then
  docker tag "$OLD_IMAGE" "$BASE_IMAGE"
  docker compose up -d --force-recreate
  rollback_rc=0
  wait_healthy "$HEALTH_URL" cid_primary || rollback_rc=$?
  if [ "$rollback_rc" -eq 0 ]; then
    echo "Rolled back to previous image ($OLD_IMAGE)" >&2
    exit 1
  fi
  echo "--- rollback container diagnostics ---"
  dump_container_diagnostics "$(cid_primary)"
fi

echo "Rollback also failed, backend left in unknown state" >&2
exit 1