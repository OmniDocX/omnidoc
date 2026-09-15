'use strict';
// Loopback-only transport between opted-in browser documents and local MCP clients.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const ROOT = path.resolve(__dirname, '..');
const PORT = Number(process.env.UNIPPT_MCP_PORT || 8142);
const ORIGINS = new Set(['http://127.0.0.1:8141', 'http://localhost:8141']);
const METHODS = new Set(['prepare', 'inspect', 'get_slide', 'get_objects', 'get_edit_schema', 'get_module_schema', 'get_animation_schema', 'apply_patch', 'apply_scene', 'apply_timeline', 'undo', 'preview', 'source_image', 'export_pptx', 'get_capabilities', 'validate', 'export']);
const WRITES = new Set(['apply_patch', 'apply_scene', 'apply_timeline', 'undo']);
// Chrome may throttle a background tab's timer to roughly one minute. Keep a
// bounded heartbeat grace period; explicit disconnect/document changes revoke
// immediately. Agent requests never renew a browser heartbeat.
const SESSION_TTL = 300000, COMMAND_TTL = 60000, RETAIN_MS = 600000;
function agentToken() {
  const dir = path.join(ROOT, '.unippt-mcp');
  fs.mkdirSync(dir, {recursive: true});
  const file = path.join(dir, 'token');
  try { fs.writeFileSync(file, crypto.randomBytes(32).toString('hex'), {flag: 'wx', mode: 0o600}); }
  catch (error) { if (error.code !== 'EEXIST') throw error; }
  const token = fs.readFileSync(file, 'utf8').trim();
  if (!/^[a-f0-9]{64}$/.test(token)) throw new Error('Invalid local MCP token');
  return token;
}
function safeEqual(a, b) {
  const x = Buffer.from(String(a || '')), y = Buffer.from(String(b || ''));
  return x.length === y.length && crypto.timingSafeEqual(x, y);
}
function createBroker({token = agentToken(), port = PORT, now = Date.now} = {}) {
  const sessions = new Map(), commands = new Map();
  const fail = (message, status = 400) => { throw Object.assign(new Error(message), {status}); };
  function sweep() {
    const time = now();
    for (const [id, s] of sessions) if (time - s.seen > SESSION_TTL) {
      const executing = [...commands.values()].some(c => c.sessionId === id && ['claimed', 'uncertain'].includes(c.state) && time <= c.deadline);
      if (!executing) sessions.delete(id);
    }
    for (const [id, c] of commands) {
      if (c.state === 'pending' && (time > c.deadline || !sessions.has(c.sessionId))) c.state = 'expired';
      if (time - c.created > RETAIN_MS) commands.delete(id);
    }
  }
  function browserSession(body) {
    const s = sessions.get(body.sessionId);
    if (!s || !safeEqual(body.secret, s.secret)) fail('Session disconnected or expired', 410);
    s.seen = now(); return s;
  }
  const server = http.createServer(async (req, res) => {
    const send = (status, body) => { res.writeHead(status, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'}); res.end(JSON.stringify(body)); };
    try {
      const address = req.socket.remoteAddress;
      if (!['127.0.0.1', '::1', '::ffff:127.0.0.1'].includes(address) || req.headers.host !== `127.0.0.1:${server.address().port}`) fail('Loopback host required', 403);
      const url = new URL(req.url, `http://127.0.0.1:${port}`), browser = url.pathname.startsWith('/browser/');
      if (browser) {
        if (!ORIGINS.has(req.headers.origin)) fail('UniPPT origin required', 403);
        res.setHeader('Access-Control-Allow-Origin', req.headers.origin);
        res.setHeader('Vary', 'Origin');
        if (req.method === 'OPTIONS') {
          res.setHeader('Access-Control-Allow-Methods', 'POST');
          res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
          res.setHeader('Access-Control-Max-Age', '600');
          res.writeHead(204); res.end(); return;
        }
      } else {
        if (req.headers.origin || !safeEqual(req.headers.authorization, `Bearer ${token}`)) fail('Local MCP authentication required', 403);
      }
      if (req.method !== 'POST' || !String(req.headers['content-type']).startsWith('application/json')) fail('JSON POST required', 405);
      const parts = []; let size = 0;
      for await (const chunk of req) { size += chunk.length; if (size > 32 * 1024 * 1024) fail('Payload too large', 413); parts.push(chunk); }
      const body = JSON.parse(Buffer.concat(parts).toString('utf8') || '{}'); sweep();
      let result;
      switch (url.pathname) {
        case '/browser/connect': {
          if (sessions.size >= 16) fail('Too many browser sessions', 429);
          const sessionId = crypto.randomUUID(), secret = crypto.randomBytes(24).toString('hex');
          sessions.set(sessionId, {sessionId, secret, title: String(body.title || '').slice(0, 300), writeAccess: body.writeAccess === true, revision: 0, seen: now()});
          result = {sessionId, secret}; break;
        }
        case '/browser/poll': {
          const s = browserSession(body);
          s.title = String(body.title || s.title).slice(0, 300); s.revision = body.revision;
          s.slideCount = body.slideCount; s.activeSlideId = body.activeSlideId;
          const command = [...commands.values()].find(c => c.sessionId === s.sessionId && c.state === 'pending');
          if (command) command.state = 'claimed';
          result = {command: command ? {requestId: command.requestId, method: command.method, args: command.args, deadline: command.deadline} : null}; break;
        }
        case '/browser/result': {
          const s = browserSession(body), c = commands.get(body.requestId);
          if (!c || c.sessionId !== s.sessionId || !['claimed', 'uncertain'].includes(c.state)) fail('Command is no longer owned by this session', 409);
          c.state = 'completed'; c.result = body.result; c.error = body.error; c.completed = now(); result = {ok: true}; break;
        }
        case '/browser/disconnect': {
          const s = browserSession(body); sessions.delete(s.sessionId); sweep(); result = {ok: true}; break;
        }
        case '/agent/health': result = {service: 'unippt-mcp', version: 1}; break;
        case '/agent/sessions': result = {sessions: [...sessions.values()].map(({secret, seen, ...s}) => ({...s, lastSeenMs: now() - seen}))}; break;
        case '/agent/submit': {
          const s = sessions.get(body.sessionId);
          if (!s) fail('No connected document. Click MCP 连接 in UniPPT.', 410);
          if (!METHODS.has(body.method)) fail('Unknown document command');
          if (WRITES.has(body.method) && !s.writeAccess) fail('Browser session is read-only', 403);
          if (!/^[\w-]{8,100}$/.test(body.requestId || '')) fail('Stable requestId (8–100 safe characters) required');
          // Compilation timing is diagnostic, not part of mutation identity.
          // The deterministic expansion must deduplicate a retried tool call.
          const identityArgs = {...body.args};
          if (body.method === 'apply_scene') delete identityArgs.compileMs;
          const fingerprint = JSON.stringify([body.sessionId, body.method, identityArgs]);
          const old = commands.get(body.requestId);
          if (old) {
            if (old.fingerprint !== fingerprint) fail('requestId was already used for different arguments', 409);
            result = old; break;
          }
          if ([...commands.values()].filter(c => ['pending', 'claimed'].includes(c.state)).length >= 32 || commands.size >= 256) fail('Command capacity reached; wait before retrying', 429);
          const c = {requestId: body.requestId, sessionId: s.sessionId, method: body.method, args: body.args || {}, fingerprint,
            state: 'pending', created: now(), deadline: now() + COMMAND_TTL};
          commands.set(c.requestId, c); result = c; break;
        }
        case '/agent/status': {
          result = commands.get(body.requestId); if (!result) fail('Unknown or expired requestId', 404); break;
        }
        case '/agent/cancel': {
          const c = commands.get(body.requestId); if (!c) fail('Unknown requestId', 404);
          if (c.state === 'pending') c.state = 'cancelled';
          else if (c.state === 'claimed') c.state = 'uncertain';
          result = c; break;
        }
        default: fail('Not found', 404);
      }
      if (result?.fingerprint) { const {fingerprint, args, ...publicResult} = result; result = publicResult; }
      send(200, result);
    } catch (error) { if (!res.headersSent) send(error.status || 400, {error: error.message}); else res.end(); }
  });
  server.requestTimeout = 15000; server.headersTimeout = 10000;
  return {server, sessions, commands, sweep};
}
if (require.main === module) {
  const {server} = createBroker();
  server.on('error', error => { console.error(error.message); process.exitCode = 1; });
  server.listen(PORT, '127.0.0.1');
}
module.exports = {ROOT, PORT, createBroker, agentToken};
