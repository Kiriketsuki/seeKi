/**
 * Playwright global setup:
 * 1. Copy seeki.toml.test → seeki.toml in project root (for "normal" mode tests)
 * 2. Build the Rust binary (skip if SEEKI_SKIP_BUILD=1)
 * 3. Start the SeeKi server
 * 4. Wait for /api/status to return healthy
 */
import { execSync, spawn, type ChildProcess } from 'child_process';
import fs from 'fs';
import path from 'path';
import net from 'net';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(__dirname, '../..');
const CONFIG_SRC = path.join(PROJECT_ROOT, 'seeki.toml.test');
const CONFIG_DST = path.join(PROJECT_ROOT, 'seeki.toml');
const CONFIG_BACKUP = path.join(PROJECT_ROOT, 'seeki.toml.user-backup');
const BASE_URL = 'http://127.0.0.1:3141';
const HEALTH_TIMEOUT_MS = 30_000;
const HEALTH_POLL_MS = 500;
const PORT = 3141;

let serverProcess: ChildProcess | undefined;

function isPortInUse(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const tester = net.createServer()
      .once('error', () => resolve(true))
      .once('listening', () => {
        tester.close(() => resolve(false));
      })
      .listen(port, '127.0.0.1');
  });
}

async function waitForHealthy(): Promise<void> {
  const deadline = Date.now() + HEALTH_TIMEOUT_MS;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${BASE_URL}/api/status`);
      if (res.ok) {
        const body = await res.json();
        console.log(`[global-setup] Server healthy: mode=${(body as { mode: string }).mode}`);
        return;
      }
    } catch {
      // Server not ready yet
    }
    await new Promise((r) => setTimeout(r, HEALTH_POLL_MS));
  }
  throw new Error(`[global-setup] Server did not become healthy within ${HEALTH_TIMEOUT_MS}ms`);
}

async function globalSetup(): Promise<void> {
  // 1. Check port availability
  if (await isPortInUse(PORT)) {
    throw new Error(
      `[global-setup] Port ${PORT} is already in use. Stop any running SeeKi instance first.`
    );
  }

  // 2. Copy test config into place
  if (!fs.existsSync(CONFIG_SRC)) {
    throw new Error(
      `[global-setup] Missing ${CONFIG_SRC}. Create it from seeki.toml.example with test DB credentials.`
    );
  }

  // Load .env.test if it exists (for DB credentials)
  const envTestPath = path.join(PROJECT_ROOT, '.env.test');
  if (fs.existsSync(envTestPath)) {
    const envContent = fs.readFileSync(envTestPath, 'utf-8');
    for (const line of envContent.split('\n')) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) continue;
      const eqIdx = trimmed.indexOf('=');
      if (eqIdx === -1) continue;
      const key = trimmed.slice(0, eqIdx).trim();
      const value = trimmed.slice(eqIdx + 1).trim().replace(/^["']|["']$/g, '');
      process.env[key] = value;
    }
  }

  // Expand env vars in the test config
  let configContent = fs.readFileSync(CONFIG_SRC, 'utf-8');
  const emptyVars: string[] = [];
  configContent = configContent.replace(/\$\{(\w+)\}/g, (_, varName) => {
    const val = process.env[varName] ?? '';
    if (!val) emptyVars.push(varName);
    return val;
  });
  if (emptyVars.length > 0) {
    console.warn(`[global-setup] WARNING: empty env vars in config: ${emptyVars.join(', ')}. Check .env.test.`);
  }

  // Back up existing user config before overwriting
  if (fs.existsSync(CONFIG_DST)) {
    fs.copyFileSync(CONFIG_DST, CONFIG_BACKUP);
    console.log(`[global-setup] Backed up existing ${CONFIG_DST} → ${CONFIG_BACKUP}`);
  }

  fs.writeFileSync(CONFIG_DST, configContent);
  console.log(`[global-setup] Copied ${CONFIG_SRC} → ${CONFIG_DST}`);

  // 3. Build (unless skipped)
  if (process.env.SEEKI_SKIP_BUILD !== '1') {
    console.log('[global-setup] Building release binary...');
    execSync('cargo build --release', {
      cwd: PROJECT_ROOT,
      stdio: 'inherit',
      timeout: 300_000,
    });
  }

  // 4. Start server
  const binaryPath = path.join(PROJECT_ROOT, 'target/release/seeki');
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`[global-setup] Binary not found at ${binaryPath}. Build may have failed.`);
  }

  console.log('[global-setup] Starting SeeKi server...');
  serverProcess = spawn(binaryPath, [], {
    cwd: PROJECT_ROOT,
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, RUST_LOG: 'seeki=info' },
  });

  serverProcess.stdout?.on('data', (data: Buffer) => {
    process.stdout.write(`[seeki] ${data.toString()}`);
  });
  serverProcess.stderr?.on('data', (data: Buffer) => {
    process.stderr.write(`[seeki] ${data.toString()}`);
  });

  serverProcess.on('exit', (code) => {
    if (code !== null && code !== 0) {
      console.error(`[global-setup] Server exited with code ${code}`);
    }
  });

  // Store PID for teardown
  if (serverProcess.pid) {
    const pidFile = path.join(PROJECT_ROOT, '.e2e-server.pid');
    fs.writeFileSync(pidFile, String(serverProcess.pid));
  }

  // 5. Wait for healthy
  await waitForHealthy();
  console.log('[global-setup] Setup complete');
}

export default globalSetup;
