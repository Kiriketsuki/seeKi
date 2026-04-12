-- Seed data for E2E tests.
-- Mirrors the MEC-Miki schema with enough rows to exercise
-- pagination (>50), sorting, filtering, NULLs, and booleans.

CREATE TABLE IF NOT EXISTS vehicle_logs (
    id            SERIAL PRIMARY KEY,
    vehicle_id    VARCHAR(20)  NOT NULL,
    event_type    VARCHAR(20)  NOT NULL,
    speed_kmh     NUMERIC(8,2),
    is_active     BOOLEAN      NOT NULL DEFAULT true,
    latitude      NUMERIC(10,6),
    longitude     NUMERIC(10,6),
    notes         TEXT,
    logged_at     TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS soc_readings (
    id            SERIAL PRIMARY KEY,
    vehicle_id    VARCHAR(20)  NOT NULL,
    soc_percent   NUMERIC(5,2) NOT NULL,
    voltage_v     NUMERIC(6,3),
    is_charging   BOOLEAN      NOT NULL DEFAULT false,
    read_at       TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- Generate 200 vehicle_logs rows
INSERT INTO vehicle_logs (vehicle_id, event_type, speed_kmh, is_active, latitude, longitude, notes, logged_at)
SELECT
    'VH-' || LPAD((i % 5 + 1)::TEXT, 3, '0'),
    CASE i % 5
        WHEN 0 THEN 'START'
        WHEN 1 THEN 'STOP'
        WHEN 2 THEN 'MOVING'
        WHEN 3 THEN 'IDLE'
        WHEN 4 THEN 'ALERT'
    END,
    CASE WHEN i % 7 = 0 THEN NULL ELSE ROUND((random() * 80)::NUMERIC, 2) END,
    i % 3 != 0,
    CASE WHEN i % 7 = 0 THEN NULL ELSE ROUND((52.0 + random() * 0.1)::NUMERIC, 6) END,
    CASE WHEN i % 7 = 0 THEN NULL ELSE ROUND((-1.5 + random() * 0.1)::NUMERIC, 6) END,
    CASE WHEN i % 10 = 0 THEN NULL ELSE 'Log entry #' || i END,
    now() - (i || ' minutes')::INTERVAL
FROM generate_series(1, 200) AS s(i);

-- Generate 80 soc_readings rows
INSERT INTO soc_readings (vehicle_id, soc_percent, voltage_v, is_charging, read_at)
SELECT
    'VH-' || LPAD((i % 5 + 1)::TEXT, 3, '0'),
    ROUND((20 + random() * 80)::NUMERIC, 2),
    CASE WHEN i % 4 = 0 THEN NULL ELSE ROUND((11.5 + random() * 1.5)::NUMERIC, 3) END,
    i % 4 = 0,
    now() - (i * 5 || ' minutes')::INTERVAL
FROM generate_series(1, 80) AS s(i);
