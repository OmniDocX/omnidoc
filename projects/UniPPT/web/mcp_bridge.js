(function (global) {
  'use strict';
  const sceneApi = () => global.UniPptMcpScene || (typeof require === 'function' ? require('./mcp_scene.js') : null);
  const animationApi = () => global.UniPptMcpAnimation || (typeof require === 'function' ? require('./mcp_animation.js') : null);
  const ENDPOINT = 'http://127.0.0.1:8142/browser/';
  const ALLOWED = new Set(['updateText', 'updateObject', 'addObject', 'removeObject', 'reorderObject', 'updateSlide', 'insertSlide', 'removeSlide', 'moveSlide', 'setTitle', 'setTransition', 'addAnimation', 'updateAnimation', 'removeAnimation']);
  const EXPORTS=Object.freeze({pptx:['/api/export-pptx','application/vnd.openxmlformats-officedocument.presentationml.presentation','pptx'],udoc:['/api/export-udoc','application/vnd.unidoc','udoc'],html:['/api/export-html','text/html','html'],pdf:['/api/export-pdf','application/pdf','pdf'],images:['/api/export-images','application/zip','zip'],video:['/api/export-video','video/mp4','mp4'],'video-fast':['/api/export-video-fast','video/mp4','mp4']});
  function capabilities(){return {version:'1.2.0',modelCalls:false,documentScoped:true,operations:[...ALLOWED],exportFormats:Object.keys(EXPORTS),
    features:{nativeObjects:['text','shape','image','group','table','chart','math','media'],text:'rich runs, styles, rotation, paragraph alignment',geometry:'frames, native paths, module layers/arrows',slides:'add/remove/reorder, background, notes, title',motion:'object animations and slide transitions; apply_timeline batch protocol',imageReconstruction:'begin_image / apply_image_layout / finish_image',validation:'structural checks and exact native preview'},
    animationSchema: animationApi().schema(),
    boundaries:[{feature:'import PPTX/UDOC/HTML, media upload, recording and playback UI',access:'Use browser file import or controls; no arbitrary path/URL/script execution tool.'},{feature:'built-in AI/OCR/inpainting',access:'Local U AI uses your configured model provider; MCP uses the client model and never calls the AI proxy.'}],
    documentation:'/mcp-protocol.html',limits:{patchOperations:100,sceneOperations:512,modules:64,patchCharacters:1000000,exportBytes:20000000,commandSeconds:60},
    warning:'Capability exposure does not imply every input/format passes visual fidelity. Long exports may exceed the bounded MCP command deadline; use the web export UI.'};}
  function validatePatch(operations, limit=100) {
    if (!Array.isArray(operations) || !operations.length || operations.length > limit) throw new Error('MCP 批量操作超过上限 '+limit);
    if (JSON.stringify(operations).length > 1000000) throw new Error('MCP 修改超过 1 MB');
    const check = (v, depth = 0) => {
      if (depth > 30) throw new Error('对象嵌套过深');
      if (typeof v === 'string' && /(?:https?:|file:|javascript:|data:|<script\b)/i.test(v)) throw new Error('MCP 局部修改不接收 URL、脚本或二进制；请在网页导入素材');
      if (v && typeof v === 'object') for (const [k, child] of Object.entries(v)) {
        if (['__proto__', 'prototype', 'constructor'].includes(k)) throw new Error('非法对象字段');
        check(child, depth + 1);
      }
    };
    for (const op of operations) { if (!ALLOWED.has(op?.op)) throw new Error('不支持的 MCP 操作：' + op?.op); check(op); }
    return operations;
  }
  function publicValue(value) {
    return JSON.parse(JSON.stringify(value, (key, v) => {
      if (typeof v === 'string' && (v.startsWith('data:') || v.length > 80000)) return {omitted: true, characters: v.length, reason: 'binary or oversized field'};
      return v;
    }));
  }
  function selectSlide(deck, id) { const slide = deck.slides.find(s => s.id === id); if (!slide) throw new Error('页面 ID 不存在'); return slide; }
  function allObjects(objects, output = []) { for (const o of objects || []) { output.push(o); if (o.children) allObjects(o.children, output); } return output; }
  // Raster bytes are explicit user-provided document material, never URLs or code.
  // Restrict this protocol extension to small, dimension-bounded PNGs.
  function nativeObject(source, repairs=[]) {
    const object = sceneApi().normalizeObject(textPatch({}, source || {}), repairs);
    if (object.imageData != null) {
      const data = object.imageData;
      if (object.kind !== 'image' || object.asset != null || data.mimeType !== 'image/png' ||
          typeof data.base64 !== 'string' || data.base64.length > 750000 ||
          !/^[A-Za-z0-9+/]+={0,2}$/.test(data.base64) || data.base64.length % 4) throw new Error('imageData 仅接收有界 PNG 图片');
      const raw = atob(data.base64), signature = [137,80,78,71,13,10,26,10];
      const uint32 = offset => ((raw.charCodeAt(offset) * 16777216) + (raw.charCodeAt(offset+1) << 16) + (raw.charCodeAt(offset+2) << 8) + raw.charCodeAt(offset+3));
      if (raw.length < 45 || signature.some((v,i) => raw.charCodeAt(i) !== v) || raw.slice(12,16) !== 'IHDR' || uint32(8) !== 13) throw new Error('PNG 文件头无效');
      const width = uint32(16), height = uint32(20);
      if (!(width > 0 && height > 0 && width <= 4096 && height <= 4096 && width * height <= 6000000)) throw new Error('PNG 尺寸超过限制');
      object.asset = 'data:image/png;base64,' + data.base64;
      delete object.imageData;
    }
    if (object.children) object.children = object.children.map(o=>nativeObject(o,repairs));
    return object;
  }
  function textPatch(original, source) {
    const patch = structuredClone(source), style = {...original.textStyle, ...patch.textStyle};
    if (patch.textStyle?.fontFamily && !Object.hasOwn(patch.textStyle, 'nativeFontFamily')) patch.textStyle.nativeFontFamily = null;
    if (patch.textRuns) {
      if (!Array.isArray(patch.textRuns) || !patch.textRuns.length) throw new Error('textRuns 必须为非空数组');
      const text = patch.textRuns.map(r => String(r.text ?? '')).join('');
      if (patch.text != null && patch.text !== text) throw new Error('text 必须与 textRuns 拼接结果一致');
      patch.text = text;
      const paragraphs = [{align: style.align || 'left', runs: []}];
      for (const r of patch.textRuns) {
        if (r.baseline && !['normal', 'sub', 'super'].includes(r.baseline)) throw new Error('baseline 无效');
        if(r.baselineOffset!==undefined&&(typeof r.baselineOffset!=='number'||!Number.isFinite(r.baselineOffset)||Math.abs(r.baselineOffset)>100))throw Error('baselineOffset must be -100–100 percent');
        String(r.text ?? '').split('\n').forEach((part, index) => {
          if (index) paragraphs.push({align: style.align || 'left', runs: []});
          paragraphs.at(-1).runs.push({fontFamily: style.fontFamily || 'Aptos', fontSize: (style.fontSize || 28) * (r.baseline && r.baseline !== 'normal' ? .65 : 1),
            color: style.color || '#172033', bold: !!style.bold, italic: !!style.italic, underline: false, strike: false,
            baseline: 'normal', hyperlinks: {click: null, hover: null}, ...r, text: part});
        });
      }
      patch.textParagraphs = paragraphs; delete patch.textRuns;
    } else if (patch.text != null && !patch.textParagraphs) {
      return textPatch(original, {...patch, textRuns: [{text: String(patch.text)}]});
    } else if (patch.textParagraphs) {
      patch.text = patch.textParagraphs.map(p => p.runs.map(r => r.text).join('')).join('\n');
    } else if (patch.textStyle && original.textParagraphs?.length) {
      patch.textParagraphs = structuredClone(original.textParagraphs).map(p => ({...p,
        ...(patch.textStyle.align ? {align: patch.textStyle.align} : {}),
        runs: p.runs.map(r => {
          const copy = {...r};
          for (const key of ['fontFamily', 'color', 'bold', 'italic', 'underline', 'strike']) if (Object.hasOwn(patch.textStyle, key)) copy[key] = patch.textStyle[key];
          if (patch.textStyle.fontFamily) copy.nativeFontFamily = null;
          if (patch.textStyle.fontSize) copy.fontSize = (r.fontSize || original.textStyle?.fontSize || 28) * patch.textStyle.fontSize / (original.textStyle?.fontSize || 28);
          return copy;
        }),
      }));
    }
    return patch;
  }
  function preparePatch(deck, operations, repairs=[], limit=100) {
    validatePatch(operations,limit);
    const textShadows = new Map();
    const remember = (original, patch) => {
      textShadows.set(original.id, {...original, ...patch, textStyle: {...original.textStyle, ...patch.textStyle}});
      return patch;
    };
    return operations.map(op => {
      if (op.op === 'updateObject') {
        if (['id', 'sourceShapeId'].some(key => Object.hasOwn(op.patch || {}, key))) throw new Error('不能改写对象标识；请使用 addObject/removeObject');
        const slides = op.slideId ? [selectSlide(deck, op.slideId)] : deck.slides;
        const found = textShadows.get(op.objectId) || slides.flatMap(s => allObjects(s.objects)).find(o => o.id === op.objectId);
        if (!found) throw new Error('对象 ID 不存在：' + op.objectId);
        const normalized=sceneApi().normalizeObject(textPatch(found, op.patch || {}),repairs,found);
        return {...op, patch: remember(found,normalized.imageData?nativeObject(normalized,repairs):normalized)};
      }
      if (op.op === 'updateText') {
        const found = textShadows.get(op.objectId) || deck.slides.flatMap(s => allObjects(s.objects)).find(o => o.id === op.objectId);
        if (found) remember(found, textPatch(found, {text: op.text}));
      }
      if (op.op === 'addObject') return {...op, object: nativeObject(op.object,repairs)};
      if (op.op === 'insertSlide' && op.slide?.objects) return {...op, slide: {...op.slide, objects: op.slide.objects.map(o=>nativeObject(o,repairs))}};
      return op;
    });
  }
  function create({host, getDeck, getSourceImage = () => null, undo, onStatus = () => {}}) {
    const scene=sceneApi();
    const endpoint = ENDPOINT;
    let credentials = null, version = 0, timer = null, lastMcpRevision = null, generation = 0;
    let autoConnect=false,connecting=null,retryTimer=null,retryMs=1500,pollingEpoch=null;
    const previewCache=new Map();let warmup={state:'not_requested'};
    const imageJobs=global.UniPptMcpImageJobs?.create();
    const prewarm=()=>{warmup={state:'requested'};return fetch('/api/mcp/prewarm',{method:'POST',headers:{'Content-Type':'application/json'},body:'{}',signal:AbortSignal.timeout(5000)}).then(r=>r.json()).then(r=>{warmup=r;}).catch(()=>{warmup={state:'unavailable',blocking:false};});};
    const post = async (route, body, signal) => {
      const r = await fetch(endpoint + route, {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body), signal: signal || AbortSignal.timeout(10000)});
      const result = await r.json(); if (!r.ok) throw Object.assign(new Error(result.error || 'MCP 连接失败'),{status:r.status}); return result;
    };
    const envelope = (result, revision = version) => ({sessionId: credentials?.sessionId, revision, ...result});
    const guard = args => { if (!Number.isInteger(args.expectedRevision) || args.expectedRevision !== version) throw new Error(`REVISION_CONFLICT：期望 ${args.expectedRevision}，当前 ${version}；请重新 inspect`); };
    host.on('change', () => { version++; previewCache.clear(); });
    host.on('load', () => { void disconnect('文稿已切换，旧 MCP 会话已关闭',true).then(()=>{if(autoConnect)void connectAutomatically();}); });
    async function execute(command) {
      const {method, args = {}, deadline} = command;
      if (!credentials || Date.now() >= deadline) throw new Error('命令已过期或连接已撤销');
      const deck = getDeck(), capturedRevision = version;
      if(method==='get_capabilities')return envelope(capabilities());
      if(method==='validate'){guard(args);return envelope({issues:host.document.validate(),slides:deck.slides.map(s=>({slideId:s.id,...scene.checkSlide(s,deck.width,deck.height)})),visualFidelityPassed:false});}
      if(['begin_image','apply_image_layout','finish_image'].includes(method)){
        if(!imageJobs)throw Error('Image MCP runtime unavailable; save the document and reload the latest editor');
        if(method==='begin_image'){
          const epoch=generation,result=await imageJobs.begin({...args,requestId:command.requestId},{document:host.document.getSummary({}),revision:version,writeAccess:credentials.writeAccess});
          if(!credentials||generation!==epoch||version!==capturedRevision)throw Error('Document changed during begin_image; begin again');
          return envelope(result);
        }
        if(method==='finish_image')return envelope(imageJobs.finish(args,version));
        guard(args);
        const record=await imageJobs.plan({...args,requestId:command.requestId},version);
        if(record.result)return record.result;
        const result=await execute({...command,method:'apply_scene',args:record.command});
        return imageJobs.committed(record,result);
      }
      if (method === 'get_module_schema') return envelope({schema:scene.moduleSchema()});
      if (method === 'get_animation_schema') return envelope({schema:animationApi().schema()});
      if (method === 'apply_timeline') {
        guard(args);
        if (!credentials.writeAccess) throw new Error('当前 MCP 会话为只读');
        const beforeIds = new Set((selectSlide(deck, args.slideId).animations || []).map(a => a.id));
        const compiled = animationApi().compile(deck, args);
        const result = await execute({...command, method:'apply_patch', args:{...args, operations:compiled.operations, preview:args.preview, includePptx:args.includePptx}});
        const after = selectSlide(getDeck(), args.slideId), added = (after.animations || []).filter(a => !beforeIds.has(a.id));
        compiled.generated = compiled.generated.map((entry,index) => ({...entry, animationId:added[index]?.id || null}));
        return {...result, timeline:compiled, animationSchemaVersion:'1.0.0'};
      }
      if (method === 'apply_scene' && !args.compiledModules && args.modules) {
        args.compiledModules = scene.compileModules(args.modules);
        delete args.modules;
      }
      if(method==='prepare'){
        await prewarm();
        if(!credentials||capturedRevision!==version)throw Error('Context changed during prepare; inspect again');
        const result={document:host.document.getSummary({}),selection:host.selection.get(),writeAccess:credentials.writeAccess,moduleSchema:scene.moduleSchema(),warmup};
        if(args.slideId){selectSlide(deck,args.slideId);const source=getSourceImage(args.slideId),match=/^data:(image\/(?:png|jpeg|webp));base64,(.+)$/.exec(source?.dataUrl||'');
          if(match)Object.assign(result,{imageBase64:match[2],mimeType:match[1],source:{name:source.name,width:source.width,height:source.height,referenceOnly:true}});
          else result.source={available:false,instruction:'Provide the original image; no retained source exists for this slide.'};}
        return envelope(result);
      }
      if (method === 'source_image') {
        selectSlide(deck, args.slideId);
        const source = getSourceImage(args.slideId), match = /^data:(image\/(?:png|jpeg|webp));base64,(.+)$/.exec(source?.dataUrl || '');
        if (!match) throw new Error('此页没有本次浏览器会话保留的原图，请提供原始图片（刷新页面会清空原图引用）');
        return envelope({imageBase64: match[2], mimeType: match[1], name: source.name, width: source.width, height: source.height, referenceOnly: true});
      }
      if (method === 'inspect') return envelope({document: host.document.getSummary(args), selection: host.selection.get(), writeAccess: credentials.writeAccess});
      if (method === 'get_edit_schema') {
        const schema = structuredClone(host.ai.tools.find(t => t.name === 'presentation.applyChangeSet').inputSchema);
        const op = schema.properties.operations.items.properties?.op;
        if (op?.enum) op.enum = op.enum.filter(name => ALLOWED.has(name));
        if (schema.properties.operations.items.oneOf) schema.properties.operations.items.oneOf = schema.properties.operations.items.oneOf.filter(item => ALLOWED.has(item.properties.op.enum[0]));
        schema.description = (schema.description || '') + ' MCP PNG extension: insertSlide/addObject image objects may supply imageData {mimeType:"image/png",base64:string}; max 750000 base64 characters, 4096 pixels per side, 6 megapixels, within the 1 MB batch. No file paths, URLs, SVG or model calls. Nested native objects and textRuns are normalized.';
        return envelope({schema, allowedOperations: [...ALLOWED], notes: ['Use bridge revision as expectedRevision, not baseRevision.', 'No external URLs/scripts/file paths. Bounded PNG imageData is allowed only in inserted image objects. Use native textRuns for subscripts.', 'updateText changes plain text; use updateObject.patch with text/textRuns for styled formulas. textStyle patches also update paragraph runs.', 'For freeform shapes use customGeometry {width,height,pathData}; addObject does not compile image-plan path/points/repeat fields.']});
      }
      if (['get_slide', 'get_objects'].includes(method)) {
        const slide = selectSlide(deck, args.slideId), objects = allObjects(slide.objects);
        if (method === 'get_objects') {
          if (!Array.isArray(args.objectIds) || args.objectIds.length < 1 || args.objectIds.length > 20) throw new Error('需要 1–20 个对象 ID');
          return envelope({slideId: slide.id, objects: args.objectIds.map(id => {const found = objects.find(o => o.id === id); if (!found) throw new Error('对象 ID 不存在：' + id); return publicValue(found);})});
        }
        const offset = args.offset ?? 0, limit = args.limit ?? 40;
        if (!Number.isInteger(offset) || offset < 0 || !Number.isInteger(limit) || limit < 1 || limit > 100) throw new Error('分页参数无效');
        const {objects: ignored, ...properties} = slide;
        return envelope({pageSize: {width: deck.width, height: deck.height}, slide: publicValue(properties), totalObjects: slide.objects.length,
          objects: publicValue(slide.objects.slice(offset, offset + limit)), nextOffset: offset + limit < slide.objects.length ? offset + limit : null});
      }
      if (['apply_patch', 'apply_scene', 'undo'].includes(method)) {
        const started=performance.now();
        guard(args);
        if (!credentials.writeAccess) throw new Error('当前 MCP 会话为只读');
        if (document.activeElement?.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName || '')) throw new Error('用户正在编辑输入框，请稍后重新 inspect');
        if (method === 'undo') {
          if (lastMcpRevision !== version) throw new Error('MCP 修改后已有其他编辑，拒绝撤销用户操作');
          undo(); lastMcpRevision = null; return envelope({undone: true});
        }
        const repairs=[],operations = preparePatch(deck,method==='apply_scene'?scene.sceneOperations(deck,args):args.operations,repairs,method==='apply_scene'?512:100);
        const prepared=performance.now();
        // The synchronous host transaction validates the entire cloned deck
        // before committing through the editor's existing undo machinery.
        const result = await host.document.transact({baseRevision: host.document.getRevision(), operations});
        lastMcpRevision = version;
        const committed=performance.now(),writtenRevision=version,updated=selectSlide(getDeck(),result.activeSlideId),checks=scene.checkSlide(updated,getDeck().width,getDeck().height);
        const summary={writeCommitted:true,includePptx:args.includePptx===true,applied:result.applied,transactionId:result.transactionId,issues:result.issues,repairs,checks,
          activeSlideId:result.activeSlideId,selectedObjectId:result.selectedObjectId,undoable:true,
          timings:{compileMs:args.compileMs||0,prepareMs:prepared-started,commitMs:committed-prepared}};
        if(method==='apply_scene'||args.preview===true){
          if(method==='apply_scene')summary.moduleMap=args.compiledModules.map(m=>({moduleId:m.id,objectIds:m.objects.map(o=>o.id)}));
          if(args.preview!==false){try{const writeTimings=summary.timings,rendered=await renderBundle(updated.id,writtenRevision,deadline);Object.assign(summary,rendered);summary.timings={...writeTimings,...rendered.timings,totalBrowserMs:performance.now()-started};summary.staleAfterRender=version!==writtenRevision;}
            catch(error){summary.previewError=error.message;summary.nativeRendered=false;summary.visualFidelityPassed=false;summary.retry='Write committed. Use preview at returned revision; do not replay the write with a new ID.';}}
        }
        return envelope(summary,writtenRevision);
      }
      if (['preview', 'export_pptx', 'export'].includes(method)) {
        guard(args);
        if(method==='preview'){const result=await renderBundle(args.slideId,capturedRevision,deadline);imageJobs?.previewed(args,result);return envelope({...result,includePptx:args.includePptx===true},capturedRevision);}
        const selected = globalThis.UniPptPresentationHost.prepareNativeExport(args.slideId ? {...deck, slides: [selectSlide(deck, args.slideId)]} : deck);
        if (method === 'preview' && !args.slideId) throw new Error('渲染需要一个明确页面 ID');
        const body = JSON.stringify({deck: selected});
        if (body.length > 24000000) throw new Error('导出文稿超过 MCP 24 MB 上限，请在网页导出');
        const format=method==='export' ? EXPORTS[args.format] : EXPORTS.pptx;
        if(!format)throw Error('不支持的导出格式；请读取 get_capabilities');
        const response = await fetch(format[0], {
          method: 'POST', headers: {'Content-Type': 'application/json'}, body,
          signal: AbortSignal.timeout(Math.max(1, deadline - Date.now() - 1000)),
        });
        if (!response.ok) throw new Error((await response.text()).slice(0, 700));
        const blob = await response.blob();
        if (blob.size > 20000000) throw new Error('MCP 导出结果超过 20 MB，请在网页下载');
        const dataUrl = await new Promise((resolve, reject) => {const reader = new FileReader(); reader.onload = () => resolve(reader.result); reader.onerror = reject; reader.readAsDataURL(blob);});
        return envelope({[method==='export'?'fileBase64':'pptxBase64']: dataUrl.split(',')[1],
          ...(method==='export'?{mimeType:format[1],extension:format[2],format:args.format}:{}),
          slideIds: selected.slides.map(s => s.id), audit: method === 'preview' ? JSON.parse(response.headers.get('X-UniPPT-Native-Audit') || 'null') : undefined,
          nativeRendered: method === 'preview', visualFidelityPassed: false}, capturedRevision);
      }
      throw new Error('未知 MCP 命令');
    }
    async function renderBundle(slideId,revision,deadline){
      const key=revision+':'+slideId,cached=previewCache.get(key);if(cached)return {...cached,cacheHit:true,cachedRenderTimings:cached.timings,timings:{cacheLookupMs:0}};
      const started=performance.now(),epoch=generation,deck=getDeck(),selected=globalThis.UniPptPresentationHost.prepareNativeExport({...deck,slides:[selectSlide(deck,slideId)]}),body=JSON.stringify({deck:selected});
      if(body.length>24000000)throw Error('MCP rendering exceeds 24 MB');
      const response=await fetch('/api/mcp/render-bundle',{method:'POST',headers:{'Content-Type':'application/json'},body,signal:AbortSignal.timeout(Math.max(1,deadline-Date.now()-1000))});
      if(!response.ok)throw Error((await response.text()).slice(0,700));const material=await response.text();if(material.length>30000000)throw Error('Render bundle exceeds 30 MB');
      const data=JSON.parse(material);data.timings={...data.timings,renderRequestMs:performance.now()-started};data.cacheHit=false;
      if(version===revision&&credentials&&generation===epoch){previewCache.clear();previewCache.set(key,data);}return data;
    }
    async function poll(epoch) {
      if (!credentials || epoch !== generation || pollingEpoch===epoch) return;
      pollingEpoch=epoch;clearTimeout(timer);
      const auth = {...credentials};
      try {
        const deck = getDeck(), result = await post('poll', {...auth, revision: version, title: deck.title, slideCount: deck.slides.length, activeSlideId: host.selection.get().slideId});
        if (epoch !== generation) return;
        if (result.command) {
          let output;
          try { output = {result: await execute(result.command)}; } catch (error) { output = {error: error.message}; }
          if (epoch === generation) await post('result', {...auth, requestId: result.command.requestId, ...output});
        }
      } catch (error) { if (epoch === generation) {await disconnect('MCP 连接中断：' + error.message,true);if(autoConnect)scheduleReconnect();} }
      if(pollingEpoch===epoch)pollingEpoch=null;
      if (credentials && epoch === generation) timer = setTimeout(() => void poll(epoch), 350);
    }
    async function disconnect(reason = 'MCP 已断开',preserveAuto=false) {
      if(!preserveAuto)autoConnect=false;clearTimeout(retryTimer);
      const auth = credentials; credentials = null; generation++; clearTimeout(timer); lastMcpRevision = null; previewCache.clear(); imageJobs?.clear(); onStatus(false, reason);
      if (auth) await post('disconnect', auth).catch(() => {});
    }
    function scheduleReconnect(){clearTimeout(retryTimer);if(autoConnect)retryTimer=setTimeout(()=>void connectAutomatically(),retryMs);retryMs=Math.min(15000,retryMs*2);}
    async function openSession(){
      if(credentials)return;
      const epoch=++generation;
      const connected=await post('connect',{title:getDeck().title,writeAccess:true});
      if(epoch!==generation){await post('disconnect',connected).catch(()=>{});return;}
      credentials={sessionId:connected.sessionId,secret:connected.secret,writeAccess:true};version=0;retryMs=1500;
      onStatus(true,autoConnect?'本机 MCP 全局已开启，可随时关闭':'MCP 已连接，可修改当前文稿');
      prewarm();void poll(epoch);
    }
    async function connectAutomatically(){
      if(!autoConnect||credentials)return;if(connecting)return connecting;
      connecting=openSession().catch(error=>{onStatus(false,'本机 MCP 等待连接：'+error.message);}).finally(()=>{connecting=null;if(autoConnect&&!credentials)scheduleReconnect();});
      return connecting;
    }
    global.document?.addEventListener?.('visibilitychange',()=>{
      if(global.document.visibilityState==='visible'){if(credentials)void poll(generation);else if(autoConnect)void connectAutomatically();}
    });
    return Object.freeze({
      async setAutoConnect(enabled){
        const local=['http://127.0.0.1:8141','http://localhost:8141'].includes(global.location?.origin);
        const eligible=local;
        autoConnect=eligible&&enabled===true;
        if(!autoConnect)return disconnect('本机 MCP 全局已关闭');
        return connectAutomatically();
      },
      async connect(event) {
        if (!event?.isTrusted) throw new Error('MCP 连接必须由真实点击开启');
        if (credentials) return disconnect();
        if(!['http://127.0.0.1:8141','http://localhost:8141'].includes(global.location?.origin))throw Error('MCP 仅支持本机编辑器');
        if (!confirm('允许本机 Codex / MCP 读取并修改当前文稿吗？\n修改可撤销；切换文稿或再次点击 MCP 会断开。\n不会分享 AI 密钥，也不会连接远程 MCP 服务。')) return;
        return openSession();
      },
      disconnect,
    });
  }
  global.UniPptMcpBridge = Object.freeze({create, validatePatch, publicValue, preparePatch, textPatch, capabilities});
  if (typeof module !== 'undefined') module.exports = global.UniPptMcpBridge;
})(globalThis);
