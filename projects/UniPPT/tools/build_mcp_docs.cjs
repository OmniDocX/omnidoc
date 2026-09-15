'use strict';
// Deterministic local documentation. No credentials, network or model calls.
const fs=require('node:fs'),path=require('node:path');
const root=path.resolve(__dirname,'..'),{TOOLS}=require('./unippt_mcp.cjs');
fs.writeFileSync(path.join(root,'web/mcp-tools.json'),JSON.stringify(TOOLS,null,2)+'\n');
const markdown=fs.readFileSync(path.join(root,'docs/MCP_PROTOCOL.md'),'utf8');
require('./build_usage_docs.cjs').build({root,tools:TOOLS,markdown});
