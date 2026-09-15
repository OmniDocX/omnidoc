(function(global) {
  'use strict';
  // Deterministic protocol compiler; no model, network, filesystem or source inference.
  const TYPES = ['text', 'box', 'layers', 'arrow', 'path', 'snowflake', 'image'];
  const finite = (n, label, fallback) => { n = n ?? fallback; if (typeof n !== 'number' || !Number.isFinite(n) || Math.abs(n) > 1e7) throw Error('Invalid ' + label); return n; };
  function normalizeGeometry(g) {
    const w=finite(g.width,'path width'),h=finite(g.height,'path height');
    if(w<=0||h<=0||typeof g.pathData!=='string'||g.pathData.length>100000)throw Error('Invalid customGeometry');
    const matches=[...g.pathData.matchAll(/[MLCQZmlcqz]|[-+]?(?:\d*\.)?\d+(?:[eE][-+]?\d+)?/g)];
    let end=0;for(const m of matches){if(!/^[\s,]*$/.test(g.pathData.slice(end,m.index)))throw Error('Paths support absolute M/L/C/Q/Z only');end=m.index+m[0].length;}
    if(!/^[\s,]*$/.test(g.pathData.slice(end)))throw Error('Invalid path suffix');
    const ts=matches.map(m=>m[0]);if(ts[0]!=='M')throw Error('Path must start with absolute M');
    // Idempotent: already integer geometry is not rescaled on every repair.
    const scale=!ts.includes('Q')&&Number.isInteger(w)&&Number.isInteger(h)&&ts.filter(t=>!/[MLCQZ]/.test(t)).every(t=>Number.isInteger(Number(t)))?1:1000;
    const u=n=>{n=finite(n,'path coordinate');return Math.round(n*scale);};let at=0,cx=0,cy=0,sx=0,sy=0;const out=[];
    while(at<ts.length){const cmd=ts[at++];if(cmd==='Z'){out.push('Z');cx=sx;cy=sy;continue;}
      const count={M:2,L:2,C:6,Q:4}[cmd];if(!count)throw Error('Paths support absolute M/L/C/Q/Z only');
      const values=ts.slice(at,at+count);if(values.length!==count||values.some(t=>!/^[+-]?(?:\d*\.)?\d+(?:e[+-]?\d+)?$/i.test(t)))throw Error('Incomplete path command');
      const v=values.map(Number);at+=count;
      if(cmd==='Q'){const [qx,qy,x,y]=v;out.push('C',u(cx+2*(qx-cx)/3),u(cy+2*(qy-cy)/3),u(x+2*(qx-x)/3),u(y+2*(qy-y)/3),u(x),u(y));cx=x;cy=y;}
      else{out.push(cmd,...v.map(u));cx=v[v.length-2];cy=v[v.length-1];if(cmd==='M'){sx=cx;sy=cy;}}
    }
    return {width:Math.max(1,u(w)),height:Math.max(1,u(h)),pathData:out.join(' ')};
  }
  function normalizeObject(source, repairs=[], original={}) {
    const o=structuredClone(source);if(o.frame)for(const [key,v]of Object.entries(o.frame))if(typeof v==='number')finite(v,'frame '+key);
    const geometry=o.customGeometry===undefined?original.customGeometry:o.customGeometry;
    if(geometry){const normalized=normalizeGeometry(geometry);if(JSON.stringify(normalized)!==JSON.stringify(geometry)){o.customGeometry=normalized;repairs.push({objectId:o.id||original.id,code:'normalized_path'});}
      if((o.kind||original.kind)==='connector'){o.kind='shape';o.customGeometry=normalized;repairs.push({objectId:o.id||original.id,code:'custom_connector_as_editable_freeform',endpointAttachment:false});}}
    if(o.children)o.children=o.children.map(child=>normalizeObject(child,repairs));
    return o;
  }
  function compileModules(modules) {
    if(!Array.isArray(modules)||!modules.length||modules.length>64||JSON.stringify(modules).length>1000000)throw Error('Need 1–64 bounded modules');
    const seen=new Set();let total=0;
    return modules.map(m=>{
      if(!/^[a-zA-Z0-9_-]{1,60}$/.test(m.id)||seen.has(m.id)||!TYPES.includes(m.kind))throw Error('Invalid/duplicate module id or kind');seen.add(m.id);
      const f=m.frame||{},x=finite(f.x,'x',0),y=finite(f.y,'y',0),w=finite(f.width,'width',120),h=finite(f.height,'height',80);
      if(w<=0||h<=0)throw Error('Module dimensions must be positive');
      const objects=[],style={fill:'transparent',stroke:'#171717',strokeWidth:1,opacity:1,...m.style};
      const emit=(kind,frame,extra={})=>{const o={id:`mcp-${m.id}-${objects.length}`,name:`MCP module ${m.id}`,kind,frame:{rotation:0,...frame},style:{...style},...extra};objects.push(o);return o;};
      const path=(pts,closed=false,fill='transparent')=>{const xs=pts.map(p=>p[0]),ys=pts.map(p=>p[1]),left=Math.min(...xs),top=Math.min(...ys),ww=Math.max(1,Math.max(...xs)-left),hh=Math.max(1,Math.max(...ys)-top);
        return emit('shape',{x:left,y:top,width:ww,height:hh},{style:{...style,fill},customGeometry:normalizeGeometry({width:ww,height:hh,pathData:pts.map((p,i)=>`${i?'L':'M'} ${p[0]-left} ${p[1]-top}`).join(' ')+(closed?' Z':'')})});};
      if(m.kind==='layers'){
        const count=finite(m.count,'count',6);if(!Number.isInteger(count)||count<1||count>64)throw Error('Layer count must be 1–64');
        const dx=finite(m.dx,'dx',w*.16),dy=finite(m.dy,'dy',0),shrink=finite(m.shrink,'shrink',0),skew=finite(m.skew,'skew',h*.2);
        for(let i=0;i<count;i++){const hh=h-i*shrink;if(hh<=0)throw Error('Taper collapses a layer');path([[x+i*dx,y+i*dy+skew],[x+i*dx+w,y+i*dy],[x+i*dx+w,y+i*dy+hh-skew],[x+i*dx,y+i*dy+hh]],true,m.style?.fill||'#BED8AE');}
      }else if(m.kind==='arrow'){
        const a=m.from||[x,y],b=m.to||[x+w,y+h];if(![a,b].every(p=>Array.isArray(p)&&p.length===2))throw Error('Arrow endpoints need [x,y]');[...a,...b].forEach(v=>finite(v,'endpoint'));
        path([a,b]);const angle=Math.atan2(b[1]-a[1],b[0]-a[0]),size=finite(m.headSize,'arrow head',6);
        if(size<=0||size>100)throw Error('Invalid arrow head');path([b,[b[0]-size*Math.cos(angle-.48),b[1]-size*Math.sin(angle-.48)],[b[0]-size*Math.cos(angle+.48),b[1]-size*Math.sin(angle+.48)]],true,style.stroke).style.strokeWidth=0;
      }else if(m.kind==='snowflake'){
        const cx=x+w/2,cy=y+h/2,r=Math.min(w,h)*.46;
        for(let i=0;i<6;i++){const a=i*Math.PI/3;path([[cx,cy],[cx+r*Math.cos(a),cy+r*Math.sin(a)]]);for(const v of [.55,.85])for(const side of [-1,1]){const px=cx+r*v*Math.cos(a),py=cy+r*v*Math.sin(a),b=a+Math.PI+side*Math.PI/3;path([[px,py],[px+r*.25*Math.cos(b),py+r*.25*Math.sin(b)]]);}}
        // One editable multi-subpath icon avoids 30 PPTX shape imports. Preserve
        // separate paths for translucent strokes: merging changes alpha overlap.
        if(style.opacity===1){const segments=objects.splice(0);emit('shape',{x,y,width:w,height:h},{style:{...style,fill:'transparent'},customGeometry:normalizeGeometry({width:w,height:h,pathData:segments.map(o=>{const ts=o.customGeometry.pathData.split(' ');return ts.map((t,i)=>/[ML]/.test(t)?t:String((i%3===1?o.frame.x-x:o.frame.y-y)+Number(t)*(i%3===1?o.frame.width/o.customGeometry.width:o.frame.height/o.customGeometry.height))).join(' ');}).join(' ')})});}
      }else if(m.kind==='text')emit('text',{x,y,width:w,height:h,rotation:finite(f.rotation,'rotation',0)},{textRuns:m.textRuns||[{text:String(m.text||'')}],textStyle:{fontFamily:'Arial',fontSize:20,color:'#171717',...m.textStyle},textFrame:{marginLeft:0,marginRight:0,marginTop:0,marginBottom:0,autoSize:'none',...m.textFrame},style:{fill:'transparent',stroke:'transparent',strokeWidth:0}});
      else if(m.kind==='path')emit('shape',{x,y,width:w,height:h},{customGeometry:normalizeGeometry(m.customGeometry||{width:w,height:h,pathData:m.pathData})});
      else if(m.kind==='image')emit('image',{x,y,width:w,height:h},{imageData:m.imageData});
      else emit('shape',{x,y,width:w,height:h},{geometry:m.geometry||'rect'});
      total+=objects.length;if(total>1000)throw Error('Expanded scene exceeds 1000 objects');
      if(m.replaceObjectIds!==undefined&&(!Array.isArray(m.replaceObjectIds)||m.replaceObjectIds.some(id=>typeof id!=='string')||new Set(m.replaceObjectIds).size!==m.replaceObjectIds.length))throw Error('Invalid replaceObjectIds');
      return {id:m.id,objects,replaceObjectIds:m.replaceObjectIds||[]};
    });
  }
  function sceneOperations(deck,args) {
    const mods=args.compiledModules;if(!Array.isArray(mods)||!mods.length||mods.length>64)throw Error('Missing compiled modules');
    const objects=mods.flatMap(m=>m.objects);if(objects.length>1000)throw Error('Scene too large');
    if(!args.slideId){if(mods.some(m=>m.replaceObjectIds.length))throw Error('Replacement requires slideId');return [{op:'insertSlide',afterSlideId:args.afterSlideId,slide:{name:args.name||'MCP scene',background:'#FFFFFF',objects}}];}
    const slide=deck.slides.find(s=>s.id===args.slideId);if(!slide)throw Error('Unknown slideId');
    const existing=new Map(slide.objects.map(o=>[o.id,o])),removed=new Set(),ops=[];
    for(const m of mods){const replacements=new Set(m.replaceObjectIds);for(const id of replacements){if(!existing.has(id)||removed.has(id))throw Error('Missing or repeated replacement ID '+id);removed.add(id);}
      const newIds=new Set(m.objects.map(o=>o.id));for(const id of replacements)if(!newIds.has(id))ops.push({op:'removeObject',slideId:slide.id,objectId:id});
      for(const o of m.objects){if(existing.has(o.id)&&!replacements.has(o.id))throw Error('Object collision; supply observed replaceObjectIds for this module');
        if(replacements.has(o.id)){const patch={customGeometry:null,asset:null,text:'',textParagraphs:[],children:[],...o};if(o.kind==='text')delete patch.text;delete patch.id;ops.push({op:'updateObject',slideId:slide.id,objectId:o.id,patch});}
        else ops.push({op:'addObject',slideId:slide.id,object:o});}
    }return ops;
  }
  function checkSlide(slide,width,height) {
    const issues=[];let objects=0,texts=0;const ids=new Set();
    const walk=list=>{for(const o of list||[]){objects++;if(ids.has(o.id))issues.push({objectId:o.id,code:'duplicate_id',severity:'error'});ids.add(o.id);if(o.kind==='text')texts++;
      const f=o.frame;if(f){const a=(f.rotation||0)*Math.PI/180,c=Math.abs(Math.cos(a)),s=Math.abs(Math.sin(a)),rw=f.width*c+f.height*s,rh=f.width*s+f.height*c,cx=f.x+f.width/2,cy=f.y+f.height/2;
        if(cx-rw/2<-.5||cy-rh/2<-.5||cx+rw/2>width+.5||cy+rh/2>height+.5)issues.push({objectId:o.id,code:'outside_canvas',severity:'warning',region:f});}
      walk(o.children);}};walk(slide.objects);
    issues.push(...(global.UniPptPresentationHost?.nativeTextIssues?.({slides:[slide]}) || []));
    return {objects,nativeTextObjects:texts,issues,structuralPassed:!issues.some(i=>i.severity==='error'),visualFidelityPassed:false};
  }
  function moduleSchema(){return {units:'slide CSS pixels; rotation in degrees',types:TYPES,common:['id','kind','frame','style','replaceObjectIds'],fields:{text:['text','textRuns','textStyle','textFrame'],layers:['count','dx','dy','skew','shrink'],arrow:['from:[x,y]','to:[x,y]','headSize'],path:['customGeometry:{width,height,pathData}'],box:['geometry'],image:['imageData:{mimeType:image/png,base64}']},example:[{id:'encoder',kind:'layers',frame:{x:100,y:80,width:32,height:150},count:6,dx:16,dy:10,shrink:20,skew:25,style:{fill:'#BED8AE',stroke:'transparent',opacity:.5}},{id:'flow',kind:'arrow',from:[240,150],to:[320,150]}],limits:{modules:64,expandedObjects:1000,atomicOperations:512},notes:['Call apply_scene once for write + native PNG + structural audit. Intermediate PPTX delivery defaults off; export_pptx once after visual review. Explicit includePptx:true opts into a preview-slide attachment, not a final whole-deck result.','Absolute M/L/C/Q/Z paths are normalized before mutation.','Opaque snowflakes are single editable multi-subpath shapes; translucent strokes remain separate.','Custom-path arrows are editable freeforms, not endpoint attachments.','Use slideId + replaceObjectIds from moduleMap for affected modules only.','No OCR, AI keys, remote models or automatic visual-fidelity pass.']};}
  const api={TYPES,moduleSchema,normalizeGeometry,normalizeObject,compileModules,sceneOperations,checkSlide};global.UniPptMcpScene=api;if(typeof module!=='undefined')module.exports=api;
})(globalThis);
