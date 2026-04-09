# Default recipe
default:
    @just --list

# Start backend + frontend dev servers
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run &
    CARGO_PID=$!
    # Wait briefly and verify the backend started
    sleep 2
    if ! kill -0 $CARGO_PID 2>/dev/null; then
        echo "ERROR: Backend failed to start. Check cargo output above." >&2
        exit 1
    fi
    cd frontend && npm run dev
    kill $CARGO_PID 2>/dev/null || true

# Start frontend only with mock data (no backend needed)
dev-mock:
    cd frontend && VITE_MOCK=true npm run dev

# Build frontend then release binary
build:
    cd frontend && VITE_MOCK=false npm run build
    cargo build --release
