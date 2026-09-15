'use strict';
// Read-only renderer compatibility adapter for DrawingML run baselines dropped
// by Artifact Tool 2.8.59. It changes Canvas painting only, never the PPTX or
// imported presentation. Unsupported/ambiguous runs remain explicit failures.
const fs=require('node:fs'),zlib=require('node:zlib');
function slideXml(input){
 const b=fs.readFileSync(input);if(b.length>100*1024*1024)throw Error('Baseline preview package exceeds 100 MiB');
 let end=b.length-22;const lower=Math.max(0,b.length-65557);while(end>=lower&&b.readUInt32LE(end)!==0x06054b50)end--;if(end<lower)throw Error('Invalid PPTX central directory');
 let at=b.readUInt32LE(end+16);const count=b.readUInt16LE(end+10);if(count===65535)throw Error('ZIP64 is unsupported by baseline preview');
 for(let i=0;i<count;i++){
  if(at+46>b.length||b.readUInt32LE(at)!==0x02014b50)throw Error('Invalid PPTX directory entry');
  const n=b.readUInt16LE(at+28),extra=b.readUInt16LE(at+30),comment=b.readUInt16LE(at+32),name=b.subarray(at+46,at+46+n).toString('utf8');
  if(name==='ppt/slides/slide1.xml'){
   const flags=b.readUInt16LE(at+8),method=b.readUInt16LE(at+10),packed=b.readUInt32LE(at+20),size=b.readUInt32LE(at+24),local=b.readUInt32LE(at+42);
   if(flags&1||size>8*1024*1024||local+30>b.length||b.readUInt32LE(local)!==0x04034b50)throw Error('Unsupported PPTX slide entry');
   const start=local+30+b.readUInt16LE(local+26)+b.readUInt16LE(local+28);if(start+packed>b.length)throw Error('Truncated PPTX slide');
   const bytes=b.subarray(start,start+packed),xml=method===0?bytes:method===8?zlib.inflateRawSync(bytes,{maxOutputLength:8*1024*1024}):null;
   if(!xml||xml.length!==size)throw Error('Invalid PPTX slide compression/size');return xml.toString('utf8');
  }at+=46+n+extra+comment;
 }throw Error('Missing native slide1.xml');
}
const unescapeXml=s=>s.replace(/&(#x[0-9a-f]+|#\d+|amp|lt|gt|quot|apos);/gi,(_,c)=>c[0]==='#'?String.fromCodePoint(c[1].toLowerCase()==='x'?parseInt(c.slice(2),16):Number(c.slice(1))):({amp:'&',lt:'<',gt:'>',quot:'"',apos:"'"})[c]);
const attrs=s=>Object.fromEntries([...s.matchAll(/([\w:]+)="([^"]*)"/g)].map(m=>[m[1],unescapeXml(m[2])]));
function baselineRuns(xml){
 if(/<!DOCTYPE|<!ENTITY/.test(xml))throw Error('XML entities are unsupported');
 const runs=[],unsupported=[];
 for(const match of xml.matchAll(/<p:sp\b[^>]*>[\s\S]*?<\/p:sp>/g)){
  const shape=match[0],meta=attrs(shape.match(/<p:cNvPr\b[^>]*>/)?.[0]||''),xf=attrs(shape.match(/<a:xfrm\b[^>]*>/)?.[0]||''),off=attrs(shape.match(/<a:off\b[^>]*>/)?.[0]||''),ext=attrs(shape.match(/<a:ext\b[^>]*>/)?.[0]||''),box=[off.x,off.y,ext.cx,ext.cy].map(n=>Number(n)/9525),paragraphs=(shape.match(/<a:p>/g)||[]).length;
  for(const rm of shape.matchAll(/<a:r>[\s\S]*?<\/a:r>/g)){
   const r=rm[0],props=attrs(r.match(/<a:rPr\b[^>]*>/)?.[0]||''),baseline=Number(props.baseline||0);if(!baseline)continue;
   const text=unescapeXml(r.match(/<a:t[^>]*>([\s\S]*?)<\/a:t>/)?.[1]||''),font=attrs(r.match(/<a:latin\b[^>]*>/)?.[0]||'').typeface,fontPx=Number(props.sz)/75;
   const item={shapeId:meta.id,name:meta.name,text,baseline,font,fontPx,box};
   if(!text||!font||!Number.isFinite(fontPx)||fontPx<=0||box.some(n=>!Number.isFinite(n))||Number(xf.rot||0)!==0||paragraphs!==1||(props.u&&props.u!=='none')||/<a:(?:outerShdw|innerShdw|gradFill)\b/.test(r)||/<p:grpSp\b/.test(xml))unsupported.push(item);else runs.push(item);
  }
 }
 return {runs,unsupported};
}
let active=false;
async function paintNativeBaselines(Context,runs,draw,scale=1){
 if(active)throw Error('Baseline projection must run serially');active=true;const original=Context.prototype.fillText,seen=new Set(),ambiguous=[];
 Context.prototype.fillText=function(text,x,y,...rest){
  const matrix=this.getTransform(),px=matrix.a*x+matrix.c*y+matrix.e,py=matrix.b*x+matrix.d*y+matrix.f,fontPx=Number(this.font.match(/([\d.]+)px/)?.[1]);
  const found=runs.filter((r,i)=>{const [l,t,w,h]=r.box;return !seen.has(i)&&r.text===String(text)&&Math.abs(fontPx-r.fontPx)<.15&&this.font.includes(r.font)&&px>=l*scale-2&&px<=(l+w)*scale+2&&py>=t*scale-2&&py<=(t+h)*scale+5;});
  if(found.length===1){const r=found[0];seen.add(runs.indexOf(r));y-=r.baseline/100000*fontPx;}else if(found.length>1)ambiguous.push(String(text));
  return original.call(this,text,x,y,...rest);
 };
 try{const result=await draw();return {result,report:{method:'DrawingML-baseline-canvas-projection',expected:runs.length,painted:seen.size,unmatched:runs.filter((_,i)=>!seen.has(i)).map(({shapeId,text})=>({shapeId,text})),ambiguous,independentPowerPointCheck:false}};}
 finally{Context.prototype.fillText=original;active=false;}
}
module.exports={slideXml,baselineRuns,paintNativeBaselines};
