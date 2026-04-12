/**
 * Playwright global teardown:
 * 1. Kill the SeeKi server process
 * 2. Remove the copied seeki.toml
 * 3. Clean up PID file
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(__dirname, '../..');
const CONFIG_DST = path.join(PROJECT_ROOT, 'seeki.toml');
const PID_FILE = path.join(PROJECT_ROOT, '.e2e-server.pid');

async function globalTeardown(): Promise<void> {
  // 1. Kill server
  if (fs.existsSync(PID_FILE)) {
    const pid = parseInt(fs.readFileSync(PID_FILE, 'utf-8').trim(), 10);
    if (!isNaN(pid)) {
      try {
        process.kill(pid, 'SIGTERM');
        console.log(`[global-teardown] Sent SIGTERM to server (PID ${pid})`);
        // Give it a moment to shut down gracefully
        await new Promise((r) => setTimeout(r, 2000));
        try {
          process.kill(pid, 0); // Check if still alive
          process.kill(pid, 'SIGKILL');
          console.log(`[global-teardown] Sent SIGKILL to server (PID ${pid})`);
        } catch {
          // Already exited
        }
      } catch {
        // Process already gone
      }
    }
    fs.unlinkSync(PID_FILE);
  }

  // 2. Remove copied config
  if (fs.existsSync(CONFIG_DST)) {
    fs.unlinkSync(CONFIG_DST);
    console.log(`[global-teardown] Removed ${CONFIG_DST}`);
  }

  console.log('[global-teardown] Teardown complete');
}

export default globalTeardown;
