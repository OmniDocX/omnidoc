(function (global) {
  "use strict";
  // A source-only pipeline: no fixture coordinates, reviewed decks or hidden
  // reference assets. The same functions run in the UI and the offline probe.
  const clone = (v) => JSON.parse(JSON.stringify(v));
  const num = (v, name) => { if (!Number.isFinite(v)) throw new Error(`${name} 必须为有限数值`); return v; };
  const normalize = (s) => String(s).normalize("NFKC").replace(/[\s_{}^̂~]/g, "").toLowerCase();
  function json(text) {
    const raw = String(text || "").trim().replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/, "");
    const result = JSON.parse(raw);
    if (!result || typeof result !== "object" || Array.isArray(result)) throw new Error("需要 JSON 对象");
    return result;
  }
  function runs(text) {
    const result = [], re = /([_^])\{([^}]+)\}/g;
    let cursor = 0, match;
    while ((match = re.exec(text))) {
      if (match.index > cursor) result.push({ text: text.slice(cursor, match.index) });
      result.push({ text: match[2], baseline: match[1] === "_" ? "sub" : "super" });
      cursor = re.lastIndex;
    }
    if (cursor < text.length) result.push({ text: text.slice(cursor) });
    return result.length ? result : [{ text }];
  }
  function frame(box) {
    if (!Array.isArray(box) || box.length < 4 || box.length > 5) throw new Error("box 为 [x,y,w,h,rotation?]");
    const [x, y, width, height, rotation = 0] = box.map((v) => num(v, "box"));
    if (width <= 0 || height <= 0) throw new Error("box 宽高必须大于零");
    return { x, y, width, height, rotation };
  }
  function validateInventory(inventory, sourceSize) {
    if (inventory.coordinateSpace === "normalized_999") return validateInventory(materialize({ shapes:[], lines:[] }, inventory, sourceSize).inventory, sourceSize);
    if (!Array.isArray(inventory.labels) || !inventory.labels.length || inventory.labels.length > 250) throw new Error("识别清单缺少 labels 或数量超限");
    if (!Array.isArray(inventory.photos) || inventory.photos.length > 40) throw new Error("识别清单需要 photos 数组");
    if (!Array.isArray(inventory.modules) || inventory.modules.length > 100) throw new Error("识别清单需要 modules 数组");
    const ids = new Set();
    for (const item of [...inventory.labels, ...inventory.photos, ...inventory.modules]) {
      if (!item.id || typeof item.id !== "string" || ids.has(item.id)) throw new Error("清单 id 必须唯一");
      ids.add(item.id);
      const f = frame(item.box);
      if (Math.abs(f.x) > sourceSize.width * 2 || Math.abs(f.y) > sourceSize.height * 2 || f.width > sourceSize.width * 2 || f.height > sourceSize.height * 2) throw new Error(`清单 ${item.id} 坐标范围错误`);
    }
    for (const label of inventory.labels) {
      if (typeof label.text !== "string" || !label.text.trim()) throw new Error("文字清单含空标签");
      if (!(label.size > 0 && label.size < sourceSize.height)) throw new Error("文字字号错误");
    }
    return inventory;
  }
  function compilePlan(plan, inventory, attachment) {
    if (inventory.coordinateSpace === "normalized_999") {
      const pixels = materialize(plan, inventory, attachment);
      return compilePlan(pixels.plan, pixels.inventory, attachment);
    }
    validateInventory(inventory, attachment);
    if (!Array.isArray(plan.shapes) || !Array.isArray(plan.lines)) throw new Error("几何方案需要 shapes 和 lines 数组");
    const objects = [], ids = new Set();
    const add = (id, object) => {
      if (typeof id !== "string" || !id || ids.has(id)) throw new Error(`图层 id 重复或为空：${id}`);
      ids.add(id); objects.push({ name: id, ...object });
    };
    for (const s of plan.shapes) {
      // [id,geometry,x,y,w,h,fill,stroke,strokeWidth,path?,count?,dx?,dy?,dash?]
      if (!Array.isArray(s) || s.length < 9 || s.length > 14) throw new Error("shape 需要至少 9 项，最多 14 项");
      const [id, geometry, x,y,w,h, fill,stroke,strokeWidth,path,count,dx,dy,dash] = s;
      if (!["rect", "roundRect", "ellipse", "path"].includes(geometry)) throw new Error(`不支持形状 ${geometry}`);
      const object = { kind: "shape", geometry: geometry === "path" ? "rect" : geometry, frame: frame([x,y,w,h]), style: { fill: fill || "transparent", stroke: stroke || "transparent", strokeWidth: num(strokeWidth, "strokeWidth"), ...(dash ? { strokeDash: dash } : {}) } };
      if (geometry === "path") {
        if (typeof path !== "string" || !/^[MLCZ\d.,+\-\seE]+$/.test(path)) throw new Error(`路径 ${id} 只支持绝对 M/L/C/Z`);
        object.path = path;
      }
      if (count != null && count !== 1) object.repeat = { count, dx: dx || 0, dy: dy || 0 };
      add(id, object);
    }
    for (const line of plan.lines) {
      // [id,x1,y1,x2,y2,color,width,dash?,arrow?]
      if (!Array.isArray(line) || line.length < 7 || line.length > 9) throw new Error("line 需要 7–9 项");
      const [id,x1,y1,x2,y2,color,width,dash,arrow = true] = line;
      [x1,y1,x2,y2,width].forEach((v) => num(v, "line"));
      const f = { x: Math.min(x1,x2), y: Math.min(y1,y2), width: Math.max(.1,Math.abs(x2-x1)), height: Math.max(.1,Math.abs(y2-y1)) };
      add(id, { kind: "connector", frame: f, path: `M ${x1-f.x} ${y1-f.y} L ${x2-f.x} ${y2-f.y}`, style: { fill: "transparent", stroke: color || "#222222", strokeWidth: width, ...(dash ? { strokeDash: dash } : {}) } });
      if (arrow) {
        const angle = Math.atan2(y2-y1,x2-x1), length = Math.max(5, width * 3.5), half = length * .45;
        const points = [[x2,y2], [x2-length*Math.cos(angle)+half*Math.sin(angle),y2-length*Math.sin(angle)-half*Math.cos(angle)], [x2-length*Math.cos(angle)-half*Math.sin(angle),y2-length*Math.sin(angle)+half*Math.cos(angle)]];
        const x = Math.min(...points.map((p) => p[0])), y = Math.min(...points.map((p) => p[1]));
        add(`${id}-head`, { kind: "shape", frame: frame([x,y, Math.max(.1,Math.max(...points.map((p) => p[0]))-x), Math.max(.1,Math.max(...points.map((p) => p[1]))-y)]), points: points.map((p) => [p[0]-x,p[1]-y]), style: { fill: color || "#222222", stroke: "transparent", strokeWidth: 0 } });
      }
    }
    for (const label of inventory.labels) {
      const textRuns = runs(label.text);
      add(label.id, { kind: "text", frame: frame(label.box), text: textRuns.map((r) => r.text).join(""), textRuns,
        textStyle: { fontFamily: label.font || inventory.font || "Times New Roman", fontSize: label.size, color: label.color || "#171717", bold: /b/.test(label.flags || ""), italic: /i/.test(label.flags || ""), align: label.align || "center" } });
    }
    const sourceCrops = inventory.photos.map((photo) => {
      const f = frame(photo.box);
      if (f.x < 0 || f.y < 0 || f.x+f.width > attachment.width || f.y+f.height > attachment.height) throw new Error(`照片 ${photo.id} 超出原图`);
      return { name: photo.id, frame: f, crop: { left: f.x / attachment.width, top: f.y / attachment.height, right: 1-(f.x+f.width)/attachment.width, bottom: 1-(f.y+f.height)/attachment.height } };
    });
    return { attachmentName: attachment.name, sourceSize: { width: attachment.width, height: attachment.height }, sourceImage: attachment.dataUrl,
      slide: { name: attachment.name.replace(/\.[^.]+$/, "") + "_AI原生重建", mode: "native", background: inventory.background || "#ffffff", objects, sourceCrops,
        verification: { labels: inventory.labels.map((l) => runs(l.text).map((r) => r.text).join("")), rotatedTextCount: inventory.labels.filter((l) => Math.abs(l.box[4] || 0) > 5).length, diagram: inventory.modules.length > 0 } } };
  }
  function materialize(plan, inventory, source) {
    const sx=source.width/999, sy=source.height/999;
    const dashStyle=(value)=>{
      if(value==null || value==="")return null;
      if(!["dash","dot"].includes(value))throw new Error("dash 必须为 dash/dot，不能是数组或未声明的线型");
      return value;
    };
    const bbox = (b) => {
      if (!Array.isArray(b) || b.length!==4 || !b.every(Number.isFinite) || b[2]<=b[0] || b[3]<=b[1] || b.some((n)=>n<0 || n>999)) throw new Error("bbox 必须为 0–999 归一化 [左,上,右,下]，不是 [x,y,w,h]");
      return [b[0]*sx,b[1]*sy,(b[2]-b[0])*sx,(b[3]-b[1])*sy];
    };
    const pixelInventory = { ...inventory, coordinateSpace:"pixels",
      labels:(inventory.labels || []).map((l)=>{
        const b=bbox(l.bbox), rotation=l.rotation || 0;
        const vertical=Math.abs(rotation)%180===90;
        const width=vertical ? b[3] : b[2], height=vertical ? b[2] : b[3];
        return {...l,box:[b[0]+b[2]/2-width/2,b[1]+b[3]/2-height/2,width,height,rotation],size:l.size || height*.9};
      }),
      photos:(inventory.photos || []).map((p)=>({...p,box:bbox(p.bbox)})),
      modules:(inventory.modules || []).map((m)=>({...m,box:bbox(m.bbox)})) };
    const shapes=(plan.shapes || []).map((s)=>{
      if (!s || Array.isArray(s) || !s.bbox) throw new Error("归一化 shape 必须是含 bbox 的对象");
      const b=bbox(s.bbox);
      // Path coordinates are local 0–1000 units inside each shape's bbox.
      let path=s.path;
      if (typeof path==="string") {
        const viewBox=s.viewBox || [1000,1000];
        if(!Array.isArray(viewBox) || viewBox.length!==2 || !viewBox.every(v=>Number.isFinite(v)&&v>0&&v<=20000))throw new Error("path viewBox 必须显式声明局部宽高");
        const values=path.match(/[-+]?(?:\d*\.)?\d+(?:e[-+]?\d+)?/gi)?.map(Number) || [];
        if(!s.viewBox && values.length>=4){
          const xs=values.filter((_,i)=>i%2===0),ys=values.filter((_,i)=>i%2===1);
          if(Math.max(...xs)-Math.min(...xs)<500 && Math.max(...ys)-Math.min(...ys)<500)throw new Error(`路径 ${s.id} 坐标与默认 1000×1000 局部坐标不符；请声明实际 viewBox，不能猜测缩放`);
        }
        let ordinal=0;
        path=path.replace(/[-+]?(?:\d*\.)?\d+(?:e[-+]?\d+)?/gi,(n)=>{const axis=ordinal++%2;return String(Number(n)*(axis?b[3]:b[2])/viewBox[axis]);});
      }
      return [s.id,s.geometry,...b,s.fill || "transparent",s.stroke || "transparent",s.width ?? 0,path || null,s.repeat?.[0] || null,(s.repeat?.[1] || 0)*sx,(s.repeat?.[2] || 0)*sy,dashStyle(s.dash)];
    });
    const lines=(plan.lines || []).map((l)=>{
      if (!l || Array.isArray(l) || !Array.isArray(l.from) || !Array.isArray(l.to) || [...l.from,...l.to].length!==4 || ![...l.from,...l.to].every((v)=>Number.isFinite(v) && v>=0 && v<=999)) throw new Error("归一化 line 需要 from/to 坐标");
      return [l.id,l.from[0]*sx,l.from[1]*sy,l.to[0]*sx,l.to[1]*sy,l.color || "#222222",l.width ?? 1,dashStyle(l.dash),l.arrow ?? true];
    });
    return {inventory:pixelInventory,plan:{shapes,lines}};
  }
  function patchState(state, patch) {
    const next = clone(state);
    for (const [field, values] of Object.entries(patch || {})) {
      if (!["labels", "photos", "modules", "shapes", "lines"].includes(field) || !Array.isArray(values) || values.length > 250) throw new Error(`不支持修正字段 ${field}`);
      const target = ["shapes", "lines"].includes(field) ? next.plan : next.inventory;
      const ids = new Set();
      for (const value of values) {
        const id = Array.isArray(value) ? value[0] : value.id;
        if (!id || ids.has(id)) throw new Error("修正 id 为空或重复");
        ids.add(id);
        const index = target[field].findIndex((old) => (Array.isArray(old) ? old[0] : old.id) === id);
        if (value.remove === true) {
          if (index < 0) throw new Error(`删除的修正图层不存在：${id}`);
          target[field].splice(index, 1);
        } else if (index < 0) target[field].push(value);
        else target[field][index] = field==="labels" && target[field][index].anchored ? {...value,bbox:target[field][index].bbox,rotation:target[field][index].rotation,anchored:true} : value;
      }
    }
    return next;
  }
  function assertReview(review) {
    if (typeof review?.passed !== "boolean" || !Array.isArray(review.issues) || !review.checks) throw new Error("视觉复核缺少 passed/issues/checks");
    for (const key of ["text", "rotation", "modules", "connections", "photos", "layout"]) if (typeof review.checks[key] !== "boolean") throw new Error(`视觉复核缺少 ${key}`);
    if (review.passed && (review.issues.length || Object.values(review.checks).some((v) => v !== true) || Object.values(review.patch || {}).some((v) => v.length))) throw new Error("视觉复核结果自相矛盾");
    return review;
  }
  const INVENTORY_PROMPT = `You reconstruct scientific figures faithfully, not redesign presentations. Output ONLY JSON, no markdown. Inspect EVERY text occurrence, vertical margin labels, math scripts, legends and headings. Source image is authoritative; OCR is fallible supporting evidence. Use the Qwen grounding coordinate system: ALL bounding boxes are [x_min,y_min,x_max,y_max] normalized to 0–999 on EACH axis. NEVER output pixels or width/height.
Return {coordinateSpace:"normalized_999",font,background,labels:[{id,text,bbox,rotation,flags,color,align?}],photos:[{id,bbox}],modules:[{id,bbox,description}],connections:[description,...]}.
Every text occurrence has a unique id, including repeated text. bbox tightly encloses VISIBLE glyphs (not hypothetical text frames). Vertical writing has a tall narrow bbox and rotation=-90 or 90; the compiler handles rotation math. Do NOT swap its bbox dimensions. Use _{subscript} and ^{superscript} for ALL mathematical scripts, never bare _words or unicode script digits. flags is b/i/bi/empty. Do not specify a guessed font size; the compiler measures from the glyph box. photos are ONLY real photographic or noise patches, not diagrams or icons. A stack of overlapping photographs may be one crop; its bbox must not include the caption. modules describe all dashed containers, feature stacks, U-net blocks, diffusion fans, curved routes and train/frozen symbols. Match ALL source labels without rewriting or adding any content.`;
  const GEOMETRY_SCHEMA = `shapes:[{id,geometry,bbox:[left,top,right,bottom],fill,stroke,width,path?,repeat?,dash?}],lines:[{id,from:[x,y],to:[x,y],color,width,dash?,arrow?}]. All bbox and endpoint coordinates are normalized 0–999 on EACH axis, NEVER pixel coordinates and NEVER width/height. geometry is rect/roundRect/ellipse/path. fill/stroke use hex or transparent. width is stroke width in original image pixels (normally 1–2). Path uses absolute M/L/C/Z in LOCAL 0–1000 coordinates INSIDE its own bbox, not the whole page. Example slanted panel: M 0 300 L 1000 0 L 1000 700 L 0 1000 Z. repeat:[count,dx,dy] offsets copies in whole-image normalized units. dash is dash/dot. arrow defaults true for lines. Curves use path plus a separate triangular arrowhead. IDs must be unique.`;
  const GEOMETRY_PROMPT = `Reconstruct ALL non-text geometry faithfully, using the attached inventory's normalized coordinates as anchors. Output ONLY JSON {shapes:[...],lines:[...]}. The inventory owns text and photos; never output text or image layers. Layer order: shapes, lines, labels/photos. Detailed neural network panels need perspective polygons, repeated narrow bars and accurately sized fan layers; never replace them with coarse rectangles. Trace the outer boundaries, stacked layers and icons. No whole-page picture or masking. Do not create invisible shapes for semantic groups. Use repeat to reduce output.\n` + GEOMETRY_SCHEMA;
  const PATH_SPACE_PROMPT = "Every path shape must also declare viewBox:[localWidth,localHeight]. Its path coordinates use that local space, and its bbox tightly bounds the full path. Prefer viewBox:[1000,1000] and path values spanning that box. Do not mix local path coordinates with image-normalized coordinates. dash must be the string dash or dot, never an array.";
  const REVIEW_PROMPT = `You are the visual QA pass of an editable-figure reconstruction. Image 1 is SOURCE; image 2 is the ACTUAL compiled preview. Independently compare every text occurrence, vertical label, module topology, layer fan, line route/arrowhead, photo crop, font scale, color and relative placement. Missing or different elements FAIL. Do not pass an approximate block diagram when the source contains detailed geometry. A valid JSON/package is irrelevant to visual fidelity.
Return ONLY JSON {passed:boolean,checks:{text:boolean,rotation:boolean,modules:boolean,connections:boolean,photos:boolean,layout:boolean},issues:[short specific differences],patch:{labels:[],photos:[],modules:[],shapes:[],lines:[]}}.
Use the provided state IDs. Each patch item is the COMPLETE replacement or addition in its original format. Remove an incorrect item with {id,remove:true}. Limit corrections to actual source mismatches. Text corrections use {id,text,bbox:[left,top,right,bottom],rotation,flags,color}, ALL bbox coordinates normalized 0–999, with a tall narrow visual bbox for vertical text. Never edit wording for style. If passed, all six checks must be true, issues empty and patch empty. If uncertain, passed=false. A repaired result will be rendered and checked again; never claim an unapplied patch passed.`;
  function makeAnchors(detections,source){
    const anchors=[];
    for(const detection of detections){
      for(const r of detection.regions || []){
        if(r.location?.length!==8)continue;
        const points=[];
        for(let i=0;i<8;i+=2)points.push(detection.rotation===90 ? [r.location[i+1],source.height-r.location[i]] : [r.location[i],r.location[i+1]]);
        const box=[Math.min(...points.map(p=>p[0])),Math.min(...points.map(p=>p[1])),Math.max(...points.map(p=>p[0])),Math.max(...points.map(p=>p[1]))];
        const area=(box[2]-box[0])*(box[3]-box[1]);
        if(area<=0)continue;
        const existing=anchors.find(a=>{const b=a.pixelBox,intersection=Math.max(0,Math.min(b[2],box[2])-Math.max(b[0],box[0]))*Math.max(0,Math.min(b[3],box[3])-Math.max(b[1],box[1]));return intersection/Math.min(area,(b[2]-b[0])*(b[3]-b[1]))>.6;});
        if(existing){if(!existing.readings.includes(r.text))existing.readings.push(r.text);continue;}
        const rotation=box[3]-box[1]>60 && String(r.text).length>3 && box[3]-box[1]>(box[2]-box[0])*2 ? -90 : 0;
        anchors.push({id:`a${anchors.length}`,readings:[r.text],pixelBox:box,bbox:box.map((v,i)=>Math.max(0,Math.min(999,v/(i%2?source.height:source.width)*999))),rotation});
      }
    }
    return anchors;
  }
  function plainMath(text){
    const scripts="₀₁₂₃₄₅₆₇₈₉";
    return [...String(text)].map(c=>c.codePointAt(0)>=0x1d400 && c.codePointAt(0)<=0x1d7ff ? c.normalize("NFKC") : c).join("").replace(/[₀-₉]+/g,s=>`_{${[...s].map(c=>scripts.indexOf(c)).join("")}}`)
      .replace(/\$/g,"").replace(/\\mathcal\{F\}/g,"ℱ").replace(/\\(?:mathbf|mathit|mathrm|mathcal)\{([^{}]*)\}/g,"$1")
      .replace(/\\hat\{([^{}]+)\}/g,"$1\u0302").replace(/\\bar\{([^{}]+)\}/g,"$1\u0304")
      .replace(/\\times/g,"×").replace(/\\cdots/g,"⋯").replace(/\\[{}]/g,m=>m[1])
      .replace(/_([A-Za-z0-9]+)/g,"_{$1}").replace(/\^([A-Za-z0-9])/g,"^{$1}");
  }
  function anchorInventory(answer,anchors,photos,source){
    if(answer.corrections){
      if(!answer.corrections || typeof answer.corrections!=="object" || Array.isArray(answer.corrections))throw new Error("corrections 必须是以锚点 id 为键的对象");
      const missing=anchors.filter(a=>!Object.hasOwn(answer.corrections,a.id));
      const unknown=Object.keys(answer.corrections).filter(id=>!anchors.some(a=>a.id===id));
      if(missing.length || unknown.length)throw new Error(`文字校正 id 不完整，缺少：${missing.map(a=>a.id).join(",")}；未知：${unknown.join(",")}`);
      answer={...answer,labels:anchors.filter(a=>answer.corrections[a.id]?.text!==null).map(a=>({...answer.corrections[a.id],anchors:[a.id]})),ignored:anchors.filter(a=>answer.corrections[a.id]?.text===null).map(a=>({id:a.id,reason:answer.corrections[a.id].reason}))};
    }
    const used=new Set();
    const labels=(answer.labels || []).map((l,i)=>{
      if(!Array.isArray(l.anchors) || !l.anchors.length)throw new Error("校正文稿的每个标签必须引用检测锚点");
      const found=l.anchors.map(id=>{const a=anchors.find(a=>a.id===id);if(!a || used.has(id))throw new Error(`重复或不存在的文字锚点 ${id}`);used.add(id);return a;});
      const b=[Math.min(...found.map(a=>a.bbox[0])),Math.min(...found.map(a=>a.bbox[1])),Math.max(...found.map(a=>a.bbox[2])),Math.max(...found.map(a=>a.bbox[3]))];
      return {id:`labels-${i+1}`,text:plainMath(l.text),bbox:b,rotation:found[0].rotation,flags:l.flags || "",color:l.color || "#171717",anchored:true};
    });
    for(const item of answer.ignored || []){if(!anchors.some(a=>a.id===item.id) || !item.reason || used.has(item.id))throw new Error("忽略检测锚点需唯一 id 和原因");used.add(item.id);}
    const missing=anchors.filter(a=>!used.has(a.id));if(missing.length)throw new Error(`未处理 OCR 锚点 ${missing.map(a=>a.id).join(",")}`);
    const extra=(answer.extraLabels || []).map((l,i)=>({...l,id:`extra-${i+1}`,text:plainMath(l.text)}));
    const unique=[];
    for(const label of [...labels,...extra]){
      const duplicate=unique.find(old=>{
        if(normalize(old.text)!==normalize(label.text))return false;
        const a=old.bbox,b=label.bbox,intersection=Math.max(0,Math.min(a[2],b[2])-Math.max(a[0],b[0]))*Math.max(0,Math.min(a[3],b[3])-Math.max(a[1],b[1]));
        return intersection/Math.min((a[2]-a[0])*(a[3]-a[1]),(b[2]-b[0])*(b[3]-b[1]))>.35;
      });
      if(duplicate){duplicate.bbox=[Math.min(duplicate.bbox[0],label.bbox[0]),Math.min(duplicate.bbox[1],label.bbox[1]),Math.max(duplicate.bbox[2],label.bbox[2]),Math.max(duplicate.bbox[3],label.bbox[3])];}
      else unique.push(label);
    }
    return {coordinateSpace:"normalized_999",font:answer.font || "Times New Roman",background:answer.background || "#ffffff",labels:unique,
      photos:photos.map((p,i)=>({id:`photos-${i+1}`,bbox:[p.box[0]/source.width*999,p.box[1]/source.height*999,(p.box[0]+p.box[2])/source.width*999,(p.box[1]+p.box[3])/source.height*999]})),modules:answer.modules || []};
  }
  const ANCHOR_PROMPT = `请忠实校正科研图文字，不要重设计。宿主提供从原图及旋转图实测的文字锚点；锚点的位置和旋转由代码负责，你不要改坐标、合并锚点或重新分配编号。
只输出 JSON：{"font":"Times New Roman","background":"#ffffff","corrections":{"a0":{"text":"校正后的可见文字","flags":"bi","color":"#171717"},"a1":{"text":null,"reason":"这是图标或连线，不是文字"}},"extraLabels":[],"modules":[]}。
corrections 必须逐一包含所给每个 id，不能出现新的 id。真正文字填 text，图标、箭头、括号装饰或重复误检填 text:null 并解释 reason。保留原文字体（serif 用 Times New Roman），flags：b粗体/i斜体/bi/空串。不要把火焰/雪花图标改成 emoji；它们由矢量轮廓保留。纠正 OCR 对公式的误认，用普通拉丁字母和 _{下标}/^{上标}，不要使用数学花体 Unicode 字母；允许 ℱ、×、ẑ 等符号。不要输出 $ 或 LaTeX 命令。相邻片段不合并：如果一个锚点仅为下标，保留其字面小号文字而不是擅自添加基字符。
只有 OCR 真正漏掉的文字才放 extraLabels:[{text,bbox:[left,top,right,bottom],rotation,flags,color}]，bbox 必须为0–999归一化可见字形边界。准确读图，完整保留侧边旋转文字、重复标签、标题、图例及数学变量。`;
  async function prepare(options) {
    const { attachment, signal, config, render, onProgress = () => {} } = options;
    const complete = options.completion || global.UniPptAiRuntime.completion;
    const started = Date.now(), stages = [];
    const source = { type: "image_url", image_url: { url: attachment.dataUrl, detail: "high" } };
    const request = async (stage, prompt, content, maxTokens) => {
      if (signal?.aborted) throw signal.reason || new DOMException("Aborted", "AbortError");
      onProgress(stage); const at = Date.now();
      let text = "";
      let answer;
      try {
        answer = await complete({ config, signal, thinking: false, maxTokens, timeoutMs: 150000,
          messages: [{ role: "system", content: prompt }, { role: "user", content }],
          onToken: (token) => { text += token; onProgress(stage, `${(text.length/1024).toFixed(1)} KiB`); } });
      } catch(error) {
        stages.push({stage,ms:Date.now()-at,chars:text.length,completed:false});
        options.onStage?.({stage,ms:Date.now()-at,content:text,error:error.message});
        error.stages=clone(stages);
        throw error;
      }
      stages.push({ stage, ms: Date.now()-at, chars: answer.content?.length || 0 });
      options.onStage?.({ stage, ms: Date.now()-at, content: answer.content });
      return json(answer.content);
    };
    const dimensions = `The original image is ${attachment.width}x${attachment.height} pixels, but ALL model bbox/endpoint coordinates must use normalized 0–999, independent of image size. User request: ${options.prompt || "Faithful editable reconstruction"}`;
    const inventory = options.anchors?.length ? anchorInventory(await request("根据实测锚点校正文字与公式",ANCHOR_PROMPT,
      [source,...(options.anchorSheet?[{type:"image_url",image_url:{url:options.anchorSheet,detail:"high"}},{type:"text",text:"第二张图是按 id 放大的原图文字裁剪，旋转文字已转正便于阅读。逐格校对，不要把蓝色 id 写进标签。"}]:[]),{type:"text",text:dimensions+"\nAnchors: "+JSON.stringify(options.anchors.map(({id,readings,bbox,rotation})=>({id,readings,bbox,rotation})))}],7000),options.anchors,options.photos || [],attachment) : await request("逐项识别文字、旋转与模块", INVENTORY_PROMPT,
      [source, ...(options.orientationViews || []), { type: "text", text: dimensions + "\nOCR reference (same normalized xyxy coordinates): " + JSON.stringify(options.ocrDetection?.regions?.filter((r)=>r.location?.length===8).map((r) => ({ text:r.text,
        bbox:[Math.min(r.location[0],r.location[2],r.location[4],r.location[6])/attachment.width*999, Math.min(r.location[1],r.location[3],r.location[5],r.location[7])/attachment.height*999, Math.max(r.location[0],r.location[2],r.location[4],r.location[6])/attachment.width*999, Math.max(r.location[1],r.location[3],r.location[5],r.location[7])/attachment.height*999].map(Math.round) })) || []) }], 9500);
    // The compiler, not a model, owns bookkeeping identities. Namespaces also
    // prevent a label and its containing module from accidentally sharing IDs.
    for (const field of ["labels", "photos", "modules"]) inventory[field]?.forEach((item,i) => { item.id = `${field}-${i+1}`; });
    validateInventory(inventory, attachment);
    const plan = options.trace ? {shapes:[],lines:[]} : await request("构建原生几何与连接线", GEOMETRY_PROMPT+"\n"+PATH_SPACE_PROMPT,
      [source, { type: "text", text: dimensions + "\nInventory: " + JSON.stringify(inventory) }], 11000);
    let state = { inventory, plan }, args, review, preview;
    const reviewedScenes=new Set();
    for (let pass = 0; pass <= 2; pass++) {
      if(options.trace){
        onProgress("从原图恢复可编辑矢量轮廓");
        const pixels=state.inventory.coordinateSpace==="normalized_999" ? materialize({shapes:[],lines:[]},state.inventory,attachment).inventory : state.inventory;
        args=compilePlan(await options.trace(pixels),pixels,attachment);
      }else args = compilePlan(state.plan, state.inventory, attachment);
      options.preflight?.(args);
      const sceneKey=JSON.stringify(args.slide);
      if(reviewedScenes.has(sceneKey)){onProgress("修正未改变画面，停止重复复核");break;}
      reviewedScenes.add(sceneKey);
      onProgress("渲染并与原图对比", `第 ${pass+1} 次`);
      preview = await render(args);
      options.onPreview?.({pass,state:clone(state),args,preview,stages:clone(stages)});
      review = assertReview(await request(`视觉复核 ${pass+1}/3`, REVIEW_PROMPT + "\nGive only the final verdict, not deliberation. List at most 8 concrete differences and patch at most 12 changed items in total. Never repeat unchanged items or return the entire inventory. If more defects remain, fail and prioritize the most important repairs.\nGeometry schema:\n" + GEOMETRY_SCHEMA+"\n"+PATH_SPACE_PROMPT + (options.trace ? "\nGeometry is deterministically vector-traced from the source pixels. Do not patch shapes or lines. Correct labels and photo bbox/rotation instead; the source tracing will rerun after your patch. Small antialias/color-quantization differences are expected; assess whether all meaningful source content is preserved." : ""),
        [source, { type:"image_url", image_url:{url:preview,detail:"high"} }, { type:"text",text:dimensions+"\nState: "+JSON.stringify(options.trace ? {inventory:state.inventory,nativeGeometry:{shapes:args.slide.objects.filter(o=>o.kind==="shape").length,connectors:args.slide.objects.filter(o=>o.kind==="connector").length,source:"Already compiled and rendered from source pixels. Not model-authored. The second image shows these native objects."}} : state) }], 6500));
      options.onCheckpoint?.({ pass, state: clone(state), args, review, preview, stages: clone(stages) });
      if (review.passed) return { args, inventory: state.inventory, plan: state.plan, review, preview,
        report: { automatic: true, visualReview: "model-reviewed", humanReviewRequired: true, stages, totalMs: Date.now()-started } };
      if (pass < 2 && Object.values(review.patch || {}).some((v) => v.length)) state = patchState(state, review.patch);
      else break;
    }
    const error = new Error(`自动视觉复核未通过，未提交页面：${(review?.issues || []).join("；")}`);
    error.review = review; error.state = state; error.args = args; error.preview = preview; error.stages = stages;
    throw error;
  }
  const esc = (v) => String(v ?? "").replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;");
  function previewSvg(args) {
    const { width, height } = args.sourceSize;
    const compiled = global.UniPptPresentationHost.compileNativeImageSlide(args, { width,height });
    const body = compiled.slide.objects.map((o, index) => {
      const f = o.frame, s = o.style || {}, t = o.textStyle || {};
      const transform = `translate(${f.x+f.width/2} ${f.y+f.height/2}) rotate(${f.rotation || 0}) translate(${-f.width/2} ${-f.height/2})`;
      const fill = s.fill === "transparent" ? "none" : s.fill || "none";
      const style = `fill="${esc(fill)}" stroke="${esc(s.stroke || "none")}" stroke-width="${s.strokeWidth || 0}"${s.strokeDash ? ' stroke-dasharray="7 6"' : ""}`;
      let inner = "";
      if (o.kind === "text") {
        const anchor = t.align === "left" ? "start" : t.align === "right" ? "end" : "middle";
        const x = t.align === "left" ? 0 : t.align === "right" ? f.width : f.width/2;
        const paragraphs = String(o.text || "").split("\n");
        const rich = o.textParagraphs?.[0]?.runs;
        const text = paragraphs.length === 1 && rich?.length ? rich.map((r) => `<tspan${r.baseline === "sub" ? ' baseline-shift="sub"' : r.baseline === "super" ? ' baseline-shift="super"' : ""} font-size="${r.fontSize || t.fontSize}"${r.italic ? ' font-style="italic"' : ""}>${esc(r.text)}</tspan>`).join("") : paragraphs.map((line,i) => `<tspan x="${x}" dy="${i ? t.fontSize*1.1 : 0}">${esc(line)}</tspan>`).join("");
        inner = `<text x="${x}" y="${f.height/2+t.fontSize*.34-(paragraphs.length-1)*t.fontSize*.55}" text-anchor="${anchor}" font-family="${esc(t.fontFamily)}" font-size="${t.fontSize}" font-weight="${t.bold ? "bold" : "normal"}" font-style="${t.italic ? "italic" : "normal"}" fill="${esc(t.color)}">${text}</text>`;
      } else if (o.kind === "image") {
        const c = o.imageCrop, iw = f.width/(1-c.left-c.right), ih = f.height/(1-c.top-c.bottom);
        inner = `<defs><clipPath id="photo-${index}"><rect width="${f.width}" height="${f.height}"/></clipPath></defs><image clip-path="url(#photo-${index})" x="${-iw*c.left}" y="${-ih*c.top}" width="${iw}" height="${ih}" href="${esc(o.asset)}"/>`;
      } else if (o.customGeometry) {
        const g = o.customGeometry;
        inner = `<svg width="${f.width}" height="${f.height}" viewBox="0 0 ${g.width} ${g.height}" overflow="visible"><path d="${esc(g.pathData)}" ${style.replace(/stroke-width="[^"]*"/, `stroke-width="${(s.strokeWidth || 0)*g.width/f.width}"`)}/></svg>`;
      } else if (o.kind === "connector" || o.geometry === "line") {
        inner = `<line x1="${o.flipH ? f.width : 0}" y1="${o.flipV ? f.height : 0}" x2="${o.flipH ? 0 : f.width}" y2="${o.flipV ? 0 : f.height}" ${style}/>`;
      } else if (o.geometry === "ellipse") inner = `<ellipse cx="${f.width/2}" cy="${f.height/2}" rx="${f.width/2}" ry="${f.height/2}" ${style}/>`;
      else inner = `<rect width="${f.width}" height="${f.height}" ${style}/>`;
      return `<g transform="${transform}">${inner}</g>`;
    }).join("");
    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}"><rect width="100%" height="100%" fill="${esc(compiled.slide.background)}"/>${body}</svg>`;
  }
  async function renderPreview(args) {
    const svg = previewSvg(args), url = URL.createObjectURL(new Blob([svg], { type:"image/svg+xml" }));
    try {
      const image = new Image();
      await new Promise((resolve,reject) => { image.onload=resolve; image.onerror=() => reject(new Error("重建预览渲染失败")); image.src=url; });
      const canvas = document.createElement("canvas");
      canvas.width=args.sourceSize.width; canvas.height=args.sourceSize.height;
      canvas.getContext("2d").drawImage(image,0,0);
      return canvas.toDataURL("image/png");
    } finally { URL.revokeObjectURL(url); }
  }
  async function detectAnchors(attachment,detect,options={}){
    const image=new Image();await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=reject;image.src=attachment.dataUrl;});
    const canvas=document.createElement("canvas");canvas.width=attachment.height;canvas.height=attachment.width;
    const ctx=canvas.getContext("2d");ctx.translate(canvas.width,0);ctx.rotate(Math.PI/2);ctx.drawImage(image,0,0);
    const rotated={...attachment,width:canvas.width,height:canvas.height,dataUrl:canvas.toDataURL("image/png")};
    const timeoutMs=options.timeoutMs ?? 45000;
    const results=await Promise.allSettled([detect(attachment,{...options,detectOnly:true,timeoutMs}),detect(rotated,{...options,detectOnly:true,timeoutMs})]);
    if(options.signal?.aborted)throw new DOMException("Aborted","AbortError");
    const detections=results.map((r,i)=>r.status==="fulfilled"?{...r.value,rotation:i*90}:{regions:[],rotation:i*90,warning:r.reason?.message});
    return { ...detections[0], detections, anchors:makeAnchors(detections,attachment), warnings:detections.filter(d=>d.warning).map(d=>d.warning) };
  }
  global.UniPptImageReconstruction = Object.freeze({ prepare, compilePlan, validateInventory, materialize, makeAnchors, anchorInventory, plainMath, patchState, assertReview, runs, normalize, previewSvg, renderPreview, detectAnchors, prompts: { inventory: INVENTORY_PROMPT, geometry: GEOMETRY_PROMPT, review: REVIEW_PROMPT } });
})(globalThis);
