# Deploy Plan: SeeKi on 192.168.1.29 → Base-21 Postgres

**Status:** PAUSED — blocked by [#57](https://github.com/Kiriketsuki/seeKi/issues/57) (schema
selection). Resume once that ships.

**Written:** 2026-04-13.

## Target topology

```
dev laptop ──scp──▶  sg-server-user@192.168.1.29 (Ubuntu)
                         │
                         │ SeeKi (systemd --system, 0.0.0.0:3141)
                         │ ├── reads seeki.toml
                         │ └── opens SSH tunnel to Base-21
                         ▼
                    mec-usr@203.127.39.21 (Base-21)
                         │
                         └── postgis/postgis:18-3.6 container
                             0.0.0.0:5433 → postgres / postgres / autoconnect_db
```

SeeKi already has built-in SSH tunnel support (`crate::ssh::SshTunnel`,
`[ssh]` config section — see `src/config.rs:231-246`). No separate `ssh -L` / autossh unit
needed.

## Discovered facts

- **.29 SSH:** `sg-server-user@192.168.1.29`, password in `~/workdev/Aurrigo/.envrc` as
  `SYNC_DOCS_PASSWORD` (also used by `AutoConnect-DOC-Hadi/deploy.sh:14-18`).
- **Base-21 SSH:** `mec-usr@203.127.39.21`, key-based auth via
  `~/workdev/keys/singtel_private_key` (passphrase-protected).
- **Base-21 Postgres:** `miki-postgres` container, listens on Base-21:5433.
  Creds from `docker inspect miki-postgres`:
  - `POSTGRES_USER=postgres`
  - `POSTGRES_PASSWORD=postgres`
  - `POSTGRES_DB=autoconnect_db`
- **Schemas in `autoconnect_db`:** `autoconnect_db` (36 tables — the real data),
  `migration_stage` (7 tables), `public` (2 tables — `web_bag*`, irrelevant).
  The PostGIS triplet (`geography_columns`, `geometry_columns`, `spatial_ref_sys`) lives
  under `autoconnect_db` and should be excluded as "system".
- **Network:** Base-21 Postgres at `203.127.39.21:5433` is **not** reachable from the dev
  laptop LAN (confirmed `/dev/tcp` → `Network is unreachable`). Whether .29 has direct
  reach is unverified — irrelevant, since we're going via SSH tunnel anyway to avoid
  exposing Postgres further and to reuse SeeKi's built-in mechanism.

## Steps to execute (once #57 is merged)

### 1. Copy SSH key to .29

```bash
source ~/workdev/Aurrigo/.envrc  # SYNC_DOCS_PASSWORD
sshpass -p "$SYNC_DOCS_PASSWORD" scp -o StrictHostKeyChecking=no \
  ~/workdev/keys/singtel_private_key \
  sg-server-user@192.168.1.29:/home/sg-server-user/.seeki/singtel_private_key

sshpass -p "$SYNC_DOCS_PASSWORD" ssh sg-server-user@192.168.1.29 \
  "chmod 600 /home/sg-server-user/.seeki/singtel_private_key"
```

Key is passphrase-protected. Put the passphrase in `.seeki.secrets` on .29 (see step 3).

### 2. Build release binary

```bash
cd ~/dev/Personal/seeKi
cargo build --release
# target/release/seeki — embeds frontend via rust-embed
```

.29 is x86_64 Ubuntu — laptop is x86_64 Arch, so a native release build should run on .29
as-is. If glibc mismatch errors appear, fall back to `--target x86_64-unknown-linux-musl`
(requires `cross` or `musl-tools`).

### 3. Copy binary + config to .29

```bash
sshpass -p "$SYNC_DOCS_PASSWORD" ssh sg-server-user@192.168.1.29 \
  "mkdir -p /home/sg-server-user/.seeki"

sshpass -p "$SYNC_DOCS_PASSWORD" scp target/release/seeki \
  sg-server-user@192.168.1.29:/home/sg-server-user/.seeki/seeki
```

Write `~/.seeki/seeki.toml` on .29:

```toml
[server]
host = "0.0.0.0"
port = 3141

[database]
kind = "postgres"
# Host is Base-21 as seen FROM Base-21 — the SSH tunnel terminates on Base-21 itself,
# so localhost:5433 reaches the postgres container.
url = "postgres://postgres:postgres@127.0.0.1:5433/autoconnect_db"
max_connections = 5
# schemas = ["autoconnect_db", "migration_stage"]   # enable once #57 ships

[ssh]
host = "203.127.39.21"
port = 22
username = "mec-usr"
auth_method = "key"
key_path = "/home/sg-server-user/.seeki/singtel_private_key"

[branding]
title = "ACT-Seeki"

[tables]
# All non-system tables across the two operational schemas.
# Explicit include list because PostGIS internal tables (geography_columns,
# geometry_columns, spatial_ref_sys) must stay hidden.
include = [
  "aurrigo_vehicles", "aurrigo_vehicles_log", "background_jobs",
  "belt_sections", "belts", "bha", "events", "fault_attributes",
  "fault_definitions", "faults", "flights", "jcpl", "locations",
  "missions", "popups", "racetracks", "roller_decks", "routes",
  "sectors", "stand_areas", "stands", "tasks", "terminal", "uld",
  "v_fault_lookup", "v_mission_uld_lookup", "v_uld_audit",
  "v_vehicle_current", "vehicle_availability", "vehicle_ctrl_codes",
  "vehicle_power", "vehicle_subsystems", "vehicle_trip_data",
  # migration_stage/*
  "aurrigo_vehicle_ctrl_from", "aurrigo_vehicle_ctrl_to",
  # Note: name collisions (aurrigo_vehicles, vehicle_availability,
  # vehicle_power, vehicle_subsystems appear in both schemas) — #57
  # should surface these as schema.table in the sidebar.
]
```

And `~/.seeki/.seeki.secrets`:

```toml
[ssh]
key_passphrase = "<singtel_private_key passphrase>"
```

`chmod 600` both files.

### 4. Install systemd unit (system scope)

`/etc/systemd/system/seeki.service`:

```ini
[Unit]
Description=SeeKi (ACT-Seeki) — Base-21 DB viewer
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=sg-server-user
Group=sg-server-user
WorkingDirectory=/home/sg-server-user/.seeki
ExecStart=/home/sg-server-user/.seeki/seeki
Restart=on-failure
RestartSec=5s

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/sg-server-user/.seeki
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now seeki
sudo systemctl status seeki
```

### 5. Verify

- `curl http://192.168.1.29:3141/api/tables` from the dev laptop.
- Browser: `http://192.168.1.29:3141/` — sidebar should list the included tables.
- `journalctl -u seeki -f` on .29 to watch for SSH / DB errors.

## Open questions for resume

- [ ] Does .29 actually need to be `system` scope, or does `sg-server-user`'s user scope
      suffice? (System scope was the user's preference — confirmed.)
- [ ] Firewall on .29 — is 3141 already open on the LAN, or does UFW need a rule?
- [ ] Should the binary live at `/usr/local/bin/seeki` instead of under `~sg-server-user/.seeki/`?
      Cleaner, but requires root scp. Current plan keeps everything under the service
      user's home for simplicity.
- [ ] Log rotation — `journalctl` handles it, so nothing extra needed.

## Rollback

```bash
sudo systemctl disable --now seeki
sudo rm /etc/systemd/system/seeki.service
sudo systemctl daemon-reload
rm -rf /home/sg-server-user/.seeki
```
