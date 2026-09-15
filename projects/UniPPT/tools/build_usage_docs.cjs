'use strict';
// Offline documentation only. It never opens a browser or invokes a model.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const {escape,page}=require('./docs_html.cjs');
function build({root,tools,markdown}){
 const read=name=>fs.readFileSync(path.join(root,name),'utf8').replace(/^\uFEFF/,'');
 const json=value=>JSON.stringify(value,null,2)+'\n',generated=[];
 const write=(name,value)=>{const target=path.join(root,name);fs.mkdirSync(path.dirname(target),{recursive:true});fs.writeFileSync(target,value);generated.push(name);};
 // Load only known schema definitions into a networkless, fileless VM.
 const context={structuredClone};vm.createContext(context);
 vm.runInContext(read('web/presentation_host.js'),context,{timeout:5000});
 const {capabilities}=require('../web/mcp_bridge.js'),allowed=capabilities().operations;
 const native=JSON.parse(JSON.stringify(context.UniPptPresentationHost.toolDefinitions.find(t=>t.name==='presentation.applyChangeSet').inputSchema));
 if(native.properties.operations.items.oneOf)native.properties.operations.items.oneOf=native.properties.operations.items.oneOf.filter(op=>allowed.includes(op.properties.op.enum[0]));
 if(native.properties.operations.items.properties?.op?.enum)native.properties.operations.items.properties.op.enum=native.properties.operations.items.properties.op.enum.filter(op=>allowed.includes(op));
 const runtime={snapshotDate:'2026-09-16',authority:'Static local schema snapshot; connected get_edit_schema/get_module_schema/get_animation_schema responses take precedence.',
  native:{schema:native,allowedOperations:allowed,notes:['Use bridge expectedRevision at the MCP boundary, not native baseRevision.','PNG imageData is an MCP insertion extension; see the protocol for limits and normalization.','No arbitrary URLs, scripts, filesystem access or identity-binding overrides.']},
  modules:require('../web/mcp_scene.js').moduleSchema(),imageRows:require('../web/mcp_image_layout.js').schema()};
 const runtimeJson=json(runtime),toolsJson=json(tools);write('web/mcp-runtime-schemas.json',runtimeJson);
 const guide=read('docs/USER_GUIDE.md'),integration=read('docs/AI_MCP_INTEGRATION.md');
 const fence=String.fromCharCode(96).repeat(3);
 const appendix=(label,value)=>'\n\n## '+label+'\n\n'+fence+'json\n'+value+fence+'\n';
 const complete=markdown+appendix('附录 A：完整本机工具 JSON Schema',toolsJson)+appendix('附录 B：原生操作、模块与图片行协议快照',runtimeJson);
 write('web/mcp-protocol.md',complete);
 const details=(label,value)=>'<details><summary>'+escape(label)+'</summary><pre tabindex="0"><code>'+escape(value)+'</code></pre></details>';
 write('web/mcp-protocol.html',page({title:'MCP 协议',description:'UniPPT 当前全部 '+tools.length+' 个本机工具、原子编辑与能力边界。',markdown,active:'protocol',appendix:details('完整本机工具 JSON Schema（'+tools.length+' 个工具）',toolsJson)+details('原生操作、模块与图片行协议快照',runtimeJson)}));
 write('web/docs/index.html',page({title:'使用指南',description:'UniPPT 本机编辑、图片转可编辑 PPT、文件保存和 MCP 使用指南。',markdown:guide,active:'guide'}));
 write('web/docs/integration.html',page({title:'本机 AI 与 MCP',description:'本机 MCP 接入、模型配置和顺序写入说明。',markdown:integration,active:'integration'}));
 const handoff='# UniPPT 本机 MCP 集成手册\n\n生成日期：2026-09-16。包含使用指南、本机 stdio 协议和原生结构快照。网页链接以运行中的本地编辑器根目录为起点。\n\n'+integration+'\n\n'+guide+'\n\n'+complete;
 write('docs/UniPPT-MCP-AI-INTEGRATION-2026-09-11.md',handoff);write('web/docs/unippt-integration.md',handoff);
 console.log(JSON.stringify({tools:tools.length,nativeOperations:allowed.length,rowKinds:runtime.imageRows.kinds.length,generated,credentialsIncluded:false,modelCalls:false}));
}
module.exports={build};
