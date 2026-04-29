#!/usr/bin/env bash
set -euo pipefail

# ── deployment config ────────────────────────────────────────────────
# Override these via env vars or edit for your target server.
: "${SSH_TARGET:="seeki-server"}"
: "${REMOTE_DIR:="/home/seeki/.seeki"}"
: "${REMOTE_BIN:="${REMOTE_DIR}/seeki"}"
: "${SERVICE:="seeki.service"}"

MUSL_TARGET="x86_64-unknown-linux-musl"
LOCAL_BINARY="target/${MUSL_TARGET}/release/seeki"

usage() {
    cat <<'EOF'
Usage: redeploy.sh [OPTIONS]

Redeploy SeeKi to a remote server. Defaults to: build, backup, upload, restart.

The SSH target is read from $SSH_TARGET (default: "seeki-server").
Configure your SSH host alias in ~/.ssh/config.

Build:
  --no-build          Skip local cargo build (use existing binary)
  --gnu               Use glibc target instead of musl (not recommended for older servers)

Cleanup:
  --wipe-config       Delete seeki.toml (will need manual re-creation)
  --wipe-db           Delete seeki.db (internal SQLite — saved views/preferences only, NOT the target database)
  --wipe-backups      Delete all *.bak, *.old, *.broken files
  --fresh             Nuclear: wipe config + db + backups (keeps SSH keys and other files)
  --no-backup         Don't backup current binary before replacing

Deploy:
  --no-restart        Upload only, don't restart the service
  --dry-run           Show what would happen, don't execute

  -h, --help          Show this help
EOF
    exit 0
}

# ── defaults ─────────────────────────────────────────────────────────
DO_BUILD=true
USE_GNU=false
WIPE_CONFIG=false
WIPE_DB=false
WIPE_BACKUPS=false
NO_BACKUP=false
NO_RESTART=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-build)      DO_BUILD=false ;;
        --gnu)           USE_GNU=true ;;
        --wipe-config)   WIPE_CONFIG=true ;;
        --wipe-db)       WIPE_DB=true ;;
        --wipe-backups)  WIPE_BACKUPS=true ;;
        --fresh)         WIPE_CONFIG=true; WIPE_DB=true; WIPE_BACKUPS=true ;;
        --no-backup)     NO_BACKUP=true ;;
        --no-restart)    NO_RESTART=true ;;
        --dry-run)       DRY_RUN=true ;;
        -h|--help)       usage ;;
        *) echo "Unknown option: $1"; usage ;;
    esac
    shift
done

run() {
    echo "  -> $*"
    if ! $DRY_RUN; then
        "$@"
    fi
}

remote() {
    if $DRY_RUN; then
        echo "  -> ssh ${SSH_TARGET}: $1"
    else
        ssh "$SSH_TARGET" "$1"
    fi
}

# ── build ────────────────────────────────────────────────────────────
if $DO_BUILD; then
    if $USE_GNU; then
        echo "Building release binary (glibc)..."
        run cargo build --release
        LOCAL_BINARY="target/release/seeki"
    else
        echo "Building release binary (musl, static)..."
        run cargo build --release --target "$MUSL_TARGET"
    fi
fi

if ! $DRY_RUN && [[ ! -f "$LOCAL_BINARY" ]]; then
    echo "Error: ${LOCAL_BINARY} not found."
    exit 1
fi

# ── stop service ─────────────────────────────────────────────────────
echo "Stopping seeki..."
remote "pkill -x seeki 2>/dev/null || true; sleep 1"

# ── backup current binary ───────────────────────────────────────────
if ! $NO_BACKUP; then
    echo "Backing up current binary..."
    TIMESTAMP=$(date +%Y%m%d-%H%M%S)
    remote "
        if [[ -f ${REMOTE_BIN} ]]; then
            cp ${REMOTE_BIN} ${REMOTE_BIN}.${TIMESTAMP}.bak
            ln -sf ${REMOTE_BIN}.${TIMESTAMP}.bak ${REMOTE_DIR}/seeki.prev
            echo \"Backed up as seeki.${TIMESTAMP}.bak\"
        else
            echo 'No existing binary to back up'
        fi
    "
fi

# ── wipe operations ─────────────────────────────────────────────────
if $WIPE_BACKUPS; then
    echo "Wiping old backups..."
    remote "rm -f ${REMOTE_DIR}/seeki.*.bak ${REMOTE_DIR}/seeki.old ${REMOTE_DIR}/seeki.broken ${REMOTE_DIR}/seeki.prev"
fi

if $WIPE_DB; then
    echo "Wiping local database (saved views/preferences)..."
    remote "rm -f ${REMOTE_DIR}/seeki.db ${REMOTE_DIR}/seeki.db-shm ${REMOTE_DIR}/seeki.db-wal"
fi

if $WIPE_CONFIG; then
    echo "Wiping seeki.toml..."
    remote "rm -f ${REMOTE_DIR}/seeki.toml"
fi

# ── upload new binary ───────────────────────────────────────────────
echo "Uploading new binary..."
if ! $DRY_RUN; then
    scp "$LOCAL_BINARY" "${SSH_TARGET}:/tmp/seeki"
fi
remote "mv /tmp/seeki ${REMOTE_BIN} && chmod 755 ${REMOTE_BIN}"

# ── restart ──────────────────────────────────────────────────────────
if ! $NO_RESTART; then
    echo "Waiting for systemd auto-restart..."
    remote "
        for i in 1 2 3 4 5; do
            sleep 1
            if pgrep -x seeki >/dev/null 2>&1; then
                echo 'Service is running (pid '\$(pgrep -x seeki)')';
                exit 0
            fi
        done
        echo 'WARNING: seeki did not restart within 5s'
    "
fi

echo ""
echo "=== Deployment complete ==="
