#!/usr/bin/env bash
set -euo pipefail

REMOTE="sg-server"
REMOTE_DIR="/home/sg-server-user/.seeki"
BINARY="seeki"
TARGET="x86_64-unknown-linux-musl"
VERSION=$(cat VERSION)

usage() {
  echo "Usage: $0 {deploy|rollback|status}"
  echo ""
  echo "  deploy    Build, upload, and deploy (keeps timestamped backup)"
  echo "  rollback  Restore the previous binary"
  echo "  status    Show service status and available backups"
  exit 1
}

cmd_deploy() {
  CARGO_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
  VERSION_SEMVER=$(echo "$VERSION" | rev | cut -d. -f1-3 | rev)
  if [ "$VERSION_SEMVER" != "$CARGO_VERSION" ]; then
    echo "FATAL: VERSION ($VERSION → $VERSION_SEMVER) != Cargo.toml ($CARGO_VERSION) — sync them first"
    exit 1
  fi
  echo "==> Deploying seeki v${VERSION}"
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
    STAMP=\$(date +%Y%m%d-%H%M%S)
    if [ -f ${REMOTE_DIR}/${BINARY} ]; then
      cp ${REMOTE_DIR}/${BINARY} ${REMOTE_DIR}/${BINARY}.v${VERSION}.\${STAMP}.bak
      ln -sf ${REMOTE_DIR}/${BINARY}.v${VERSION}.\${STAMP}.bak ${REMOTE_DIR}/${BINARY}.prev
    fi
    sudo systemctl stop seeki.service
    mv /tmp/${BINARY}-deploy ${REMOTE_DIR}/${BINARY}
    chmod +x ${REMOTE_DIR}/${BINARY}
    sudo systemctl start seeki.service
    sleep 2
    if systemctl is-active --quiet seeki.service; then
      echo \"==> seeki v${VERSION} is running (backup: ${BINARY}.v${VERSION}.\${STAMP}.bak)\"
    else
      echo 'FATAL: seeki.service failed to start — rolling back'
      cp ${REMOTE_DIR}/${BINARY}.prev ${REMOTE_DIR}/${BINARY}
      sudo systemctl start seeki.service
      exit 1
    fi
  "
  echo "==> Done."
}

cmd_rollback() {
  echo "==> Rolling back on ${REMOTE}..."
  ssh -t "$REMOTE" "
    set -e
    if [ ! -e ${REMOTE_DIR}/${BINARY}.prev ]; then
      echo 'FATAL: no previous backup found'
      exit 1
    fi
    PREV=\$(readlink -f ${REMOTE_DIR}/${BINARY}.prev)
    echo \"Restoring: \${PREV}\"
    sudo systemctl stop seeki.service
    cp \"\${PREV}\" ${REMOTE_DIR}/${BINARY}
    chmod +x ${REMOTE_DIR}/${BINARY}
    sudo systemctl start seeki.service
    sleep 2
    if systemctl is-active --quiet seeki.service; then
      echo '==> seeki.service is running (rolled back)'
    else
      echo 'FATAL: seeki.service failed to start after rollback'
      exit 1
    fi
  "
  echo "==> Done."
}

cmd_status() {
  ssh "$REMOTE" "
    echo '--- Service ---'
    systemctl is-active seeki.service && echo 'Running' || echo 'Stopped'
    echo ''
    echo '--- Backups ---'
    ls -lht ${REMOTE_DIR}/${BINARY}*.bak 2>/dev/null || echo 'No backups found'
    echo ''
    echo '--- Current symlink ---'
    ls -l ${REMOTE_DIR}/${BINARY}.prev 2>/dev/null || echo 'No .prev symlink'
  "
}

case "${1:-}" in
  deploy)  cmd_deploy ;;
  rollback) cmd_rollback ;;
  status)  cmd_status ;;
  *)       usage ;;
esac
