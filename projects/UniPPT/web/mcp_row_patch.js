(function(global){
'use strict';
const byteLength=s=>new TextEncoder().encode(s).length;
const FORBIDDEN=new Set(['__proto__','prototype','constructor']);
const plain=v=>v!==null&&typeof v==='object'&&!Array.isArray(v)&&[Object.prototype,null].includes(Object.getPrototypeOf(v));
function safe(value,depth=0){
  if(depth>16)throw Error('Patch nesting exceeds 16 levels');
  if(typeof value==='number'&&!Number.isFinite(value))throw Error('Invalid patch number');
  if(value&&typeof value==='object'){
    if(!Array.isArray(value)&&!plain(value))throw Error('Patch needs plain JSON');
    for(const [key,v]of Object.entries(value)){if(FORBIDDEN.has(key))throw Error('Unsafe patch key');safe(v,depth+1);}
  }else if(!['string','number','boolean'].includes(typeof value)&&value!==null)throw Error('Patch needs plain JSON');
}
function merge(base,patch){
  const result=plain(base)?structuredClone(base):{};
  for(const [key,value]of Object.entries(patch))result[key]=plain(value)?merge(result[key],value):structuredClone(value);
  return result;
}
function resolveRows(args,job){
  const rows=args.rows??[],patches=args.patches??[];
  if(!Array.isArray(rows)||!Array.isArray(patches)||!rows.length&&!patches.length||rows.length+patches.length>160)throw Error('Need 1–160 rows/patches combined');
  if(byteLength(JSON.stringify({rows,patches}))>150000)throw Error('Row/patch input exceeds 150000 bytes');
  safe({rows,patches});
  const resolved=structuredClone(rows),replace=args.replaceRegions??[];
  if(!Array.isArray(replace)||new Set(replace).size!==replace.length)throw Error('Invalid replaceRegions');
  const ids=new Set(rows.map(r=>r?.[0]));
  for(const patch of patches){
    if(!plain(patch)||Object.keys(patch).some(k=>!['id','frame','value','options'].includes(k))||Object.keys(patch).length<2)throw Error('Patch needs id and frame/value/options');
    const id=patch.id;
    if(typeof id!=='string'||ids.has(id))throw Error('Duplicate or invalid patch region');
    if(!job.slideId||args.slideId!==job.slideId||!Object.hasOwn(job.regions,id)||!Object.hasOwn(job.layoutRows||{},id))throw Error('Patch requires an existing committed source row in this image job; resend the full row for older jobs');
    const row=structuredClone(job.layoutRows[id]);
    if(Object.hasOwn(patch,'frame')){
      const f=patch.frame;
      if(!Array.isArray(f)||f.length!==4||f.some(v=>typeof v!=='number'||!Number.isFinite(v)||Math.abs(v)>1e6)||f[2]<=0||f[3]<=0)throw Error('Patch frame needs [x,y,width,height] in source pixels');
      row.splice(2,4,...f);
    }
    // Values and arrays replace in full; nested option maps merge, preserving
    // unmentioned text overrides and native style/geometry parameters.
    if(Object.hasOwn(patch,'value'))row[6]=structuredClone(patch.value);
    if(Object.hasOwn(patch,'options')){
      if(!plain(patch.options))throw Error('Patch options must be a plain object');
      row[7]=merge(row[7],patch.options);
    }
    resolved.push(row);ids.add(id);
  }
  if(byteLength(JSON.stringify(resolved))>150000)throw Error('Resolved rows exceed 150000 bytes');
  return {rows:resolved,replace:[...new Set([...replace,...patches.map(p=>p.id)])],authoring:{patchCount:patches.length,inputBytes:byteLength(JSON.stringify({rows,patches})),resolvedRowBytes:byteLength(JSON.stringify(resolved)),modelTimeMeasured:false}};
}

global.UniPptMcpRowPatch={resolveRows};if(typeof module!=='undefined')module.exports={resolveRows};
})(globalThis);
