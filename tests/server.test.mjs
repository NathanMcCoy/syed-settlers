// The server, run for real on a free port: what visitors get.
import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import { start } from '../server/server.mjs';

const server = await start({ port: 0, host: '127.0.0.1' });
const base = `http://127.0.0.1:${server.address().port}`;
after(() => server.close());

test('/healthz answers', async () => {
  const res = await fetch(`${base}/healthz`);
  assert.equal(res.status, 200);
  assert.deepEqual(await res.json(), { ok: true });
  assert.equal(res.headers.get('cache-control'), 'no-store');
});

test('the page and the engine are served with the right types and headers', async () => {
  for (const [path, type] of [['/', 'text/html'], ['/js/viewer.js', 'text/javascript'], ['/js/settlers.wasm', 'application/wasm'], ['/css/app.css', 'text/css']]) {
    const res = await fetch(base + path);
    assert.equal(res.status, 200, path);
    assert.ok(res.headers.get('content-type').startsWith(type), path);
    assert.equal(res.headers.get('cache-control'), 'no-cache, no-transform');
    assert.match(res.headers.get('content-security-policy'), /script-src 'self' 'wasm-unsafe-eval'/);
    assert.equal(res.headers.get('x-content-type-options'), 'nosniff');
  }
});

test('an unchanged file answers 304', async () => {
  const first = await fetch(`${base}/js/engine.js`);
  const again = await fetch(`${base}/js/engine.js`, { headers: { 'If-None-Match': first.headers.get('etag') } });
  assert.equal(again.status, 304);
});

test('paths cannot climb out of site/', async () => {
  for (const path of ['/../server/server.mjs', '/%2e%2e/server/server.mjs', '/..%2fpackage.json', '//etc/passwd']) {
    const res = await fetch(base + path);
    assert.ok([403, 404].includes(res.status), `${path}: ${res.status}`);
  }
  assert.equal((await fetch(`${base}/%zz`)).status, 400);
  assert.equal((await fetch(`${base}/`, { method: 'POST' })).status, 405);
});
