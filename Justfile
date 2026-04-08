# Default recipe
default:
    @just --list

# Start backend + frontend dev servers
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run &
    CARGO_PID=$!
    cd frontend && npm run dev
    kill $CARGO_PID 2>/dev/null || true

# Start frontend only with mock data (no backend needed)
dev-mock:
    cd frontend && VITE_MOCK=true npm run dev

# Build frontend then release binary
build:
    cd frontend && VITE_MOCK=false npm run build
    cargo build --release
