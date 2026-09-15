'use strict';
// Compatibility stays inside the consented MCP protocol. Never retry a failed
// write as another method: choose the method during the read-only handshake.
const crypto = require('node:crypto');
const {textPatch, validatePatch} = require('../web/mcp_bridge.js');
const unknown = status => status.state === 'completed' && /^(未知 MCP 命令|Unknown (?:MCP )?(?:command|method))$/.test(status.error || '');
async function prepareContext(execute, sessionId) {
  const modern = await execute('prepare', sessionId, {});
  if (!unknown(modern)) {
    if (modern.state !== 'completed' || modern.error) throw Error(modern.error || 'Browser context uncertain');
    return modern.result;
  }
  const context = await execute('inspect', sessionId, {});
  const schema = await execute('get_edit_schema', sessionId, {});
  if ([context,schema].some(s => s.state !== 'completed' || s.error)) throw Error('Legacy browser context unavailable');
  if (!['insertSlide','addObject','updateObject','removeObject'].every(op => schema.result.allowedOperations?.includes(op))) throw Error('Legacy browser lacks required native operations');
  return {...context.result, protocolMode:'legacy-patch'};
}
function legacyCommand(command) {
  const objects = command.compiledModules.flatMap(m => m.objects).map(o => textPatch({}, o));
  const replacements = command.compiledModules.flatMap(m => m.replaceObjectIds || []);
  if (new Set(replacements).size !== replacements.length) throw Error('Duplicate region replacement');
  let operations;
  if (!command.slideId) {
    if (replacements.length) throw Error('Replacement requires an existing slide');
    operations = [{op:'insertSlide',afterSlideId:command.afterSlideId,slide:{name:command.name,background:'#FFFFFF',objects}}];
  } else {
    const old = new Set(replacements), current = new Set(objects.map(o => o.id));
    operations = replacements.filter(id => !current.has(id)).map(objectId => ({op:'removeObject',slideId:command.slideId,objectId}));
    for (const o of objects) {
      if (old.has(o.id)) {
        const {id,...fields} = o;
        operations.push({op:'updateObject',slideId:command.slideId,objectId:id,patch:{customGeometry:null,asset:null,text:'',textParagraphs:[],children:[],...fields}});
      } else operations.push({op:'addObject',slideId:command.slideId,object:o});
    }
  }
  // Older bridges cap native operation batches at 100; never split an atomic
  // edit automatically. One insertSlide may still carry hundreds of objects.
  validatePatch(operations);
  return {expectedRevision:command.expectedRevision,operations};
}
const childId = (id,phase) => 'mcp-compat-'+crypto.createHash('sha256').update(id+':'+phase).digest('hex').slice(0,40);
async function applyLegacy(execute, sessionId, requestId, record, onCommit) {
  const started = performance.now();
  const status = await execute('apply_patch',sessionId,record.legacyCommand,requestId);
  if (status.state !== 'completed' || status.error) return status;
  const result = status.result;
  // Do not infer an active page from a subsequent selection or a guessed ID.
  const slideId = result.activeSlideId || result.slideId;
  result.writeCommitted = true;
  result.protocolMode = 'legacy-patch';
  try { onCommit(result); } catch(error) { result.imageJournalError=error.message; }
  if (!slideId || !Number.isInteger(result.revision)) return {...status,result:{...result,nativeRendered:false,previewError:'Committed write returned no authoritative slide/revision; inspect before rendering'}};
  const args = {slideId,expectedRevision:result.revision};
  const read = async(method,phase) => {const id=childId(requestId,phase);try{return await execute(method,sessionId,args,id);}catch(error){return {state:'completed',requestId:id,error:error.message};}};
  const [preview, exported] = await Promise.all([
    read('preview','preview'),
    read('export_pptx','export'),
  ]);
  const combined = {...result,slideId,visualFidelityPassed:false};
  if (preview.state === 'completed' && !preview.error) Object.assign(combined,preview.result);
  else Object.assign(combined,{nativeRendered:false,previewError:preview.error || preview.state,previewRequestId:preview.requestId});
  if (exported.state === 'completed' && !exported.error) combined.pptxBase64 = exported.result.pptxBase64;
  else Object.assign(combined,{exportError:exported.error || exported.state,exportRequestId:exported.requestId});
  combined.timings = {...combined.timings,legacyBundleMs:performance.now()-started};
  // Both reads carry the exact committed revision; the browser rejects races.
  combined.revision = result.revision;
  return {...status,result:combined};
}
module.exports = {prepareContext,legacyCommand,applyLegacy};
