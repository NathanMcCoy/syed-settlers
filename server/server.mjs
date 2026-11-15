// Syed Settlers' server: serves site/ and answers /healthz. Node's own modules only. Games will run
// here too (every game, even one person against bots, so the server holds the dice and the cards),
// using the same engine as the page: site/js/settlers.wasm.
//   node server/server.mjs              listens on PORT (default 3000)
//   node server/server.mjs --port 8091  for trying it on this PC (npm run serve)
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { HEADERS } from './headers.mjs';
import { makeGames } from './games.mjs';
import { loadEngine } from '../site/js/engine.js';

const SITE = fileURLToPath(new URL('../site/', import.meta.url));
const TYPES = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.png': 'image/png',
  '.jpg': 'image/jpeg', '.json': 'application/json', '.txt': 'text/plain; charset=utf-8', '.ico': 'image/x-icon',
};

function send(res, status, type, body, extra = {}) {
  res.writeHead(status, { ...HEADERS, 'Content-Type': type, ...extra });
  res.end(body);
}

// A file from site/. Paths can't climb out of it (".." is resolved first and checked). Each file
// gets an ETag from its size and time, so a browser checking for a newer copy gets a quick 304.
async function file(req, res, path) {
  let full = normalize(join(SITE, path));
  if (!(full + sep).startsWith(SITE)) return send(res, 403, 'text/plain', 'Forbidden');   // SITE ends with a separator
  try {
    let info = await stat(full);
    if (info.isDirectory()) { full = join(full, 'index.html'); info = await stat(full); }
    const type = TYPES[extname(full)] || 'application/octet-stream';
    const tag = `W/"${info.size.toString(36)}-${Math.floor(info.mtimeMs).toString(36)}"`;
    if (req.headers['if-none-match'] === tag) return send(res, 304, type, undefined, { ETag: tag });
    const body = req.method === 'HEAD' ? undefined : await readFile(full);
    send(res, 200, type, body, { ETag: tag, 'Content-Length': info.size });
  } catch {
    send(res, 404, 'text/plain; charset=utf-8', 'Not found');
  }
}

const json = (res, status, data) => send(res, status, 'application/json', JSON.stringify(data), { 'Cache-Control': 'no-store' });

// A small JSON body (moves and new games are tiny; anything big is refused)
function body(req) {
  return new Promise((ok, fail) => {
    let size = 0;
    const chunks = [];
    req.on('data', c => {
      size += c.length;
      if (size <= 8192) chunks.push(c);
    });
    // Too big: refuse once it's all arrived (stopping mid-way would cut the connection before the
    // answer); what's beyond the limit is never kept
    req.on('end', () => {
      if (size > 8192) return fail(Object.assign(new Error('too much data'), { status: 413 }));
      try { ok(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}')); } catch { fail(new Error('not JSON')); }
    });
    req.on('error', fail);
  });
}

// The game API (server/games.mjs does the work):
//   POST /api/games                 {seats: [{kind: 'person'|'bot', name?}], settings?}: a new game
//   GET  /api/games/ID?k=KEY        the board and the game as that seat sees it (no key: watching)
//   POST /api/games/ID/move         {k, move}: a move as text
//   GET  /api/games/ID/live?k=KEY   live updates (Server-Sent Events)
async function api(req, res, path, games) {
  const parts = path.split('/').filter(Boolean);   // ['api', 'games', ID, 'move']
  const key = new URLSearchParams(req.url.split('?')[1] || '').get('k');
  try {
    if (parts[1] !== 'games' || parts.length > 4) return json(res, 404, { error: 'not found' });
    if (parts.length === 2 && req.method === 'POST') return json(res, 201, games.create(await body(req)));
    const id = parts[2];
    if (parts.length === 3 && req.method === 'GET') return json(res, 200, games.open(id, key));
    if (parts[3] === 'move' && req.method === 'POST') {
      const { k, move } = await body(req);
      games.act(id, k, move);
      return json(res, 200, { ok: true });
    }
    if (parts[3] === 'live' && req.method === 'GET') return games.listen(id, key, res);
    return json(res, 405, { error: 'method not allowed' });
  } catch (e) {
    return json(res, e.status || 400, { error: e.message });
  }
}

export function handle(req, res, games) {
  let path;
  // The path straight from the request line, without its ?query (not new URL(): it reads a path
  // starting with // as another host). file() keeps it inside site/.
  try { path = decodeURIComponent(req.url.split('?')[0]); } catch { return send(res, 400, 'text/plain', 'Bad request'); }
  if (path === '/healthz') return send(res, 200, 'application/json', JSON.stringify({ ok: true }), { 'Cache-Control': 'no-store' });
  if (path.startsWith('/api/') && games) return api(req, res, path, games);
  if (req.method !== 'GET' && req.method !== 'HEAD') return send(res, 405, 'text/plain', 'Method not allowed', { Allow: 'GET, HEAD' });
  return file(req, res, path);
}

// Starts listening; port 0 picks a free one (the tests do). Resolves with the http.Server.
// botDelay: milliseconds between bot moves (the tests use 0).
export async function start({ port = 3000, host = '0.0.0.0', botDelay = 700 } = {}) {
  const engine = await loadEngine(await readFile(join(SITE, 'js', 'settlers.wasm')));
  const games = makeGames(engine, { botDelay });
  const server = createServer((req, res) => {
    Promise.resolve(handle(req, res, games)).catch(() => { if (!res.headersSent) send(res, 500, 'text/plain', 'Something went wrong'); });
  });
  server.on('close', () => games.stop());
  server.games = games;
  return new Promise(ok => server.listen(port, host, () => ok(server)));
}

// Run directly: listen, and stop cleanly when the container is stopped
if (process.argv[1] && fileURLToPath(import.meta.url) === normalize(process.argv[1])) {
  const flag = process.argv.indexOf('--port');
  const port = flag > 0 ? Number(process.argv[flag + 1]) : Number(process.env.PORT) || 3000;
  const host = flag > 0 ? '127.0.0.1' : '0.0.0.0';   // on this PC, only this PC; in the container, its own network
  const server = await start({ port, host });
  console.log(`Syed Settlers on http://${host === '0.0.0.0' ? 'localhost' : host}:${server.address().port}/`);
  const stop = () => { server.close(() => process.exit(0)); setTimeout(() => process.exit(0), 5000).unref(); };
  process.on('SIGTERM', stop);
  process.on('SIGINT', stop);
}
