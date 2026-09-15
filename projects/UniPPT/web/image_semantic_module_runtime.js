(function (global) {
  'use strict';
  // Experimental source-measurement layer. No figure-specific geometry or text.
  function measure(rgba, width, height, options = {}) {
    if (rgba.length !== width * height * 4) throw new Error('Invalid source pixel dimensions');
    const counts = new Map();
    for (let p = 0; p < width * height; p++) {
      const i = p * 4, r = rgba[i], g = rgba[i + 1], b = rgba[i + 2];
      // Very pale coloured bands still carry semantic geometry. A 15-level
      // chroma cutoff silently lost the white-washed panel intersection.
      if (Math.min(r, g, b) > 247 || (Math.max(r, g, b) < 100 && Math.max(r, g, b) - Math.min(r, g, b) < 6)) continue;
      const key = (r << 16) | (g << 8) | b;
      counts.set(key, (counts.get(key) || 0) + 1);
    }
    const palette = [...counts].sort((a, b) => b[1] - a[1]).filter(([, n]) => n >= (options.minColorArea || 30)).slice(0, Math.min(40, options.paletteLimit || 12))
      .map(([rgb, area], i) => ({id: `color${i}`, rgb: [rgb >> 16, rgb >> 8 & 255, rgb & 255], fill: `#${rgb.toString(16).padStart(6, '0')}`, area}));
    const regions = [], queue = new Int32Array(width * height);
    for (const color of palette) {
      const tags = new Uint8Array(width * height), seen = new Uint8Array(tags.length);
      for (let p = 0; p < tags.length; p++) tags[p] = color.rgb.every((v, i) => Math.abs(v - rgba[p * 4 + i]) <= (options.tolerance ?? 2)) ? 1 : 0;
      for (let start = 0; start < tags.length; start++) {
        if (!tags[start] || seen[start]) continue;
        let head = 0, tail = 1; queue[0] = start; seen[start] = 1;
        let x0 = width, y0 = height, x1 = 0, y1 = 0;
        while (head < tail) {
          const p = queue[head++], x = p % width, y = Math.floor(p / width);
          x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x + 1); y1 = Math.max(y1, y + 1);
          for (const q of [x ? p - 1 : -1, x + 1 < width ? p + 1 : -1, y ? p - width : -1, y + 1 < height ? p + width : -1]) {
            if (q >= 0 && tags[q] && !seen[q]) { seen[q] = 1; queue[tail++] = q; }
          }
        }
        if (tail < (options.minRegionArea || 20)) continue;
        const pixels = Array.from(queue.subarray(0, tail));
        regions.push({id: `${color.id}-r${regions.filter(r => r.colorId === color.id).length}`, colorId: color.id, fill: color.fill, area: tail,
          box: [x0, y0, x1 - x0, y1 - y0], pixels});
      }
    }
    return {width, height, palette, regions, rgba};
  }
  function convexHull(points) {
    const pts = [...new Map(points.map(p => [p.join(','), p])).values()].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    if (pts.length < 3) throw new Error('A polygon needs non-collinear source pixels');
    const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    const half = list => { const out = []; for (const p of list) { while (out.length > 1 && cross(out[out.length - 2], out[out.length - 1], p) <= 0) out.pop(); out.push(p); } return out; };
    return [...half(pts).slice(0, -1), ...half([...pts].reverse()).slice(0, -1)];
  }
  function publicMeasurements(measured) {
    const {rgba, ...metadata} = measured;
    return {...metadata, regions: measured.regions.map(({pixels, ...rest}) => rest)};
  }
  // Local overlap patches only: source color islands become separate editable
  // silhouettes. This does not imply recovery of the original transparent layers.
  function patchPath(region, measured) {
    const mask = new Set(region.pixels), stride = measured.width + 1, edges = new Map();
    const color=measured.palette.find(c=>c.id===region.colorId);
    if(measured.rgba && color){
      const others=measured.palette.filter(c=>c.area>300 && Math.hypot(...c.rgb.map((v,i)=>v-color.rgb[i]))>3).map(c=>c.rgb).concat([[255,255,255]]);
      for(const p of region.pixels)for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++){
        const x=p%measured.width+dx,y=Math.floor(p/measured.width)+dy;if(x<region.box[0]||x>=region.box[0]+region.box[2]||y<region.box[1]||y>=region.box[1]+region.box[3])continue;
        const q=y*measured.width+x,rgb=Array.from(measured.rgba.slice(q*4,q*4+3)),d=Math.hypot(...rgb.map((v,i)=>v-color.rgb[i]));
        if(d<32 && others.every(c=>d<=Math.hypot(...rgb.map((v,i)=>v-c[i]))))mask.add(q);
      }
    }
    const add = (a,b) => {if (!edges.has(a)) edges.set(a,[]); edges.get(a).push(b);};
    for (const p of mask) {
      const x=p%measured.width, y=Math.floor(p/measured.width), a=y*stride+x;
      if (!mask.has(p-measured.width)) add(a,a+1);
      if (x+1===measured.width || !mask.has(p+1)) add(a+1,a+stride+1);
      if (!mask.has(p+measured.width)) add(a+stride+1,a+stride);
      if (!x || !mask.has(p-1)) add(a+stride,a);
    }
    const paths=[];
    while(edges.size) {
      const start=edges.keys().next().value; let p=start, loop=[], guard=0;
      do {loop.push([p%stride-region.box[0], Math.floor(p/stride)-region.box[1]]); const from=p, next=edges.get(from); if (!next?.length) break; p=next.pop(); if (!next.length) edges.delete(from);} while(p!==start && guard++<mask.size*8);
      const simple=loop.filter((b,i) => {const a=loop[(i+loop.length-1)%loop.length],c=loop[(i+1)%loop.length];return (b[0]-a[0])*(c[1]-b[1])!==(b[1]-a[1])*(c[0]-b[0]);});
      if(simple.length>2) paths.push(simple.map((p,i)=>`${i?'L':'M'} ${p[0]} ${p[1]}`).join(' ')+' Z');
    }
    return paths.join(' ');
  }
  function recoverFan(input, measured) {
    const light=measured.palette.find(c=>c.id===input.color), regions=input.regions.map(id=>measured.regions.find(r=>r.id===id));
    if(!light||regions.some(r=>!r||r.colorId!==light.id)||regions.length<6||regions.length>30)throw new Error('Fan requires source-measured light overlap islands');
    const ranked=[...regions].sort((a,b)=>b.area-a.area), outer=ranked.slice(0,2).sort((a,b)=>a.box[0]-b.box[0]);
    if(ranked[1].area<ranked[2].area*3||Math.abs(outer[0].box[1]-outer[1].box[1])>3||Math.abs(outer[0].box[3]-outer[1].box[3])>3)throw new Error('Source does not support a symmetric two-sided fan');
    const cy=outer.reduce((s,r)=>s+r.box[1]+r.box[3]/2,0)/2;
    // Opposite outer sheets expose opposite corners; the other corners are
    // occluded by the remaining sheets and must not determine the shear.
    const shear=Math.min(...outer.flatMap(r=>[
      Math.min(...r.pixels.filter(p=>p%measured.width===r.box[0]).map(p=>Math.floor(p/measured.width)))-r.box[1],
      r.box[1]+r.box[3]-1-Math.max(...r.pixels.filter(p=>p%measured.width===r.box[0]+r.box[2]-1).map(p=>Math.floor(p/measured.width)))
    ]).filter(v=>v>0));
    if(shear<=0||shear>outer[0].box[2]*2)throw new Error('Fan shear is not source-supported');
    const deficit=light.rgb.map(v=>255-v), norm=Math.hypot(...deficit);
    const darker=measured.palette.filter(c=>c.id!==light.id&&c.area>1000).map(c=>({c,d:c.rgb.map(v=>255-v)})).filter(({d})=>Math.hypot(...d)>norm*1.2&&d.reduce((s,v,i)=>s+v*deficit[i],0)/(Math.hypot(...d)*norm)>.999).sort((a,b)=>Math.hypot(...a.d)-Math.hypot(...b.d));
    if(!darker.length)throw new Error('Fan opacity needs a measured double-overlap tone');
    const alpha=2-darker[0].d.reduce((s,v,i)=>s+v*deficit[i],0)/(norm*norm);
    if(!(alpha>.1&&alpha<.6))throw new Error('Invalid inferred fan opacity');
    const rgb=deficit.map(v=>Math.round(Math.max(0,255-v/alpha))),fill='#'+rgb.map(v=>v.toString(16).padStart(2,'0')).join('');
    const layers=[];
    for(const r of regions){
      let x,w,h;
      if(outer.includes(r)){x=r.box[0];w=r.box[2];h=r.box[3]-shear;}
      else if(r.box[1]+r.box[3]/2<cy){w=outer[0].box[2];x=r.box[0]+r.box[2]-w;h=2*(cy-r.box[1])-shear;}
      else {w=outer[1].box[2];x=r.box[0];h=2*(r.box[1]+r.box[3]-cy)-shear;}
      if(h<8||h>outer[0].box[3]||x<0||x+w>measured.width)throw new Error('Inconsistent fan sheet bounds');
      const top=cy-h/2-shear/2;
      layers.push({kind:'shape',name:`${input.id}-${r.id}`,frame:{x,y:top,width:w,height:h+shear},points:[[0,shear],[w,0],[w,h],[0,h+shear]],style:{fill,stroke:'transparent',strokeWidth:0,opacity:alpha}});
    }
    return {objects:layers,report:{method:'two-sided-fan-source-island-geometry',layerCount:layers.length,shear,alpha,fill,cy}};
  }
  function normalizeCurve(raw) {
    const c={...raw, start:raw.start?.slice(),segments:[]};let previous=c.start;
    for(const s of raw.segments||[]){let next;
      if(s.length===4&&s.every(Number.isFinite)){
        const q=s.slice(0,2),end=s.slice(2);next=[previous.map((v,i)=>v+(q[i]-v)*2/3),end.map((v,i)=>v+(q[i]-v)*2/3),end];
      } else if(s.length===6&&s.every(Number.isFinite))next=[s.slice(0,2),s.slice(2,4),s.slice(4,6)];
      else {next=s.map(p=>{if(!Array.isArray(p))throw new Error('Invalid native curve point');return p.slice();});if(next.length===4&&next[0][0]===previous?.[0]&&next[0][1]===previous?.[1])next=next.slice(1);}
      c.segments.push(next);previous=next.at(-1);
    }return c;
  }
  function build(spec, measured) {
    if (!spec || !Array.isArray(spec.objects) || spec.objects.length > 100) throw new Error('Expected at most 100 semantic objects');
    if (!Array.isArray(spec.texts) || spec.texts.length > 30) throw new Error('Expected a text inventory');
    const ids = new Set(), objects = [];
    for (const input of spec.objects) {
      if (!input.id || ids.has(input.id)) throw new Error('Each object needs a unique id'); ids.add(input.id);
      if(input.kind==='fanSheets'){objects.push(...recoverFan(input,measured).objects);continue;}
      const found = (input.regions || []).map(id => { const r = measured.regions.find(r => r.id === id); if (!r) throw new Error(`Unknown measured region ${id}`); return r; });
      if (!found.length) throw new Error(`Object ${input.id} needs source-measured regions`);
      const x = Math.min(...found.map(r => r.box[0])), y = Math.min(...found.map(r => r.box[1]));
      const w = Math.max(...found.map(r => r.box[0] + r.box[2])) - x, h = Math.max(...found.map(r => r.box[1] + r.box[3])) - y;
      const color = measured.palette.find(c => c.id === input.color);
      if (!color) throw new Error(`Unknown measured color ${input.color}`);
      if (input.kind === 'patches') {
        if (measured.width*measured.height>100000 || Math.max(...color.rgb)-Math.min(...color.rgb)<10) throw new Error('Overlap patches require a bounded colored module, not a full-slide or text trace');
        for (const r of found) {
          // A semantic group may span tones. Color remains owned by each source
          // island, never by the model's group label.
          const ownColor=measured.palette.find(c=>c.id===r.colorId);
          if(!ownColor || Math.max(...ownColor.rgb)-Math.min(...ownColor.rgb)<10) throw new Error('Overlap patches cannot include neutral text pixels');
          objects.push({kind:'shape',name:`${input.id}-${r.id}`,frame:{x:r.box[0],y:r.box[1],width:r.box[2],height:r.box[3]},path:patchPath(r,measured),style:{fill:r.fill,stroke:r.fill,strokeWidth:.35}});
        }
        continue;
      }
      const object = {kind: 'shape', name: input.id, frame: {x, y, width: w, height: h}, style: {fill: color.fill, stroke: 'transparent', strokeWidth: 0}};
      if (input.kind === 'wash') {
        if (!Array.isArray(input.samples) || !input.samples.length) throw new Error('A white wash needs observed/base color pairs');
        let numerator = 0, denominator = 0;
        for (const pair of input.samples) {
          const base = measured.palette.find(c => c.id === pair.base), observed = measured.palette.find(c => c.id === pair.observed);
          if (!base || !observed) throw new Error('Unknown white-wash sample color');
          for (let i = 0; i < 3; i++) {numerator += (observed.rgb[i] - base.rgb[i]) * (255 - base.rgb[i]); denominator += (255 - base.rgb[i]) ** 2;}
        }
        const alpha = numerator / denominator;
        if (!(alpha > .02 && alpha < .98)) throw new Error('Source colors do not support a translucent white wash');
        object.geometry = 'rect'; object.style.fill = '#ffffff'; object.style.opacity = alpha;
      } else if (input.kind === 'contour') {
        if (w * h > measured.width * measured.height * .08) throw new Error('Contour recovery is restricted to isolated small icons');
        const margin = 2, x0 = Math.max(0, x - margin), y0 = Math.max(0, y - margin), x1 = Math.min(measured.width, x + w + margin), y1 = Math.min(measured.height, y + h + margin);
        const mask = new Set();
        for (let py = y0; py < y1; py++) for (let px = x0; px < x1; px++) {
          const i = (py * measured.width + px) * 4, rgb = [measured.rgba[i], measured.rgba[i + 1], measured.rgba[i + 2]];
          const distance = Math.hypot(...rgb.map((v, j) => v - color.rgb[j]));
          const otherDistance = Math.min(...measured.palette.filter(c => c.id !== color.id && c.area > 500).map(c => Math.hypot(...rgb.map((v, j) => v - c.rgb[j]))));
          if (distance < 115 && distance < otherDistance && Math.max(...rgb) - Math.min(...rgb) > 25) mask.add(py * measured.width + px);
        }
        const stride = measured.width + 1, edges = new Map(), add = (a, b) => { if (!edges.has(a)) edges.set(a, []); edges.get(a).push(b); };
        for (const p of mask) {const px = p % measured.width, py = Math.floor(p / measured.width), a = py * stride + px;
          if (!mask.has(p - measured.width)) add(a, a + 1);
          if (px + 1 === measured.width || !mask.has(p + 1)) add(a + 1, a + stride + 1);
          if (!mask.has(p + measured.width)) add(a + stride + 1, a + stride);
          if (px === 0 || !mask.has(p - 1)) add(a + stride, a);
        }
        const paths = [];
        while (edges.size) {const first = edges.keys().next().value; let current = first, loop = [], guard = 0;
          do {loop.push([current % stride - x0, Math.floor(current / stride) - y0]); const next = edges.get(current); if (!next?.length) break; const to = next.pop(); if (!next.length) edges.delete(current); current = to;} while (current !== first && guard++ < mask.size * 8);
          const simple = loop.filter((p, i) => {const a = loop[(i + loop.length - 1) % loop.length], b = loop[(i + 1) % loop.length]; return (p[0] - a[0]) * (b[1] - p[1]) !== (p[1] - a[1]) * (b[0] - p[0]);});
          if (simple.length > 2) paths.push(simple.map((p, i) => `${i ? 'L' : 'M'} ${p[0]} ${p[1]}`).join(' ') + ' Z');
        }
        if (!paths.length) throw new Error('No source-backed icon contour was recovered');
        object.frame = {x: x0, y: y0, width: x1 - x0, height: y1 - y0}; object.path = paths.join(' ');
      } else if (input.kind === 'hull') {
        const points = found.flatMap(r => r.pixels.flatMap(p => { const px = p % measured.width, py = Math.floor(p / measured.width); return [[px, py], [px + 1, py], [px, py + 1], [px + 1, py + 1]]; }));
        object.points = convexHull(points).map(p => [p[0] - x, p[1] - y]);
      } else if (input.kind === 'rect' || input.kind === 'roundRect' || input.kind === 'ellipse') {
        object.geometry = input.kind;
        if (input.kind === 'roundRect') object.cornerRadius = Math.max(0, Math.min(8, Number(input.radius) || 3));
      } else throw new Error(`Unsupported measured primitive ${input.kind}`);
      objects.push(object);
    }
    if (spec.curves && (!Array.isArray(spec.curves) || spec.curves.length>100)) throw new Error('Too many native curves');
    for (const raw of spec.curves || []) {
      const c=normalizeCurve(raw);
      if (!c.id || ids.has(c.id)) throw new Error('Each curve needs a unique id'); ids.add(c.id);
      if (!Array.isArray(c.start) || c.start.length !== 2 || !Array.isArray(c.segments) || !c.segments.length || c.segments.length > 8) throw new Error('A native curve needs one start and bounded cubic segments');
      const points = [c.start, ...c.segments.flat()];
      const inside=(p,margin=0)=>Array.isArray(p)&&p.length===2&&p.every(Number.isFinite)&&p[0]>=-margin&&p[0]<=measured.width+margin&&p[1]>=-margin&&p[1]<=measured.height+margin;
      if (c.segments.some(s => !Array.isArray(s) || s.length !== 3 || !inside(s[2])) || !inside(c.start) || points.some(p=>!inside(p,Math.min(measured.width,measured.height)*.25))) throw new Error('Curve control points must stay in original crop pixels');
      if (!/^#[\da-f]{6}$/i.test(c.color) || !(c.width >= .2 && c.width <= 5)) throw new Error('Invalid source curve stroke');
      const x = Math.min(...points.map(p => p[0])), y = Math.min(...points.map(p => p[1]));
      const width = Math.max(.5, Math.max(...points.map(p => p[0])) - x), height = Math.max(.5, Math.max(...points.map(p => p[1])) - y);
      const local = p => `${p[0] - x} ${p[1] - y}`;
      objects.push({kind: 'shape', name: c.id, frame: {x, y, width, height}, path: `M ${local(c.start)} ` + c.segments.map(s => 'C ' + s.map(local).join(' ')).join(' '),
        style: {fill: 'transparent', stroke: c.color, strokeWidth: c.width, ...(c.dash ? {strokeDash: 'dash'} : {})}});
    }
    for (const t of spec.texts) {
      if (!t.id || ids.has(t.id)) throw new Error('Each text needs a unique id'); ids.add(t.id);
      if (!t.text || !Array.isArray(t.box) || t.box.length !== 4 || !t.box.every(Number.isFinite) || t.box[2] <= 0 || t.box[3] <= 0) throw new Error('Text requires content and pixel xywh box');
      const [x, y, width, height] = t.box;
      const size = Number(t.size); if (!(size > 3 && size <= 100)) throw new Error('Invalid text size');
      if (t.scriptScale != null && (!Number.isFinite(t.scriptScale) || t.scriptScale < .2 || t.scriptScale > 1.5)) throw new Error('Invalid native script size ratio');
      const runs = [], re = /([_^])\{([^}]+)\}/g; let cursor = 0, match;
      while ((match = re.exec(t.text))) { if (match.index > cursor) runs.push({text: t.text.slice(cursor, match.index)}); runs.push({text: match[2], baseline: match[1] === '_' ? 'sub' : 'super'}); cursor = re.lastIndex; }
      if (cursor < t.text.length) runs.push({text: t.text.slice(cursor)});
      for (const run of runs) run.fontSize = size * (run.baseline ? (t.scriptScale ?? .65) : 1);
      objects.push({kind: 'text', name: t.id, frame: {x, y, width, height, rotation: t.rotation || 0}, text: runs.map(r => r.text).join(''), textRuns: runs,
        textStyle: {fontFamily: t.font === 'serif' || !t.font ? 'Times New Roman' : t.font === 'sans-serif' ? 'Arial' : t.font, fontSize: size, color: '#171717', italic: !!t.italic, bold: !!t.bold, align: 'center'}});
    }
    return {attachmentName: 'Semantic module experiment', sourceSize: {width: measured.width, height: measured.height}, slide: {name: 'Semantic module experiment', background: '#ffffff', objects, sourceCrops: [],
      verification: {labels: objects.filter(o => o.kind === 'text').map(o => o.text), rotatedTextCount: objects.filter(o => o.kind === 'text' && Math.abs(o.frame.rotation) > 5).length, diagram: true}}};
  }
  const api = {measure, convexHull, publicMeasurements, build, recoverFan, normalizeCurve};
  global.UniPptImageSemanticModule = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
