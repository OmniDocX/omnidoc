'use strict';
// Small real-stdio integration client for regression tests and local diagnostics.
const {spawn} = require('node:child_process');
const path = require('node:path');
function openClient() {
  const child = spawn(process.execPath, [path.join(__dirname, 'unippt_mcp.cjs')], {stdio: ['pipe', 'pipe', 'inherit'], windowsHide: true});
  const pending = new Map(); let ordinal = 0, buffer = '';
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', chunk => {
    buffer += chunk; let newline;
    while ((newline = buffer.indexOf('\n')) >= 0) {
      const message = JSON.parse(buffer.slice(0, newline)); buffer = buffer.slice(newline + 1);
      const waiter = pending.get(message.id); if (!waiter) continue;
      pending.delete(message.id); clearTimeout(waiter.timer);
      message.error ? waiter.reject(new Error(message.error.message)) : waiter.resolve(message.result);
    }
  });
  child.on('exit', () => { for (const waiter of pending.values()) {clearTimeout(waiter.timer); waiter.reject(new Error('MCP process exited'));} pending.clear(); });
  const request = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++ordinal, timer = setTimeout(() => {pending.delete(id); reject(new Error('MCP client timeout'));}, 65000);
    pending.set(id, {resolve, reject, timer}); child.stdin.write(JSON.stringify({jsonrpc: '2.0', id, method, params}) + '\n');
  });
  return {request, call: (name, args = {}) => request('tools/call', {name, arguments: args}), close: () => child.stdin.end()};
}
if (require.main === module) {
  const client = openClient();
  (async () => {
    const init = await client.request('initialize', {protocolVersion: '2025-06-18', capabilities: {}, clientInfo: {name: 'unippt-smoke', version: '1'}});
    const result = await client.call(process.argv[2] || 'unippt_list_sessions', JSON.parse(process.argv[3] || '{}'));
    console.log(JSON.stringify({server: init.serverInfo, ...result}));
    if (result.isError) process.exitCode = 1;
  })().catch(e => {console.error(e.message); process.exitCode = 1;}).finally(() => client.close());
}
module.exports = {openClient};
