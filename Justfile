# Default recipe
default:
    @just --list

# Start backend + frontend dev servers
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run &
    CARGO_PID=$!
    trap "kill $CARGO_PID 2>/dev/null" EXIT INT TERM
    # Wait briefly and verify the backend started
    sleep 2
    if ! kill -0 $CARGO_PID 2>/dev/null; then
        echo "ERROR: Backend failed to start. Check cargo output above." >&2
        exit 1
    fi
    cd frontend && npm run dev

# Start frontend only with mock data (no backend needed)
dev-mock:
    cd frontend && VITE_MOCK=true npm run dev

# Build frontend then release binary
build:
    cd frontend && VITE_MOCK=false npm run build
    cargo build --release

# Run E2E tests (builds release binary, starts server, runs Playwright).
# WARNING: global-setup.ts applies tests/fixtures/seed.sql, which executes
# `TRUNCATE TABLE vehicle_logs, soc_readings RESTART IDENTITY` before inserting
# synthetic test data. Always run against a DEDICATED test DB, not production.
# To skip seeding (pre-populated DB, or to protect existing data):
#   SEEKI_SKIP_SEED=1 just test-e2e
test-e2e:
    cd frontend && VITE_MOCK=false npm run build
    cargo build --release
    cd frontend && SEEKI_SKIP_BUILD=1 npx playwright test

# Run E2E tests across all browsers (Chrome + Firefox + WebKit).
# Same TRUNCATE warning as test-e2e applies.
test-e2e-all:
    cd frontend && VITE_MOCK=false npm run build
    cargo build --release
    cd frontend && SEEKI_SKIP_BUILD=1 SEEKI_ALL_BROWSERS=1 npx playwright test

# Run E2E tests in Playwright UI mode (interactive debugging)
test-e2e-ui:
    cd frontend && VITE_MOCK=false npm run build
    cargo build --release
    cd frontend && SEEKI_SKIP_BUILD=1 npx playwright test --ui
