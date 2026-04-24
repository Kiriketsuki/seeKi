#!/usr/bin/env bash
set -euo pipefail

REMOTE="sg-server"
REMOTE_DIR="/home/sg-server-user/.seeki"
BINARY="seeki"
TARGET="x86_64-unknown-linux-musl"

echo "==> Building frontend..."
(cd frontend && npm run build --silent)

echo "==> Building release binary (musl)..."
cargo build --release --target "$TARGET" 2>&1 | grep -E 'Compiling seeki|Finished|error'

LOCAL_BIN="target/${TARGET}/release/${BINARY}"
if [ ! -f "$LOCAL_BIN" ]; then
  echo "FATAL: binary not found at $LOCAL_BIN"
  exit 1
fi

echo "==> Uploading to ${REMOTE}..."
scp -q "$LOCAL_BIN" "${REMOTE}:/tmp/${BINARY}-deploy"

echo "==> Deploying on ${REMOTE} (sudo will prompt for password)..."
ssh -t "$REMOTE" "
  set -e
  sudo systemctl stop seeki.service
  cp ${REMOTE_DIR}/${BINARY} ${REMOTE_DIR}/${BINARY}.prev
  mv /tmp/${BINARY}-deploy ${REMOTE_DIR}/${BINARY}
  chmod +x ${REMOTE_DIR}/${BINARY}
  sudo systemctl start seeki.service
  sleep 2
  if systemctl is-active --quiet seeki.service; then
    echo '==> seeki.service is running'
  else
    echo 'FATAL: seeki.service failed to start — rolling back'
    cp ${REMOTE_DIR}/${BINARY}.prev ${REMOTE_DIR}/${BINARY}
    sudo systemctl start seeki.service
    exit 1
  fi
"

echo "==> Done."
