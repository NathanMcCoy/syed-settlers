// Builds the Rust engine (engine/src) to WebAssembly as site/js/settlers.wasm, for the server and
// the page. Run it after changing anything in engine/src (not src/bin): tests/engine.test.mjs fails
// if the .wasm wasn't built from the source as it is now.
//   npm run build
// Needs Rust with its WebAssembly target: rustup target add wasm32-unknown-unknown
import { spawnSync } from 'node:child_process';
import { copyFileSync, statSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = fileURLToPath(new URL('../engine/', import.meta.url));
const cargo = ['cargo', join(homedir(), '.cargo', 'bin', process.platform === 'win32' ? 'cargo.exe' : 'cargo')]
  .find(c => spawnSync(c, ['--version'], { encoding: 'utf8' }).status === 0);
if (!cargo) throw new Error('Rust (cargo) is not installed');
// simd128: WebAssembly's vector instructions, ready for the bots' arithmetic. Every current browser
// has them (Safari since iOS 16.4), and so does Node.
const built = spawnSync(cargo, ['build', '--release', '--lib', '--target', 'wasm32-unknown-unknown'],
  { cwd: dir, stdio: 'inherit', env: { ...process.env, RUSTFLAGS: '-C target-feature=+simd128' } });
if (built.status !== 0) process.exit(built.status ?? 1);
const to = fileURLToPath(new URL('../site/js/settlers.wasm', import.meta.url));
copyFileSync(join(dir, 'target', 'wasm32-unknown-unknown', 'release', 'settlers.wasm'), to);
console.log(`Wrote site/js/settlers.wasm (${(statSync(to).size / 1024).toFixed(0)} KB)`);
