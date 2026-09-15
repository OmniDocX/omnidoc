use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{Read, Write};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use thiserror::Error;
use unippt_core::{Deck, SceneObject};

use crate::asset_transport::{externalize_deck_assets, AssetCatalog};
use crate::opc_snapshot::{self, Blob, OpcPackageIndex, OpcSnapshot};
use crate::udoc::{is_supported_unidoc_type, CURRENT_UNIDOC_TYPE};

const MARKER_OPEN: &str = r#"<script type="application/x-unippt+json" id="unippt-data">"#;
const MARKER_CLOSE: &str = "</script>";
const MAX_PPTX_SIZE: usize = 512 * 1024 * 1024;
const MAX_HTML_ASSET_COUNT: usize = 16_384;
const MAX_HTML_ASSET_BYTES: usize = 512 * 1024 * 1024;
const PORTABLE_ASSET_PREFIX: &str = "unippt-asset:";

// This block is injected after the compact template's primitive player
// functions. Function declarations are hoisted and the later declarations
// intentionally replace the primitive implementations. Keeping the complete
// stepping state machine together makes the exported HTML follow the same
// PowerPoint trigger semantics as the editor without adding an external
// runtime dependency.
const LOSSLESS_PLAYER_RUNTIME: &str = r#"
function assetBytes(encoded){const raw=atob(String(encoded||'')),bytes=new Uint8Array(raw.length);for(let i=0;i<raw.length;i++)bytes[i]=raw.charCodeAt(i);return bytes}
function resolveDeckAssets(deck,assets,blobs){const index=new Map((assets||[]).map(a=>[a.id,a])),blobIndex=new Map((blobs||[]).map(b=>[b.path,b]));function source(a){const path=a.browserBlob||a.originalBlob;if(path){const blob=blobIndex.get(path);if(!blob)throw new Error('missing UniPPT blob: '+path);return blob}return{base64:a.browserBase64||a.originalBase64}}function url(id){if(assetObjectUrls.has(id))return assetObjectUrls.get(id);const a=index.get(id);if(!a)throw new Error('missing UniPPT asset: '+id);const mime=a.browserMimeType||a.mimeType||'application/octet-stream',src=source(a),value=src.url?new URL(src.url,document.baseURI).href:URL.createObjectURL(new Blob([assetBytes(src.base64)],{type:mime}));assetObjectUrls.set(id,value);trustedAssetUrls.add(value);return value}function one(value){const text=String(value||''),prefix='unippt-asset:';return text.startsWith(prefix)?url(text.slice(prefix.length)):value}function fill(value){return String(value||'').replace(/unippt-asset:[0-9a-f]{64}/gi,m=>one(m))}function object(o){if(!o)return;o.asset=one(o.asset);o.shapeFillAsset=one(o.shapeFillAsset);if(o.style)o.style.fill=fill(o.style.fill);if(o.media){o.media.asset=one(o.media.asset);o.media.playbackAsset=one(o.media.playbackAsset)}(o.children||[]).forEach(object)}for(const f of deck.fonts||[])f.dataUri=one(f.dataUri);for(const slide of deck.slides||[]){slide.backgroundAsset=one(slide.backgroundAsset);[...(slide.masterObjects||[]),...(slide.layoutObjects||[]),...(slide.objects||[])].forEach(object)}addEventListener('pagehide',()=>{for(const value of assetObjectUrls.values())if(String(value).startsWith('blob:'))URL.revokeObjectURL(value);assetObjectUrls.clear();trustedAssetUrls.clear()},{once:true})}
function assetSafe(value){const url=String(value||'');return /^data:(?:font\/|application\/)/i.test(url)||trustedAssetUrls.has(url)}
function cropMetrics(crop,width=100,height=100){const c=crop||{},left=Math.max(0,Math.min(.9999,+c.left||0)),top=Math.max(0,Math.min(.9999,+c.top||0)),right=Math.max(0,Math.min(.9999,+c.right||0)),bottom=Math.max(0,Math.min(.9999,+c.bottom||0)),visibleWidth=Math.max(.0001,1-Math.min(.9999,left+right)),visibleHeight=Math.max(.0001,1-Math.min(.9999,top+bottom));return{x:-left/visibleWidth*width,y:-top/visibleHeight*height,width:width/visibleWidth,height:height/visibleHeight}}
function cropImage(image,crop){const c=cropMetrics(crop);Object.assign(image.style,{position:'absolute',left:c.x+'%',top:c.y+'%',width:c.width+'%',height:c.height+'%',objectFit:'fill',maxWidth:'none',maxHeight:'none'})}
function imageEffects(e,n,o){const effects=o.imageEffects||{},radius=Math.max(0,+effects.softEdgeRadius||0);if(radius>0){const frame=o.frame||{},width=Math.max(1,+frame.width||1),height=Math.max(1,+frame.height||1),x=Math.min(50,radius/width*100),y=Math.min(50,radius/height*100),mask='linear-gradient(to right,transparent 0%,#000 '+x+'%,#000 '+(100-x)+'%,transparent 100%),linear-gradient(to bottom,transparent 0%,#000 '+y+'%,#000 '+(100-y)+'%,transparent 100%)';Object.assign(n.style,{maskImage:mask,maskComposite:'intersect',webkitMaskImage:mask,webkitMaskComposite:'source-in'})}const d=o.imageEffects&&o.imageEffects.duotone,c=v=>/^#[0-9a-f]{6}$/i.test(v||'')?[1,3,5].map(i=>parseInt(v.slice(i,i+2),16)/255):null,a=c(d&&d.shadowColor),b=c(d&&d.highlightColor);if(!a||!b)return;const ns='http://www.w3.org/2000/svg',v=document.createElementNS(ns,'svg'),f=document.createElementNS(ns,'filter'),m=document.createElementNS(ns,'feColorMatrix'),t=document.createElementNS(ns,'feComponentTransfer'),id='unippt-duotone-'+String(o.id||'image').replace(/[^a-z0-9_-]/gi,'-')+'-'+Math.random().toString(36).slice(2);f.id=id;f.setAttribute('color-interpolation-filters','sRGB');m.setAttribute('type','saturate');m.setAttribute('values','0');for(let i=0;i<3;i++){const q=document.createElementNS(ns,'feFunc'+'RGB'[i]);q.setAttribute('type','table');q.setAttribute('tableValues',a[i]+' '+b[i]);t.append(q)}f.append(m,t);const z=document.createElementNS(ns,'defs');z.append(f);v.append(z);Object.assign(v.style,{position:'absolute',width:'0',height:'0',overflow:'hidden'});v.setAttribute('aria-hidden','true');e.append(v);n.style.filter='url(#'+id+')'}
function imageFillMetrics(crop,fillRect,width=100,height=100){const f=fillRect||{},left=+f.left||0,top=+f.top||0,right=+f.right||0,bottom=+f.bottom||0,d={x:left*width,y:top*height,width:Math.max(.0001,(1-left-right)*width),height:Math.max(.0001,(1-top-bottom)*height)},c=cropMetrics(crop,d.width,d.height);return{x:d.x+c.x,y:d.y+c.y,width:c.width,height:c.height}}
function applyImageFill(e,o){if(!o.shapeFillAsset||o.customGeometry||o.kind==='image')return;if(/\brepeat\b/i.test(o.style?.fill||'')){e.style.backgroundImage='url('+JSON.stringify(o.shapeFillAsset)+')';e.style.backgroundRepeat='repeat';return}const f=o.frame||{},c=imageFillMetrics(o.imageCrop,o.imageFillRect,Math.max(1,+f.width||1),Math.max(1,+f.height||1)),n=new Image;n.className='shape-fill-image';n.src=o.shapeFillAsset;n.alt='';Object.assign(n.style,{position:'absolute',left:c.x+'px',top:c.y+'px',width:c.width+'px',height:c.height+'px',maxWidth:'none',maxHeight:'none',objectFit:'fill',pointerEvents:'none',zIndex:'-1'});e.style.background='transparent';e.prepend(n)}
function clear(preserveUnderlay=null){epoch++;clearTimeout(playerAutoPlayTimer);playerAutoPlayTimer=0;timers.forEach(clearTimeout);timers=[];animationPlayers.forEach(p=>{try{p.cancel()}catch(_){}});animationPlayers=[];players.forEach(p=>{try{p.cancel()}catch(_){}});players=[];animationTargetCursor=null;animationBatchStartCursor=null;if(activeTransition){activeTransition.settled=true;activeTransition.cleanup?.();if(activeTransition.underlay!==preserveUnderlay)activeTransition.underlay?.remove();activeTransition=null}globalThis.UniPptMedia?.stopAll(s,true,true);return epoch}
function later(fn,delay,token=epoch){const timer=setTimeout(()=>{if(token===epoch)fn()},Math.max(0,+delay||0));timers.push(timer);return timer}
function control(e,a){return globalThis.UniPptMedia?.controlNode(e,a)||false}
function media(e,o){return globalThis.UniPptMedia?.attach(e,o,false,{scope:'presentation',slideKey:d.slides[page]?.sourcePartName||d.slides[page]?.id||String(page)})||null}
function transition(sl,underlay=null){const t=sl.transition;if(!t){underlay?.remove();return null}let result=globalThis.UniPptPresentationTransitions?.create(s,t,underlay,{slideIndex:page});if(!result){const kind=String(t.kind||'fade').toLowerCase(),dur=Math.max(1,+t.durationMs||700);let keyframes;if(['circle','diamond','plus','wedge','wheel'].includes(kind))keyframes=[{clipPath:'circle(0% at 50% 50%)'},{clipPath:'circle(75% at 50% 50%)'}];else if(['wipe','push','pull','cover','strips','split','blinds','checker','randombar','comb'].includes(kind))keyframes=[{clipPath:'inset(0 0 0 100%)',opacity:.45},{clipPath:'inset(0 0 0 0)',opacity:1}];else if(kind!=='cut'&&kind!=='none')keyframes=[{opacity:0},{opacity:1}];if(!keyframes){underlay?.remove();return null}const player=s.animate(keyframes,{duration:dur,easing:'ease-out',fill:'both'});result={player,animations:[player]}}const record={...result,underlay,settled:false};activeTransition=record;for(const player of result.animations||[result.player])players.push(player);const settle=()=>{if(record.settled)return;record.settled=true;if(activeTransition===record)activeTransition=null;record.cleanup?.();record.underlay?.remove();for(const player of record.animations||[]){try{if(player.playState!=='idle')player.cancel()}catch(_){}}};result.player.finished.then(settle,settle);return result.player}
function scheduleAnimationBatch(es,startIndex=0){return globalThis.UniPptPresentationFlow.scheduleAnimationBatch(es,startIndex)}
function scheduleAnimationNavigation(es,startIndex=0){return globalThis.UniPptPresentationFlow.scheduleAnimationNavigation(es,startIndex)}
function animationNavigationRanges(es){return globalThis.UniPptPresentationFlow.animationNavigationRanges(es)}
function previousAnimationNavigationCursor(es,value){return globalThis.UniPptPresentationFlow.previousAnimationNavigationCursor(es,value)}
function losslessAnimationFrames(a,e){const base=e.dataset.animationBase||e.dataset.base||e.style.transform||'',exit=a.class==='exit';let result=globalThis.UniPptPresetAnimation?.frames(a,{node:e,slideWidth:d.width,slideHeight:d.height,baseTransform:base});if(!result&&a.effect==='flyIn'){const direction=a.direction||'left',x=direction==='left'?'-22%':direction==='right'?'22%':'0',y=direction==='up'?'-22%':direction==='down'?'22%':'0';result=[{opacity:0,transform:base+' translate('+x+','+y+')'},{opacity:1,transform:base+' translate(0,0)'}]}else if(!result&&a.effect==='wipe'){const direction=a.direction||'left',hidden=direction==='right'?'inset(0 100% 0 0)':direction==='up'?'inset(100% 0 0 0)':direction==='down'?'inset(0 0 100% 0)':'inset(0 0 0 100%)';result=[{clipPath:hidden,opacity:1},{clipPath:'inset(0 0 0 0)',opacity:1}]}else if(!result&&a.effect==='zoom')result=[{opacity:0,transform:base+' scale(.25)'},{opacity:1,transform:base+' scale(1)'}];else if(!result&&a.effect==='spin')result=[{transform:base+' rotate(0deg)'},{transform:base+' rotate(360deg)'}];else if(!result&&a.effect==='growShrink')result=[{transform:base+' scale(1)'},{transform:base+' scale(1.28)'},{transform:base+' scale(1)'}];else if(!result&&a.effect==='motionPath')result=globalThis.UniPptMotionPath?.frames(a.motionPath,d.width,d.height,base)||[{transform:base+' translate(0,0)'},{transform:base+' translate(15%,0)'}];else if(!result)result=[{opacity:0},{opacity:1}];return exit&&!a.propertyAnimations?.length?result.reverse():result}
function play(a,offset=0,token=epoch){const e=s.querySelector('[data-id="'+CSS.escape(a.targetObjectId||'')+'"]');if(!e)return 0;const delay=Math.max(0,+offset||0),timing=globalThis.UniPptPresentationFlow.animationPlaybackTiming(a),activeDuration=globalThis.UniPptPresentationFlow.animationActiveDuration(a),run=()=>{if(token!==epoch||!e.isConnected)return;e.style.visibility='visible';const keyframes=losslessAnimationFrames(a,e),previousWillChange=e.style.willChange,properties=new Set;for(const frame of keyframes){if(frame.transform!=null)properties.add('transform');if(frame.opacity!=null)properties.add('opacity');if(frame.clipPath!=null)properties.add('clip-path');if(frame.filter!=null)properties.add('filter');if(frame.maskImage!=null||frame.webkitMaskImage!=null)properties.add('mask')}if(properties.size)e.style.willChange=[...properties].join(',');const player=e.animate(keyframes,timing);players.push(player);animationPlayers.push(player);const restore=()=>{e.style.willChange=previousWillChange};player.oncancel=restore;player.onfinish=()=>{restore();if(token!==epoch||!e.isConnected)return;if(a.class==='exit'&&!a.autoReverse&&!(Number(a.speed)<0)){e.style.visibility='hidden';e.style.opacity='0'}else e.style.opacity='1'}};if(delay===0)run();else later(run,delay,token);return delay+activeDuration}
function effect(a,offset=0,token=epoch){if(a.effect!=='media'&&a.class!=='media')return play(a,offset,token);const e=s.querySelector('[data-id="'+CSS.escape(a.targetObjectId||'')+'"]');if(!e)return 0;const delay=Math.max(0,+offset||0),dur=globalThis.UniPptPresentationFlow.animationActiveDuration(a),run=()=>{if(token!==epoch||!e.isConnected)return;e.style.visibility='visible';control(e,a.mediaAction||'play')};if(delay===0)run();else later(run,delay,token);return delay+dur}
function runAnimationBatch(es=effects(),startOffset=0){if(animationTargetCursor!=null)return null;const schedule=scheduleAnimationNavigation(es,cursor),token=epoch;if(!schedule.entries.length||!schedule.positions.length)return schedule;const first=schedule.positions[0];animationBatchStartCursor=first.startIndex;animationTargetCursor=first.endIndex;for(const item of schedule.entries)effect(item.animation,Math.max(0,+startOffset||0)+item.start,token);schedule.positions.forEach((position,index)=>later(()=>{if(token!==epoch)return;cursor=position.endIndex;const nextPosition=schedule.positions[index+1];animationBatchStartCursor=nextPosition?.startIndex??null;animationTargetCursor=nextPosition?.endIndex??null;status()},Math.max(0,+startOffset||0)+position.end,token));status();return schedule}
function finishAnimationBatch(){const target=animationTargetCursor;if(target==null)return false;timers.forEach(clearTimeout);timers=[];animationPlayers.forEach(player=>{try{player.cancel()}catch(_){}});players=players.filter(player=>!animationPlayers.includes(player));animationPlayers=[];for(const animation of effects().slice(0,target))applyPlayerAnimationFinalState(animation,true);cursor=target;animationTargetCursor=null;animationBatchStartCursor=null;status();syncPlayerAudio();return true}
function cancelAnimationBatch(){if(animationTargetCursor==null)return false;const restore=animationBatchStartCursor??cursor;seekPlayerCursor(restore);return true}
function startAutomaticBatch(startOffset=0){if(restoringPlayerState)return false;const es=effects();if(!(cursor===0&&es.length&&(es[0].trigger||'onClick')!=='onClick'))return false;const delay=Math.max(0,+startOffset||0);if(!playerActivated){pendingPlayerAutomaticStartOffset=delay;return false}pendingPlayerAutomaticStartOffset=null;runAnimationBatch(es,delay);return true}
function stopPersistentMedia(){globalThis.UniPptMedia?.stopPersistent(true)}
function advance(){hideLosslessEndNotice();const es=effects();if(finishAnimationBatch())return;if(cursor>=es.length){if(page<d.slides.length-1){page++;draw(true)}else showLosslessEndNotice();return}runAnimationBatch(es)}
function rollback(){if(cancelAnimationBatch())return;const previous=previousAnimationNavigationCursor(effects(),cursor);if(previous!=null){seekPlayerCursor(previous);return}if(page>0){page--;draw(true,{reverse:true});seekPlayerCursor(effects().length)}}
function draw(playTransition=false,options={}){const sl=d.slides[page],runtime=globalThis.UniPptPresentationTransitions;const underlay=playTransition&&sl.transition?runtime?.captureUnderlay(s):null;clear(underlay);cursor=0;s.replaceChildren();s.style.width=d.width+'px';s.style.height=d.height+'px';s.style.background=sl.background||'#fff';s.style.backgroundImage=sl.backgroundAsset?'url('+JSON.stringify(sl.backgroundAsset)+')':'none';s.style.backgroundSize='100% 100%';s.style.backgroundPosition='center';s.style.backgroundRepeat='no-repeat';[...(sl.masterObjects||[]),...(sl.layoutObjects||[]),...(sl.objects||[])].forEach((o,z)=>add(o,z,s));for(const a of effects()){if(a.class==='entrance'){const e=s.querySelector('[data-id="'+CSS.escape(a.targetObjectId||'')+'"]');if(e){e.style.visibility='hidden';e.style.opacity='0'}}}scale();status();let transitionValue=sl.transition;if(options.reverse&&transitionValue){const reverse={l:'r',left:'right',r:'l',right:'left',u:'d',up:'down',d:'u',down:'up',in:'out',out:'in'};transitionValue={...transitionValue,direction:reverse[String(transitionValue.direction||'left').toLowerCase()]||transitionValue.direction}}if(playTransition&&transitionValue)transition({...sl,transition:transitionValue},underlay);else underlay?.remove();const lead=playTransition&&transitionValue&&!['none','cut'].includes(String(transitionValue.kind||'').toLowerCase())?Math.max(1,+transitionValue.durationMs||700):0;if(playerAutoPlay)scheduleLosslessAutoPlay(lead+(effects().length?0:1000));else startAutomaticBatch(lead);if(!playerAutoPlay&&sl.transition&&Number(sl.transition.advanceAfterMs)>0)later(advance,sl.transition.advanceAfterMs)}
prev.onclick=rollback;
"#;
const LOSSLESS_PLAYER_CONTROLS_RUNTIME: &str =
    include_str!("../../../web/lossless_player_controls.js");

#[derive(Debug, Error)]
pub enum HtmlError {
    #[error("无损 HTML 序列化失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("无损 HTML Brotli 编解码失败: {0}")]
    Brotli(String),
    #[error("不是有效的 UniPPT 无损 HTML：{0}")]
    Invalid(String),
    #[error("lossless HTML asset processing failed: {0}")]
    Asset(String),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HtmlAsset {
    id: String,
    mime_type: String,
    data_uri_prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_blob: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    browser_mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    browser_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    browser_blob: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HtmlBlob {
    path: String,
    size: usize,
    sha256: String,
    base64: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HtmlPayload {
    format: String,
    version: u32,
    #[serde(default = "default_document_format")]
    document_format: String,
    #[serde(default = "default_unidoc_version")]
    unidoc_version: u32,
    unidoc_type: String,
    app: String,
    deck: Deck,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    baseline_deck: Option<Deck>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    assets: Vec<HtmlAsset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opc_package: Option<OpcPackageIndex>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    blobs: Vec<HtmlBlob>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pptx_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pptx_br: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pptx_size: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scene_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    baseline_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opc_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pptx_sha256: Option<String>,
}

#[derive(Deserialize)]
struct RawHtmlPayload {
    deck: Box<RawValue>,
    #[serde(default, rename = "baselineDeck")]
    baseline_deck: Option<Box<RawValue>>,
    #[serde(default, rename = "opcPackage")]
    opc_package: Option<Box<RawValue>>,
}

fn default_document_format() -> String {
    "udoc".into()
}

fn default_unidoc_version() -> u32 {
    3
}

#[allow(dead_code)]
pub struct DecodedHtml {
    pub deck: Deck,
    pub presentation: Vec<u8>,
}

pub(crate) struct DecodedCompactHtml {
    pub(crate) deck: Deck,
    pub(crate) original_deck: Deck,
    pub(crate) presentation: Option<Vec<u8>>,
    pub(crate) opc: OpcSnapshot,
    pub(crate) presentation_digest: [u8; 32],
    pub(crate) assets: AssetCatalog,
    pub(crate) version: u32,
}

pub fn encode(deck: &Deck, presentation: &[u8]) -> Result<Vec<u8>, HtmlError> {
    let mut compact = deck.clone();
    let mut catalog = AssetCatalog::default();
    externalize_deck_assets(&mut compact, "html", &mut catalog).map_err(HtmlError::Asset)?;
    let opc = opc_snapshot::explode(presentation).map_err(HtmlError::Invalid)?;
    encode_cached(&compact, &compact, "html", &catalog, &opc)
}

/// Builds the standalone player used by the local Chromium publishing
/// pipeline. Unlike `encode_cached`, this intentionally carries only scene
/// data and the assets referenced by that scene. The canonical lossless HTML
/// keeps the complete OPC snapshot so it can be imported again; paying that
/// serialization and I/O cost before every PDF/image/video render is both
/// unnecessary and very expensive for media-heavy presentations.
pub(crate) fn encode_render(deck: &Deck) -> Result<Vec<u8>, HtmlError> {
    let mut compact = deck.clone();
    let mut catalog = AssetCatalog::default();
    externalize_deck_assets(&mut compact, "render", &mut catalog).map_err(HtmlError::Asset)?;
    encode_render_cached(&compact, "render", &catalog)
}

pub(crate) fn encode_render_cached(
    compact: &Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<Vec<u8>, HtmlError> {
    let mut persisted_deck = compact.clone();
    persisted_deck.source_import_id = None;
    let referenced = portableize_deck_assets(&mut persisted_deck, cache_id, catalog)?;
    let mut blob_bytes = BTreeMap::new();
    let assets = serialize_assets_v4(&referenced, catalog, &mut blob_bytes)?;
    let blobs = serialize_blobs(&blob_bytes)?;
    let payload = HtmlPayload {
        format: "unippt-render-html".into(),
        version: 1,
        document_format: default_document_format(),
        unidoc_version: default_unidoc_version(),
        unidoc_type: CURRENT_UNIDOC_TYPE.into(),
        app: "UniPPT".into(),
        deck: persisted_deck,
        baseline_deck: None,
        assets,
        opc_package: None,
        blobs,
        pptx_base64: None,
        pptx_br: None,
        pptx_size: None,
        scene_sha256: None,
        baseline_sha256: None,
        opc_sha256: None,
        pptx_sha256: None,
    };
    render_html(&compact.title, &payload)
}

pub(crate) fn encode_cached(
    compact: &Deck,
    baseline: &Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
    opc: &OpcSnapshot,
) -> Result<Vec<u8>, HtmlError> {
    let mut persisted_deck = compact.clone();
    let mut persisted_baseline = baseline.clone();
    persisted_deck.source_import_id = None;
    persisted_baseline.source_import_id = None;
    let mut referenced = portableize_deck_assets(&mut persisted_deck, cache_id, catalog)?;
    referenced.extend(portableize_deck_assets(
        &mut persisted_baseline,
        cache_id,
        catalog,
    )?);
    let baseline_deck = (persisted_baseline != persisted_deck).then_some(persisted_baseline);
    let mut blob_bytes = opc.blobs.clone();
    let assets = serialize_assets_v4(&referenced, catalog, &mut blob_bytes)?;
    let blobs = serialize_blobs(&blob_bytes)?;
    let scene_sha256 = sha256_hex(&script_safe_json_bytes(&persisted_deck)?);
    let baseline_sha256 = baseline_deck
        .as_ref()
        .map(script_safe_json_bytes)
        .transpose()?
        .map(|bytes| sha256_hex(&bytes));
    let opc_sha256 = sha256_hex(&script_safe_json_bytes(&opc.index)?);
    let payload = HtmlPayload {
        format: "unippt-html".into(),
        version: 4,
        document_format: default_document_format(),
        unidoc_version: default_unidoc_version(),
        unidoc_type: CURRENT_UNIDOC_TYPE.into(),
        app: "UniPPT".into(),
        deck: persisted_deck,
        baseline_deck,
        assets,
        opc_package: Some(opc.index.clone()),
        blobs,
        pptx_base64: None,
        pptx_br: None,
        pptx_size: None,
        scene_sha256: Some(scene_sha256),
        baseline_sha256,
        opc_sha256: Some(opc_sha256),
        pptx_sha256: None,
    };
    render_html(&compact.title, &payload)
}

fn render_html(title: &str, payload: &HtmlPayload) -> Result<Vec<u8>, HtmlError> {
    let title = html_escape(title);
    let template = HTML_TEMPLATE
        .replace("__TITLE__", &title)
        .replace("__CHART_RUNTIME__", include_str!("../../../web/chart_runtime.js"))
        .replace(
            "__PRESENTATION_SCENE_RUNTIME__",
            include_str!("../../../web/presentation_scene_runtime.js"),
        )
        .replace(
            "__PRESET_ANIMATION_RUNTIME__",
            include_str!("../../../web/preset_animation_runtime.js"),
        )
        .replace(
            "__MOTION_PATH_RUNTIME__",
            include_str!("../../../web/motion_path_runtime.js"),
        )
        .replace(
            "__MEDIA_RUNTIME__",
            include_str!("../../../web/media_runtime.js"),
        )
        .replace(
            "__PRESENTATION_FLOW_RUNTIME__",
            include_str!("../../../web/presentation_flow_runtime.js"),
        )
        .replace(
            "__PRESENTATION_TRANSITION_RUNTIME__",
            include_str!("../../../web/presentation_transition_runtime.js"),
        )
        .replace(
            ",d=payload.deck,s=document.getElementById('stage')",
            ",d=payload.deck,trustedAssetUrls=new Set,assetObjectUrls=new Map;resolveDeckAssets(d,payload.assets||[],payload.blobs||[]);const s=document.getElementById('stage')",
        )
        .replace(
            "let f;if(kind==='flyIn')",
            "let f=globalThis.UniPptPresetAnimation?.frames(a,{node:e,slideWidth:d.width,slideHeight:d.height,baseTransform:base});if(f){}else if(kind==='flyIn')",
        )
        .replace(
            "dir==='left'?'-18%':dir==='right'?'18%':'0',y=dir==='up'?'-18%':dir==='down'?'18%':'0'",
            "dir==='left'?'-22%':dir==='right'?'22%':'0',y=dir==='up'?'-22%':dir==='down'?'22%':'0'",
        )
        .replace(
            "else if(kind==='motionPath')f=[{transform:base+' translate(0,0)'},{transform:base+' translate(15%,0)'}]",
            "else if(kind==='motionPath')f=globalThis.UniPptMotionPath?.frames(a.motionPath,d.width,d.height,base)||[{transform:base+' translate(0,0)'},{transform:base+' translate(15%,0)'}]",
        )
        // `advTm="0"` is emitted by several PowerPoint/WPS templates as a
        // disabled timing sentinel. Scheduling it at 1 ms skips the deck.
        .replace(
            "if(sl.transition&&sl.transition.advanceAfterMs!=null)timers.push(setTimeout(advance,Math.max(1,sl.transition.advanceAfterMs)))",
            "if(sl.transition&&Number(sl.transition.advanceAfterMs)>0)timers.push(setTimeout(advance,sl.transition.advanceAfterMs))",
        )
        // Keep the compact inline runtime syntactically valid. The same
        // conditional occurs in fly-in, wipe, and zoom animation branches.
        .replace("return exit=f.reverse():f", "return exit?f.reverse():f")
        .replace(
            "function box(e,o,z){",
            "function dash(v){return /dot/i.test(v||'')?'dotted':/dash/i.test(v||'')?'dashed':'solid'}function alpha(c,a){const m=/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(c||'');return m?'rgba('+parseInt(m[1],16)+','+parseInt(m[2],16)+','+parseInt(m[3],16)+','+Math.max(0,Math.min(1,a))+')':c}function shadow(v){return v?(v.inset?'inset ':'')+(v.offsetX||0)+'px '+(v.offsetY||0)+'px '+Math.max(0,v.blur||0)+'px '+alpha(v.color||'#000000',v.opacity??.35):'none'}function safe(u){return /^(https?:|mailto:|tel:)/i.test(String(u||'').trim())}function jump(n){if(n<0||n>=d.slides.length)return false;page=n;draw(true);return true}function go(a){if(!a)return false;const x=String(a.action||'').toLowerCase();if(x.includes('jump=nextslide'))return jump(page+1);if(x.includes('jump=previousslide'))return jump(page-1);if(x.includes('jump=firstslide'))return jump(0);if(x.includes('jump=lastslide'))return jump(d.slides.length-1);if(x.includes('jump=endshow')){document.exitFullscreen?.();return true}if(a.target&&!a.external){const n=d.slides.findIndex(v=>v.sourcePartName===a.target);if(n>=0)return jump(n)}if(safe(a.target)){open(a.target,'_blank','noopener,noreferrer');return true}return false}function wire(e,o){const h=o.hyperlinks||{},tip=h.click?.tooltip||h.hover?.tooltip;if(!h.click&&!h.hover)return;e.style.cursor='pointer';if(tip)e.title=tip;if(h.click)e.onclick=v=>{if(go(h.click)){v.preventDefault();v.stopPropagation()}};if(h.hover)e.onmouseenter=v=>{if(e.dataset.hoverLink!=='1'&&go(h.hover)){e.dataset.hoverLink='1';v.stopPropagation()}}}function box(e,o,z){",
        )
        .replace(
            "function box(e,o,z){",
            "function textGradient(n,r){const g=r.gradient;if(!g||(g.stops||[]).length===0)return;const stops=g.stops.map(x=>alpha(x.color||'#000000',x.opacity??1)+' '+Math.max(0,Math.min(1,+x.position||0))*100+'%');n.style.backgroundImage='linear-gradient('+(+g.angle||0)+'deg,'+stops.join(',')+')';n.style.backgroundClip='text';n.style.webkitBackgroundClip='text';n.style.color='transparent';n.style.webkitTextFillColor='transparent'}function box(e,o,z){",
        )
        .replace(
            "box(e,o,z);parent.append(e);",
            "box(e,o,z);wire(e,o);parent.append(e);",
        )
        .replace(
            "box(e,o,z);wire(e,o);parent.append(e);",
            "box(e,o,z);wire(e,o);custom(e,o);parent.append(e);",
        )
        .replace(
            "});line.append(n)}flow.append(line)}e.append(flow)",
            "});textGradient(n,r);wire(n,r);line.append(n)}flow.append(line)}e.append(flow)",
        )
        .replace(
            "border:(st.strokeWidth||0)+'px solid '+(st.stroke||'transparent'),opacity:",
            "border:(st.strokeWidth||0)+'px '+dash(st.strokeDash)+' '+(st.stroke||'transparent'),boxShadow:shadow(st.shadow),opacity:",
        )
        .replace(
            "textDecoration:(r.underline?'underline ':'')+(r.strikethrough?'line-through':''),verticalAlign:",
            "textDecoration:(r.underline?'underline ':'')+(r.strikethrough?'line-through':''),textDecorationStyle:/wavy/i.test(r.underlineStyle||'')?'wavy':/dbl/i.test(r.underlineStyle||'')?'double':'solid',opacity:r.alpha??1,verticalAlign:",
        )
        .replace(
            ".o.math{justify-content:center;overflow:visible}",
            ".o.math{justify-content:center;overflow:visible}.o.table{padding:0!important;align-items:stretch!important;background:transparent}.table-grid{width:100%;height:100%;display:grid;min-width:0;min-height:0;overflow:hidden}.table-cell{box-sizing:border-box;min-width:0;min-height:0;overflow:hidden;display:flex;line-height:1.15;white-space:normal}",
        )
        .replace(
            "function add(o,z,parent){",
            "function edge(e,n,v){e.style[n]=v?(v.width||0)+'px '+dash(v.dash)+' '+(v.color||'transparent'):'0 solid transparent'}function table(e,o){const t=o.table,g=document.createElement('div');g.className='table-grid';const cols=t.columns&&t.columns.length?t.columns:Array(Math.max(1,...t.rows.map(r=>(r.cells||[]).length))).fill(1);g.style.gridTemplateColumns=cols.map(v=>'minmax(0,'+Math.max(1,+v||1)+'fr)').join(' ');g.style.gridTemplateRows=t.rows.map(r=>'minmax(0,'+Math.max(1,+r.height||1)+'fr)').join(' ');t.rows.forEach((r,y)=>(r.cells||[]).forEach((c,x)=>{if(c.hMerge||c.vMerge)return;const n=document.createElement('div'),ts=c.textStyle||{},tf=c.textFrame||{},b=c.borders||{};n.className='table-cell';n.style.gridColumn=(x+1)+' / span '+Math.max(1,c.gridSpan||1);n.style.gridRow=(y+1)+' / span '+Math.max(1,c.rowSpan||1);Object.assign(n.style,{background:c.fill||'transparent',color:ts.color||'#172033',fontFamily:ts.fontFamily||'Aptos,sans-serif',fontSize:(ts.fontSize||18)+'px',fontWeight:ts.bold?'700':'400',fontStyle:ts.italic?'italic':'normal',textAlign:ts.align||'left',padding:(tf.marginTop??5)+'px '+(tf.marginRight??8)+'px '+(tf.marginBottom??5)+'px '+(tf.marginLeft??8)+'px',alignItems:tf.verticalAlign==='top'?'flex-start':tf.verticalAlign==='bottom'?'flex-end':'center'});edge(n,'borderLeft',b.left);edge(n,'borderRight',b.right);edge(n,'borderTop',b.top);edge(n,'borderBottom',b.bottom);if((c.textParagraphs||[]).length)rich(n,c);else n.textContent=c.text||'';g.append(n)}));e.append(g)}function add(o,z,parent){",
        )
        .replace(
            "function add(o,z,parent){",
            "function custom(e,o){const g=o.customGeometry;if(!g||!g.pathData)return false;const w=+g.width||0,h=+g.height||0;if(w<0||h<0||!(w>0||h>0))return false;const ns='http://www.w3.org/2000/svg',v=document.createElementNS(ns,'svg'),p=document.createElementNS(ns,'path'),st=o.style||{};v.classList.add('custom-geometry');v.setAttribute('viewBox',(w>0?0:-.5)+' '+(h>0?0:-.5)+' '+Math.max(1,w)+' '+Math.max(1,h));v.setAttribute('preserveAspectRatio','none');p.setAttribute('d',g.pathData);if(o.shapeFillAsset){const defs=document.createElementNS(ns,'defs'),pat=document.createElementNS(ns,'pattern'),im=document.createElementNS(ns,'image'),id='shape-fill-'+String(o.id||'shape').replace(/[^a-zA-Z0-9_-]/g,'-')+'-'+Math.random().toString(36).slice(2);pat.id=id;pat.setAttribute('patternUnits','userSpaceOnUse');pat.setAttribute('width',Math.max(1,w));pat.setAttribute('height',Math.max(1,h));im.setAttribute('href',o.shapeFillAsset);im.setAttribute('width',Math.max(1,w));im.setAttribute('height',Math.max(1,h));im.setAttribute('preserveAspectRatio','none');pat.append(im);defs.append(pat);v.append(defs);p.setAttribute('fill','url(#'+id+')')}else p.setAttribute('fill',st.fill||'transparent');p.setAttribute('fill-rule','evenodd');p.setAttribute('stroke',st.stroke||'transparent');p.setAttribute('stroke-width',Math.max(0,st.strokeWidth||0));p.setAttribute('vector-effect','non-scaling-stroke');v.append(p);e.style.background='transparent';e.style.border='0';e.style.padding='0';e.style.overflow='visible';e.append(v);return true}function add(o,z,parent){",
        )
        .replace(
            "e.className='o'+(o.kind==='group'?' group':o.kind==='math'?' math formula':'');",
            "e.className='o'+(o.kind==='group'?' group':o.kind==='math'?' math formula':o.kind==='table'?' table':'');",
        )
        .replace(
            "}else if(o.kind==='math'&&o.formula)e.textContent=",
            "}else if(o.kind==='table'&&o.table&&(o.table.rows||[]).length)table(e,o);else if(o.kind==='math'&&o.formula)e.textContent=",
        )
        .replace(
            "}else if(o.kind==='table'&&o.table&&(o.table.rows||[]).length)table(e,o);else if(o.kind==='math'&&o.formula)e.textContent=",
            "}else if(o.kind==='table'&&o.table&&(o.table.rows||[]).length)table(e,o);else if(o.kind==='chart'&&o.chart)globalThis.UniPPTChartRuntime.render(e,o.chart);else if(o.kind==='math'&&o.formula)e.textContent=",
        )
        .replace(
            ".o img{width:100%;height:100%;object-fit:cover}",
            ".o img{width:100%;height:100%;object-fit:cover}.o>.custom-geometry{position:absolute;inset:0;width:100%;height:100%;overflow:visible;pointer-events:none}.o>video.media,.o>video.native-media{position:absolute;inset:0;z-index:1;width:100%;height:100%;object-fit:cover;pointer-events:none}.o>audio.media,.o>audio.native-media{display:none}.o.media-blocked:after,.o.media-play-blocked:after{content:attr(data-media-prompt);position:absolute;right:8px;bottom:8px;z-index:4;padding:5px 9px;border-radius:3px;background:#202020e8;color:#fff;font:12px/1.2 \"Segoe UI\",\"Microsoft YaHei\",sans-serif;white-space:nowrap}",
        )
        .replace(
            "function add(o,z,parent){",
            "function control(e,a){const n=e.querySelector('audio.media,video.media');if(!n)return false;const x=String(a||'play').toLowerCase(),st=+(n.dataset.trimStart||0),en=+(n.dataset.trimEnd||0);if(x.includes('pause')){n.pause();return true}if(x.includes('stop')){n.pause();try{n.currentTime=st}catch(_){}return true}if(n.ended||n.currentTime<st||(en>0&&n.currentTime>=en))try{n.currentTime=st}catch(_){}n.play().catch(()=>{e.classList.add('media-blocked');e.dataset.mediaPrompt=n.tagName==='AUDIO'?'单击启用音频':'单击播放视频'});return true}function media(e,o){const m=o.media;if(!m||!m.asset)return;const n=document.createElement(m.kind==='video'?'video':'audio');n.className='media';n.src=m.asset;n.preload='metadata';n.playsInline=true;n.volume=Math.max(0,Math.min(1,m.volume??1));n.dataset.trimStart=String(Math.max(0,m.trimStartMs||0)/1000);n.dataset.trimEnd=m.trimEndMs==null?'':String(Math.max(0,m.trimEndMs)/1000);n.dataset.loop=m.loopPlayback?'1':'0';if(m.kind==='video'&&o.asset)n.poster=o.asset;else n.hidden=true;n.onloadedmetadata=()=>{const st=+n.dataset.trimStart;if(st>0)try{n.currentTime=st}catch(_){}};n.ontimeupdate=()=>{const en=+n.dataset.trimEnd;if(en>0&&n.currentTime>=en){if(n.dataset.loop==='1'){try{n.currentTime=+n.dataset.trimStart||0}catch(_){};n.play().catch(()=>{})}else n.pause()}};n.onended=()=>{if(n.dataset.loop==='1'){try{n.currentTime=+n.dataset.trimStart||0}catch(_){};n.play().catch(()=>{})}};n.onplay=()=>{e.classList.remove('media-blocked');delete e.dataset.mediaPrompt};e.append(n);e.addEventListener('click',v=>{v.stopPropagation();control(e,n.paused?'play':'pause')})}function add(o,z,parent){",
        )
        .replace(
            "const m=o.media;if(!m||!m.asset)return;",
            "const m=o.media;if(!m||!(m.playbackAsset||m.asset))return;",
        )
        .replace("n.src=m.asset;", "n.src=m.playbackAsset||m.asset;")
        .replace(
            "else e.textContent=o.text||(['chart','table','smartArt'].includes(o.kind)?o.name:'')}",
            "else if(!o.customGeometry)e.textContent=o.text||(['chart','table','smartArt'].includes(o.kind)?o.name:'');applyImageFill(e,o);media(e,o)}",
        )
        .replace(
            "let page=0,cursor=0,timers=[];",
            r#"function cssq(v){return '"'+String(v||'').replace(/\\/g,'\\\\').replace(/"/g,'\\"').replace(/[\r\n]/g,' ')+'"'}let ff=[];function fontFaces(){for(const x of ff)document.fonts.delete(x);ff=[];document.getElementById('unippt-embedded-fonts')?.remove();const a=(d.fonts||[]).map(f=>{const u=String(f.dataUri||'');if(!f.family||!/^data:(?:font\/|application\/)/i.test(u))return null;return{family:f.family,source:u,format:f.format||'truetype',weight:Math.max(1,Math.min(1000,Number(f.weight)||400)),style:f.style==='italic'?'italic':'normal'}}).filter(Boolean);if(globalThis.FontFace){for(const x of a){try{const f=new FontFace(x.family,'url('+JSON.stringify(x.source)+') format('+cssq(x.format)+')',{weight:String(x.weight),style:x.style,display:'block'});document.fonts.add(f);ff.push(f);void f.load().catch(()=>document.fonts.delete(f))}catch(_){}}return}const h=document.createElement('div');h.id='unippt-embedded-fonts';h.hidden=true;for(const x of a){const s=document.createElement('style');s.textContent='@font-face{font-family:'+cssq(x.family)+';src:url('+JSON.stringify(x.source)+') format('+cssq(x.format)+');font-weight:'+x.weight+';font-style:'+x.style+';font-display:block;}';h.append(s)}document.head.append(h)}fontFaces();let page=0,cursor=0,timers=[];"#,
        )
        .replace(
            "fontFaces();let page=0,cursor=0,timers=[];",
            "fontFaces();let page=0,cursor=0,timers=[],epoch=0,players=[];",
        )
        .replace(
            "fontFaces();let page=0,cursor=0,timers=[],epoch=0,players=[];",
            "fontFaces();let page=0,cursor=0,timers=[],epoch=0,players=[],animationPlayers=[],animationTargetCursor=null,animationBatchStartCursor=null,activeTransition=null;",
        )
        .replace(
            "if(!f.family||!/^data:(?:font\\/|application\\/)/i.test(u))return null;",
            "if(!f.family||!assetSafe(u))return null;",
        )
        .replace(
            "if(sl.transition&&Number(sl.transition.advanceAfterMs)>0)timers.push(setTimeout(advance,sl.transition.advanceAfterMs))",
            "startAutomaticBatch();if(sl.transition&&Number(sl.transition.advanceAfterMs)>0)later(advance,sl.transition.advanceAfterMs)",
        )
        // Use native PowerPoint stepping: the leading automatic group starts
        // on entry, each click consumes exactly one on-click group (plus its
        // with/after dependants), and the following click changes slide.
        .replace(
            "fontFamily:ts.fontFamily||'Aptos,sans-serif'",
            "fontFamily:ts.fontFamily||ts.nativeFontFamily||'Aptos,sans-serif'",
        )
        .replace(
            "fontFamily:r.fontFamily||(o.textStyle||{}).fontFamily",
            "fontFamily:r.fontFamily||r.nativeFontFamily||(o.textStyle||{}).fontFamily||(o.textStyle||{}).nativeFontFamily",
        )
        .replace(
            "if(o.kind==='image'&&o.asset){const im=new Image;im.src=o.asset;e.append(im)}",
            "if(o.kind==='image'&&o.asset){const im=new Image;im.src=o.asset;cropImage(im,o.imageCrop);e.append(im);imageEffects(e,im,o)}",
        )
        .replace(
            "im.setAttribute('width',Math.max(1,w));im.setAttribute('height',Math.max(1,h));im.setAttribute('preserveAspectRatio','none');",
            "const crop=imageFillMetrics(o.imageCrop,o.imageFillRect,Math.max(1,w),Math.max(1,h));im.setAttribute('x',crop.x);im.setAttribute('y',crop.y);im.setAttribute('width',crop.width);im.setAttribute('height',crop.height);im.setAttribute('preserveAspectRatio','none');",
        )
        .replace(
            "const effects=()=>[...(d.slides[page].animations||[])].sort((a,b)=>(a.order||0)-(b.order||0));",
            "const effects=()=>[...(d.slides[page].inheritedAnimations||[]),...(d.slides[page].animations||[])].sort((a,b)=>(a.order||0)-(b.order||0));",
        )
        .replace(
            "(sl.objects||[]).forEach((o,z)=>add(o,z,s));",
            "[...(sl.masterObjects||[]),...(sl.layoutObjects||[]),...(sl.objects||[])].forEach((o,z)=>add(o,z,s));",
        )
        .replace(
            "prev.onclick=()=>{if(page>0){page--;draw(true)}};",
            LOSSLESS_PLAYER_RUNTIME,
        )
        .replace(
            "prev.onclick=rollback;",
            LOSSLESS_PLAYER_CONTROLS_RUNTIME,
        )
        .replace(
            "[...(sl.masterObjects||[]),...(sl.layoutObjects||[]),...(sl.objects||[])].forEach((o,z)=>add(o,z,s));",
            "globalThis.UniPptPresentationScene.renderSlide(sl,s,{clear:false,applyBackground:false,deckWidth:d.width,deckHeight:d.height,dynamicObjects:d.extensions?.['org.unippt.dynamic']?.objects||{},mediaContext:{scope:'presentation',slideKey:sl.sourcePartName||sl.id||String(page)},onHyperlink:link=>go(link)});",
        )
        .replace(
            "count.textContent=(page+1)+' / '+d.slides.length+(es.length?' · '+cursor+'/'+es.length:'');",
            "count.textContent=(page+1)+' / '+d.slides.length;count.title=es.length?'动画 '+cursor+' / '+es.length:'';",
        )
        .replace(
            "function scale(){const q=Math.min(innerWidth/d.width,innerHeight/d.height);s.style.transform='translate(-50%,-50%) scale('+q+')'}",
            "function scale(){const q=Math.min(innerWidth/d.width,innerHeight/d.height)*(typeof playerZoom==='number'?playerZoom:1);s.style.transform='translate(-50%,-50%) scale('+q+')'}",
        )
        .replace(
            "next.onclick=advance;s.onclick=advance;onkeydown=e=>{if(['ArrowRight','ArrowDown','PageDown',' '].includes(e.key)){e.preventDefault();advance()}if(['ArrowLeft','ArrowUp','PageUp'].includes(e.key)){e.preventDefault();prev.click()}};onresize=scale;draw(false)",
            "installLosslessPlayerControls();onresize=scale;draw(false)",
        );
    write_payload_into_template(template, payload)
}

fn write_payload_into_template(
    template: String,
    payload: &HtmlPayload,
) -> Result<Vec<u8>, HtmlError> {
    let (prefix, suffix) = template.split_once("__DATA__").ok_or_else(|| {
        HtmlError::Invalid("lossless HTML template payload marker is missing".into())
    })?;
    let mut output = Vec::with_capacity(template.len().saturating_add(64 * 1024));
    output.extend_from_slice(prefix.as_bytes());
    serde_json::to_writer(ScriptSafeWriter(&mut output), payload)?;
    output.extend_from_slice(suffix.as_bytes());
    Ok(output)
}

fn script_safe_json_bytes(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut output = Vec::new();
    serde_json::to_writer(ScriptSafeWriter(&mut output), value)?;
    Ok(output)
}

struct ScriptSafeWriter<'a>(&'a mut Vec<u8>);

impl Write for ScriptSafeWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let mut start = 0;
        for (index, byte) in buffer.iter().enumerate() {
            if *byte == b'<' {
                self.0.extend_from_slice(&buffer[start..index]);
                self.0.extend_from_slice(b"\\u003c");
                start = index + 1;
            }
        }
        self.0.extend_from_slice(&buffer[start..]);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn serialize_assets_v4(
    referenced: &BTreeSet<String>,
    catalog: &AssetCatalog,
    blobs: &mut BTreeMap<String, Blob>,
) -> Result<Vec<HtmlAsset>, HtmlError> {
    if referenced.len() > MAX_HTML_ASSET_COUNT {
        return Err(HtmlError::Asset(format!(
            "too many referenced assets: {}",
            referenced.len()
        )));
    }
    let mut total = 0usize;
    referenced
        .iter()
        .map(|id| {
            let asset = catalog
                .get(id)
                .ok_or_else(|| HtmlError::Asset(format!("cached scene asset is missing: {id}")))?;
            total = total
                .checked_add(asset.bytes.len())
                .ok_or_else(|| HtmlError::Asset("asset byte count overflow".into()))?;
            if total > MAX_HTML_ASSET_BYTES {
                return Err(HtmlError::Asset(format!(
                    "referenced assets exceed {} bytes",
                    MAX_HTML_ASSET_BYTES
                )));
            }
            let original_sha256 = sha256_hex(asset.bytes.as_ref());
            let original_blob = opc_snapshot::blob_path_for_digest(&original_sha256);
            insert_html_blob(blobs, &original_blob, Arc::clone(&asset.bytes))?;
            let has_browser_projection = asset.mime_type.as_ref()
                != asset.browser_mime_type.as_ref()
                || !Arc::ptr_eq(&asset.bytes, &asset.browser_bytes);
            let browser_blob = if has_browser_projection {
                let digest = sha256_hex(asset.browser_bytes.as_ref());
                let path = opc_snapshot::blob_path_for_digest(&digest);
                insert_html_blob(blobs, &path, Arc::clone(&asset.browser_bytes))?;
                Some(path)
            } else {
                None
            };
            Ok(HtmlAsset {
                id: id.clone(),
                mime_type: asset.mime_type.to_string(),
                data_uri_prefix: asset.data_uri_prefix.to_string(),
                original_base64: None,
                original_blob: Some(original_blob),
                browser_mime_type: has_browser_projection
                    .then(|| asset.browser_mime_type.to_string()),
                browser_base64: None,
                browser_blob,
            })
        })
        .collect()
}

fn insert_html_blob(
    blobs: &mut BTreeMap<String, Blob>,
    path: &str,
    bytes: Blob,
) -> Result<(), HtmlError> {
    if let Some(existing) = blobs.get(path) {
        if existing.as_ref() != bytes.as_ref() {
            return Err(HtmlError::Invalid(format!(
                "content-addressed blob collision: {path}"
            )));
        }
    } else {
        blobs.insert(path.to_string(), bytes);
    }
    Ok(())
}

fn serialize_blobs(blobs: &BTreeMap<String, Blob>) -> Result<Vec<HtmlBlob>, HtmlError> {
    let mut total = 0usize;
    blobs
        .iter()
        .map(|(path, bytes)| {
            total = total
                .checked_add(bytes.len())
                .ok_or_else(|| HtmlError::Invalid("HTML blob byte count overflow".into()))?;
            if total > MAX_PPTX_SIZE.saturating_add(MAX_HTML_ASSET_BYTES) {
                return Err(HtmlError::Invalid(
                    "HTML content-addressed blobs exceed safety limit".into(),
                ));
            }
            let sha256 = sha256_hex(bytes.as_ref());
            let expected = opc_snapshot::blob_path_for_digest(&sha256);
            if path != &expected {
                return Err(HtmlError::Invalid(format!(
                    "non-content-addressed HTML blob path: {path}"
                )));
            }
            Ok(HtmlBlob {
                path: path.clone(),
                size: bytes.len(),
                sha256,
                base64: STANDARD.encode(bytes.as_ref()),
            })
        })
        .collect()
}

fn portableize_deck_assets(
    deck: &mut Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<BTreeSet<String>, HtmlError> {
    let mut referenced = BTreeSet::new();
    for font in &mut deck.fonts {
        portableize_string(&mut font.data_uri, cache_id, catalog, &mut referenced)?;
    }
    for slide in &mut deck.slides {
        portableize_optional(
            &mut slide.background_asset,
            cache_id,
            catalog,
            &mut referenced,
        )?;
        portableize_objects(
            &mut slide.master_objects,
            cache_id,
            catalog,
            &mut referenced,
        )?;
        portableize_objects(
            &mut slide.layout_objects,
            cache_id,
            catalog,
            &mut referenced,
        )?;
        portableize_objects(&mut slide.objects, cache_id, catalog, &mut referenced)?;
    }
    Ok(referenced)
}

fn portableize_objects(
    objects: &mut [SceneObject],
    cache_id: &str,
    catalog: &AssetCatalog,
    referenced: &mut BTreeSet<String>,
) -> Result<(), HtmlError> {
    for object in objects {
        portableize_object_asset(
            &mut object.asset,
            &mut object.style.fill,
            cache_id,
            catalog,
            referenced,
        )?;
        portableize_object_asset(
            &mut object.shape_fill_asset,
            &mut object.style.fill,
            cache_id,
            catalog,
            referenced,
        )?;
        if let Some(media) = &mut object.media {
            portableize_optional(&mut media.asset, cache_id, catalog, referenced)?;
            portableize_optional(&mut media.playback_asset, cache_id, catalog, referenced)?;
        }
        portableize_objects(&mut object.children, cache_id, catalog, referenced)?;
    }
    Ok(())
}

fn portableize_object_asset(
    value: &mut Option<String>,
    style_fill: &mut String,
    cache_id: &str,
    catalog: &AssetCatalog,
    referenced: &mut BTreeSet<String>,
) -> Result<(), HtmlError> {
    let previous = value
        .as_ref()
        .filter(|current| style_fill.contains(current.as_str()))
        .cloned();
    portableize_optional(value, cache_id, catalog, referenced)?;
    if let (Some(previous), Some(current)) = (previous, value.as_deref()) {
        *style_fill = style_fill.replace(&previous, current);
    }
    Ok(())
}

fn portableize_optional(
    value: &mut Option<String>,
    cache_id: &str,
    catalog: &AssetCatalog,
    referenced: &mut BTreeSet<String>,
) -> Result<(), HtmlError> {
    if let Some(value) = value {
        portableize_string(value, cache_id, catalog, referenced)?;
    }
    Ok(())
}

fn portableize_string(
    value: &mut String,
    cache_id: &str,
    catalog: &AssetCatalog,
    referenced: &mut BTreeSet<String>,
) -> Result<(), HtmlError> {
    let cache_prefix = format!("/api/cache/{cache_id}/asset/");
    let id = if let Some(id) = value.strip_prefix(&cache_prefix) {
        Some(id.to_string())
    } else if let Some(id) = value.strip_prefix(PORTABLE_ASSET_PREFIX) {
        Some(id.to_string())
    } else {
        if value.starts_with("/api/cache/") {
            return Err(HtmlError::Asset(
                "scene contains an asset reference from another document cache".into(),
            ));
        }
        None
    };
    let Some(id) = id else {
        return Ok(());
    };
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HtmlError::Asset(format!("invalid asset id: {id}")));
    }
    if catalog.get(&id).is_none() {
        return Err(HtmlError::Asset(format!(
            "cached scene asset is missing: {id}"
        )));
    }
    referenced.insert(id.clone());
    *value = format!("{PORTABLE_ASSET_PREFIX}{id}");
    Ok(())
}

#[allow(dead_code)]
pub fn decode(bytes: &[u8]) -> Result<DecodedHtml, HtmlError> {
    let mut decoded = decode_compact(bytes)?;
    let presentation = match decoded.presentation.take() {
        Some(presentation) => presentation,
        None => decoded.opc.rebuild().map_err(HtmlError::Invalid)?,
    };
    if decoded.version >= 3 {
        restore_deck_assets_from_catalog(&mut decoded.deck, &decoded.assets)?;
    }
    Ok(DecodedHtml {
        deck: decoded.deck,
        presentation,
    })
}

pub(crate) fn decode_compact(bytes: &[u8]) -> Result<DecodedCompactHtml, HtmlError> {
    let (mut payload, presentation, opc, blob_map) = decode_payload(bytes)?;
    let assets = if payload.version == 4 {
        asset_catalog_from_html_v4(&payload.assets, &blob_map)?
    } else if payload.version == 3 {
        asset_catalog_from_html_legacy(&payload.assets)?
    } else {
        AssetCatalog::default()
    };
    let original_deck = payload
        .baseline_deck
        .take()
        .unwrap_or_else(|| payload.deck.clone());
    let presentation_digest = if let Some(presentation) = presentation.as_deref() {
        Sha256::digest(presentation).into()
    } else {
        opc.digest()
    };
    Ok(DecodedCompactHtml {
        deck: payload.deck,
        original_deck,
        presentation,
        opc,
        presentation_digest,
        assets,
        version: payload.version,
    })
}

pub(crate) fn bind_cached_asset_refs(
    deck: &mut Deck,
    cache_id: &str,
    catalog: &AssetCatalog,
) -> Result<(), HtmlError> {
    let values = catalog
        .iter()
        .map(|(id, _)| (id.to_string(), format!("/api/cache/{cache_id}/asset/{id}")))
        .collect::<HashMap<_, _>>();
    restore_deck_references(deck, &values)
}

#[allow(clippy::type_complexity)]
fn decode_payload(
    bytes: &[u8],
) -> Result<
    (
        HtmlPayload,
        Option<Vec<u8>>,
        OpcSnapshot,
        BTreeMap<String, Blob>,
    ),
    HtmlError,
> {
    let html =
        std::str::from_utf8(bytes).map_err(|_| HtmlError::Invalid("文件不是 UTF-8 HTML".into()))?;
    let start = html
        .find(MARKER_OPEN)
        .map(|index| index + MARKER_OPEN.len())
        .ok_or_else(|| HtmlError::Invalid("缺少 application/x-unippt+json 标记".into()))?;
    let end = html[start..]
        .find(MARKER_CLOSE)
        .map(|index| start + index)
        .ok_or_else(|| HtmlError::Invalid("内嵌 UniPPT 数据没有闭合".into()))?;
    let payload_json = &html[start..end];
    let raw_payload: RawHtmlPayload = serde_json::from_str(payload_json)?;
    let payload: HtmlPayload = serde_json::from_str(payload_json)?;
    if payload.format != "unippt-html"
        || !matches!(payload.version, 1..=4)
        || payload.document_format != "udoc"
        || payload.unidoc_version != 3
        || !is_supported_unidoc_type(&payload.unidoc_type)
        || payload.app != "UniPPT"
        || payload.deck.format != "unippt"
    {
        return Err(HtmlError::Invalid(
            "必须声明 unidoc_type=\"pptx\"（兼容旧值 \"ppt\"）且 app=\"UniPPT\"".into(),
        ));
    }
    if payload.version == 4
        && (payload.scene_sha256.is_none()
            || payload.opc_sha256.is_none()
            || payload.baseline_deck.is_some() != payload.baseline_sha256.is_some())
    {
        return Err(HtmlError::Invalid(
            "v4 lossless HTML requires scene integrity and a hash-bound baseline".into(),
        ));
    }
    if let Some(expected) = payload.scene_sha256.as_deref() {
        let actual = sha256_hex(raw_payload.deck.get().as_bytes());
        if actual != expected {
            return Err(HtmlError::Invalid(format!(
                "embedded UniPPT scene SHA-256 mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    if let Some(expected) = payload.baseline_sha256.as_deref() {
        let raw = raw_payload.baseline_deck.as_ref().ok_or_else(|| {
            HtmlError::Invalid("baseline SHA-256 exists without baselineDeck".into())
        })?;
        let actual = sha256_hex(raw.get().as_bytes());
        if actual != expected {
            return Err(HtmlError::Invalid(format!(
                "embedded UniPPT baseline SHA-256 mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    if let Some(expected) = payload.opc_sha256.as_deref() {
        let raw = raw_payload
            .opc_package
            .as_ref()
            .ok_or_else(|| HtmlError::Invalid("OPC SHA-256 exists without opcPackage".into()))?;
        let actual = sha256_hex(raw.get().as_bytes());
        if actual != expected {
            return Err(HtmlError::Invalid(format!(
                "embedded OPC index SHA-256 mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    if payload.version == 4 {
        if payload.pptx_base64.is_some()
            || payload.pptx_br.is_some()
            || payload.pptx_size.is_some()
            || payload.pptx_sha256.is_some()
        {
            return Err(HtmlError::Invalid(
                "v4 lossless HTML must not embed a complete PPTX".into(),
            ));
        }
        let blobs = decode_html_blobs(&payload.blobs)?;
        let index = payload
            .opc_package
            .clone()
            .ok_or_else(|| HtmlError::Invalid("v4 lossless HTML is missing opcPackage".into()))?;
        let opc = opc_snapshot::hydrate(index, |path| blobs.get(path).cloned())
            .map_err(HtmlError::Invalid)?;
        return Ok((payload, None, opc, blobs));
    }
    let presentation = if payload.version >= 2 {
        let expected = payload
            .pptx_size
            .filter(|size| *size <= MAX_PPTX_SIZE)
            .ok_or_else(|| HtmlError::Invalid("PPTX Brotli 原始尺寸缺失或越界".into()))?;
        let packed = STANDARD
            .decode(
                payload
                    .pptx_br
                    .as_deref()
                    .ok_or_else(|| HtmlError::Invalid("v2 无损 HTML 缺少 pptxBr".into()))?,
            )
            .map_err(|error| HtmlError::Invalid(format!("PPTX Brotli Base64 损坏：{error}")))?;
        br_decompress(&packed, expected)?
    } else {
        STANDARD
            .decode(
                payload
                    .pptx_base64
                    .as_deref()
                    .ok_or_else(|| HtmlError::Invalid("v1 无损 HTML 缺少 pptxBase64".into()))?,
            )
            .map_err(|error| HtmlError::Invalid(format!("PPTX 无损载荷损坏：{error}")))?
    };
    if !presentation.starts_with(b"PK") {
        return Err(HtmlError::Invalid("PPTX 无损载荷不是 OPC ZIP".into()));
    }
    if let Some(expected) = payload.pptx_sha256.as_deref() {
        let actual = sha256_hex(&presentation);
        if actual != expected {
            return Err(HtmlError::Invalid(format!(
                "embedded native PPTX SHA-256 mismatch: expected {expected}, got {actual}"
            )));
        }
    }
    let opc = opc_snapshot::explode(&presentation).map_err(HtmlError::Invalid)?;
    Ok((payload, Some(presentation), opc, BTreeMap::new()))
}

fn asset_catalog_from_html_legacy(assets: &[HtmlAsset]) -> Result<AssetCatalog, HtmlError> {
    if assets.len() > MAX_HTML_ASSET_COUNT {
        return Err(HtmlError::Asset(format!(
            "too many embedded assets: {}",
            assets.len()
        )));
    }
    let mut catalog = AssetCatalog::default();
    let mut ids = BTreeSet::new();
    let mut total = 0usize;
    for asset in assets {
        if !ids.insert(asset.id.clone()) {
            return Err(HtmlError::Asset(format!(
                "duplicate embedded asset: {}",
                asset.id
            )));
        }
        if asset.browser_mime_type.is_some() != asset.browser_base64.is_some() {
            return Err(HtmlError::Asset(format!(
                "incomplete browser projection for asset {}",
                asset.id
            )));
        }
        if asset.original_blob.is_some() || asset.browser_blob.is_some() {
            return Err(HtmlError::Asset(format!(
                "legacy HTML asset unexpectedly references a blob: {}",
                asset.id
            )));
        }
        let original_base64 = asset.original_base64.as_deref().ok_or_else(|| {
            HtmlError::Asset(format!("asset {} is missing originalBase64", asset.id))
        })?;
        let bytes = STANDARD.decode(original_base64).map_err(|error| {
            HtmlError::Asset(format!("asset {} has invalid base64: {error}", asset.id))
        })?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| HtmlError::Asset("asset byte count overflow".into()))?;
        if total > MAX_HTML_ASSET_BYTES {
            return Err(HtmlError::Asset(format!(
                "embedded assets exceed {} bytes",
                MAX_HTML_ASSET_BYTES
            )));
        }
        catalog
            .insert_original_asset(&asset.id, &asset.mime_type, &asset.data_uri_prefix, bytes)
            .map_err(HtmlError::Asset)?;
    }
    Ok(catalog)
}

fn asset_catalog_from_html_v4(
    assets: &[HtmlAsset],
    blobs: &BTreeMap<String, Blob>,
) -> Result<AssetCatalog, HtmlError> {
    if assets.len() > MAX_HTML_ASSET_COUNT {
        return Err(HtmlError::Asset(format!(
            "too many embedded assets: {}",
            assets.len()
        )));
    }
    let mut catalog = AssetCatalog::default();
    let mut ids = BTreeSet::new();
    let mut total = 0usize;
    for asset in assets {
        if !ids.insert(asset.id.clone()) {
            return Err(HtmlError::Asset(format!(
                "duplicate embedded asset: {}",
                asset.id
            )));
        }
        if asset.original_base64.is_some() || asset.browser_base64.is_some() {
            return Err(HtmlError::Asset(format!(
                "v4 asset contains legacy inline base64: {}",
                asset.id
            )));
        }
        let path = asset.original_blob.as_deref().ok_or_else(|| {
            HtmlError::Asset(format!("v4 asset {} is missing originalBlob", asset.id))
        })?;
        let raw = blobs.get(path).cloned().ok_or_else(|| {
            HtmlError::Asset(format!("asset {} references missing blob {path}", asset.id))
        })?;
        total = total
            .checked_add(raw.len())
            .ok_or_else(|| HtmlError::Asset("asset byte count overflow".into()))?;
        if total > MAX_HTML_ASSET_BYTES {
            return Err(HtmlError::Asset(format!(
                "embedded assets exceed {} bytes",
                MAX_HTML_ASSET_BYTES
            )));
        }
        if asset.browser_mime_type.is_some() != asset.browser_blob.is_some() {
            return Err(HtmlError::Asset(format!(
                "incomplete browser projection for asset {}",
                asset.id
            )));
        }
        if let Some(browser_path) = asset.browser_blob.as_deref() {
            let _ = blobs.get(browser_path).ok_or_else(|| {
                HtmlError::Asset(format!(
                    "asset {} references missing browser blob {browser_path}",
                    asset.id
                ))
            })?;
        }
        catalog
            .insert_original_asset_arc(&asset.id, &asset.mime_type, &asset.data_uri_prefix, raw)
            .map_err(HtmlError::Asset)?;
        let cached = catalog
            .get(&asset.id)
            .expect("asset was inserted immediately above");
        match (
            asset.browser_mime_type.as_deref(),
            asset.browser_blob.as_deref(),
        ) {
            (Some(mime), Some(path)) => {
                let browser = blobs.get(path).expect("browser blob validated above");
                if mime != cached.browser_mime_type.as_ref()
                    || browser.as_ref() != cached.browser_bytes.as_ref()
                {
                    return Err(HtmlError::Asset(format!(
                        "browser projection mismatch for asset {}",
                        asset.id
                    )));
                }
            }
            (None, None)
                if cached.mime_type.as_ref() != cached.browser_mime_type.as_ref()
                    || !Arc::ptr_eq(&cached.bytes, &cached.browser_bytes) =>
            {
                return Err(HtmlError::Asset(format!(
                    "missing browser projection for asset {}",
                    asset.id
                )));
            }
            _ => {}
        }
    }
    Ok(catalog)
}

fn decode_html_blobs(blobs: &[HtmlBlob]) -> Result<BTreeMap<String, Blob>, HtmlError> {
    let mut decoded = BTreeMap::new();
    let mut total = 0usize;
    for blob in blobs {
        let raw = STANDARD.decode(&blob.base64).map_err(|error| {
            HtmlError::Invalid(format!("blob {} has invalid base64: {error}", blob.path))
        })?;
        total = total
            .checked_add(raw.len())
            .ok_or_else(|| HtmlError::Invalid("HTML blob byte count overflow".into()))?;
        if total > MAX_PPTX_SIZE.saturating_add(MAX_HTML_ASSET_BYTES) {
            return Err(HtmlError::Invalid(
                "HTML content-addressed blobs exceed safety limit".into(),
            ));
        }
        let actual = sha256_hex(&raw);
        let expected_path = opc_snapshot::blob_path_for_digest(&actual);
        if raw.len() != blob.size || actual != blob.sha256 || blob.path != expected_path {
            return Err(HtmlError::Invalid(format!(
                "HTML blob integrity mismatch: {}",
                blob.path
            )));
        }
        if decoded.insert(blob.path.clone(), Arc::from(raw)).is_some() {
            return Err(HtmlError::Invalid(format!(
                "duplicate HTML blob: {}",
                blob.path
            )));
        }
    }
    Ok(decoded)
}

#[allow(dead_code)]
fn restore_deck_assets_from_catalog(
    deck: &mut Deck,
    catalog: &AssetCatalog,
) -> Result<(), HtmlError> {
    let data_uris = catalog
        .iter()
        .map(|(id, asset)| {
            (
                id.to_string(),
                format!(
                    "{}{}",
                    asset.data_uri_prefix,
                    STANDARD.encode(asset.bytes.as_ref())
                ),
            )
        })
        .collect::<HashMap<_, _>>();
    restore_deck_references(deck, &data_uris)
}

fn restore_deck_references(
    deck: &mut Deck,
    data_uris: &HashMap<String, String>,
) -> Result<(), HtmlError> {
    for font in &mut deck.fonts {
        restore_string(&mut font.data_uri, data_uris)?;
    }
    for slide in &mut deck.slides {
        restore_optional(&mut slide.background_asset, data_uris)?;
        restore_objects(&mut slide.master_objects, data_uris)?;
        restore_objects(&mut slide.layout_objects, data_uris)?;
        restore_objects(&mut slide.objects, data_uris)?;
    }
    Ok(())
}

fn restore_objects(
    objects: &mut [SceneObject],
    data_uris: &HashMap<String, String>,
) -> Result<(), HtmlError> {
    for object in objects {
        restore_object_asset(&mut object.asset, &mut object.style.fill, data_uris)?;
        restore_object_asset(
            &mut object.shape_fill_asset,
            &mut object.style.fill,
            data_uris,
        )?;
        if let Some(media) = &mut object.media {
            restore_optional(&mut media.asset, data_uris)?;
            restore_optional(&mut media.playback_asset, data_uris)?;
        }
        restore_objects(&mut object.children, data_uris)?;
    }
    Ok(())
}

fn restore_object_asset(
    value: &mut Option<String>,
    style_fill: &mut String,
    data_uris: &HashMap<String, String>,
) -> Result<(), HtmlError> {
    let previous = value
        .as_ref()
        .filter(|current| style_fill.contains(current.as_str()))
        .cloned();
    restore_optional(value, data_uris)?;
    if let (Some(previous), Some(current)) = (previous, value.as_deref()) {
        *style_fill = style_fill.replace(&previous, current);
    }
    Ok(())
}

fn restore_optional(
    value: &mut Option<String>,
    data_uris: &HashMap<String, String>,
) -> Result<(), HtmlError> {
    if let Some(value) = value {
        restore_string(value, data_uris)?;
    }
    Ok(())
}

fn restore_string(
    value: &mut String,
    data_uris: &HashMap<String, String>,
) -> Result<(), HtmlError> {
    let Some(id) = value.strip_prefix(PORTABLE_ASSET_PREFIX) else {
        return Ok(());
    };
    *value = data_uris
        .get(id)
        .cloned()
        .ok_or_else(|| HtmlError::Asset(format!("embedded scene asset is missing: {id}")))?;
    Ok(())
}

#[allow(dead_code)]
fn br_compress(input: &[u8]) -> Result<Vec<u8>, HtmlError> {
    let params = brotli::enc::BrotliEncoderParams {
        quality: 5,
        lgwin: 22,
        ..Default::default()
    };
    let mut output = Vec::new();
    let mut reader = input;
    brotli::BrotliCompress(&mut reader, &mut output, &params)
        .map_err(|error| HtmlError::Brotli(error.to_string()))?;
    Ok(output)
}

fn sha256_hex(input: &[u8]) -> String {
    Sha256::digest(input)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn br_decompress(input: &[u8], expected: usize) -> Result<Vec<u8>, HtmlError> {
    if expected > MAX_PPTX_SIZE {
        return Err(HtmlError::Invalid(
            "PPTX Brotli 解压尺寸超过安全上限".into(),
        ));
    }
    let mut output = Vec::with_capacity(expected.min(8 * 1024 * 1024));
    let decoder = brotli::Decompressor::new(std::io::Cursor::new(input), 64 * 1024);
    decoder
        .take(expected.saturating_add(1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| HtmlError::Brotli(error.to_string()))?;
    if output.len() != expected {
        return Err(HtmlError::Invalid(format!(
            "PPTX Brotli 解压尺寸不匹配：声明 {expected}，实际 {}",
            output.len()
        )));
    }
    Ok(output)
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const HTML_TEMPLATE: &str = r##"<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>__TITLE__</title><style>
html,body{margin:0;width:100%;height:100%;overflow:hidden;background:#090909;font-family:"Segoe UI","Microsoft YaHei",sans-serif;overscroll-behavior:none;touch-action:none;-webkit-touch-callout:none;-webkit-user-select:none;user-select:none}.stage{position:absolute;left:50%;top:50%;overflow:hidden;transform-origin:center;background:#fff}.stage,.stage *{-webkit-user-select:none;user-select:none;-webkit-user-drag:none}.o{position:absolute;display:flex;box-sizing:border-box;white-space:pre-wrap;overflow:hidden;transform-origin:center;line-height:1.18}.o.group{display:block;padding:0;overflow:visible;border:0!important;background:transparent!important}.o img{width:100%;height:100%;object-fit:cover;pointer-events:none;-webkit-user-drag:none}.o.math{justify-content:center;overflow:visible}.content{width:100%;min-width:0}.p{position:relative;min-height:1em;margin:0;white-space:pre-wrap}.bullet{display:inline-block;min-width:1.15em;margin-left:-1.15em}.run{white-space:inherit}.nav{position:fixed;left:50%;bottom:max(18px,calc(env(safe-area-inset-bottom) + 10px));z-index:1000;display:flex;align-items:center;gap:2px;max-width:calc(100vw - 16px);min-height:38px;padding:5px 7px;transform:translate3d(-50%,8px,0);border:1px solid #ffffff20;border-radius:999px;background:#242424d9;color:#fff;box-shadow:0 7px 24px #0008;backdrop-filter:blur(14px);opacity:0;visibility:hidden;pointer-events:none;transition:opacity .18s ease,transform .18s ease,visibility 0s linear .18s}.nav.visible,.nav:focus-within{transform:translate3d(-50%,0,0);opacity:.94;visibility:visible;pointer-events:auto;transition-delay:0s}.nav button{display:grid;place-items:center;width:32px;height:32px;min-width:32px;padding:0;border:0;border-radius:50%;background:transparent;color:#f7f7f7;font:300 24px/1 "Segoe UI Symbol","Segoe UI",sans-serif;cursor:pointer}.nav button:hover,.nav button:focus-visible{background:#ffffff24;outline:none}.nav button:active{background:#ffffff36;transform:scale(.94)}.nav button:disabled{opacity:.35;cursor:default}.nav .page-jump{width:27px;min-width:27px;color:#e7e7e7;font-size:16px;font-weight:400}.nav .auto-play{margin-left:3px;border-left:1px solid #ffffff2b;border-radius:0 999px 999px 0;font-size:15px}.nav .auto-play.active,.nav .auto-play[aria-pressed="true"]{background:#ffffff2c;box-shadow:inset 0 0 0 1px #ffffff24}.nav .count{min-width:58px;padding:0 4px;text-align:center;color:#eee;font:400 13px/1 "Segoe UI",sans-serif;font-variant-numeric:tabular-nums}.nav .audio,.nav .zoom{display:flex;align-items:center;gap:2px;margin-left:5px;padding-left:7px;border-left:1px solid #ffffff2b}.nav .audio{gap:4px}.nav .audio button{font-size:16px}.nav .zoom button{width:27px;min-width:27px;font-size:17px}.nav .zoom-output{min-width:42px;padding:0 2px;text-align:center;color:#eee;font:400 11px/1 "Segoe UI",sans-serif;font-variant-numeric:tabular-nums;cursor:pointer}.nav input[type=range]{width:116px;height:18px;margin:0 5px 0 0;accent-color:#f1f1f1;cursor:pointer;touch-action:pan-x}.nav .close{font-size:24px}.progress{position:fixed;left:0;bottom:0;height:3px;background:#d35230;transition:width .25s}.formula{font-family:Cambria Math,serif;font-style:italic}@media(max-width:700px){.nav{gap:0;padding:4px}.nav button{width:28px;min-width:28px}.nav .count{min-width:48px}.nav input[type=range]{width:70px}.nav .zoom-output{min-width:36px}.nav .audio,.nav .zoom{margin-left:2px;padding-left:3px}}@media(max-width:470px){.nav .audio input[type=range]{display:none}.nav .audio{padding-right:2px}}
</style></head><body><div id="stage" class="stage"></div><div id="playerControls" class="nav" role="toolbar" aria-label="幻灯片放映控制"><button id="prevSlide" class="page-jump" type="button" title="上一页（Ctrl+Home）" aria-label="上一页" aria-keyshortcuts="Control+Home">&#8676;</button><button id="prev" type="button" title="上一个动画节点（左方向键）" aria-label="上一个动画节点或上一张幻灯片">&#8249;</button><span id="count" class="count"></span><button id="next" type="button" title="下一个动画节点（右方向键或空格）" aria-label="下一个动画节点或下一张幻灯片">&#8250;</button><button id="nextSlide" class="page-jump" type="button" title="下一页（Ctrl+End）" aria-label="下一页" aria-keyshortcuts="Control+End">&#8677;</button><button id="playerAutoPlay" class="auto-play" type="button" title="自动放映：本页全部动画完成后自动换页（A）" aria-label="自动放映" aria-pressed="false" aria-keyshortcuts="A">&#9654;</button><span class="audio"><button id="playerMute" type="button" aria-label="静音" aria-pressed="false"><span id="playerVolumeGlyph" aria-hidden="true">🔊</span></button><input id="playerVolume" type="range" min="0" max="100" step="1" value="100" aria-label="放映音量" aria-valuetext="100%"></span><span class="zoom" aria-label="放映缩放"><button id="playerZoomOut" type="button" title="缩小（Ctrl+-）" aria-label="缩小">&#8722;</button><button id="playerZoomValue" class="zoom-output" type="button" title="恢复 100%（Ctrl+0）" aria-label="放映缩放 100%">100%</button><button id="playerZoomIn" type="button" title="放大（Ctrl++）" aria-label="放大">&#43;</button></span><button id="playerClose" class="close" type="button" title="退出放映（Esc）" aria-label="退出幻灯片放映">&times;</button></div><div id="progress" class="progress"></div>
  <script type="application/x-unippt+json" id="unippt-data">__DATA__</script><script>__CHART_RUNTIME__</script><script>__PRESENTATION_SCENE_RUNTIME__</script><script>__PRESET_ANIMATION_RUNTIME__</script><script>__MOTION_PATH_RUNTIME__</script><script>__MEDIA_RUNTIME__</script><script>__PRESENTATION_FLOW_RUNTIME__</script><script>__PRESENTATION_TRANSITION_RUNTIME__</script><script>
(()=>{'use strict';const payload=JSON.parse(document.getElementById('unippt-data').textContent),d=payload.deck,s=document.getElementById('stage'),count=document.getElementById('count'),progress=document.getElementById('progress'),prev=document.getElementById('prev'),next=document.getElementById('next');let page=0,cursor=0,timers=[];const effects=()=>[...(d.slides[page].animations||[])].sort((a,b)=>(a.order||0)-(b.order||0));function clear(){timers.forEach(clearTimeout);timers=[]}function box(e,o,z){const f=o.frame||{},ts=o.textStyle||{},st=o.style||{},tf=o.textFrame||{};const base='rotate('+(f.rotation||0)+'deg) scaleX('+(o.flipH?-1:1)+') scaleY('+(o.flipV?-1:1)+')';e.dataset.id=o.id;e.dataset.base=base;Object.assign(e.style,{zIndex:z+1,left:(f.x||0)+'px',top:(f.y||0)+'px',width:Math.max(1,f.width||1)+'px',height:Math.max(1,f.height||1)+'px',transform:base,background:st.fill||'transparent',border:(st.strokeWidth||0)+'px solid '+(st.stroke||'transparent'),opacity:st.opacity??1,color:ts.color||'#172033',fontFamily:ts.fontFamily||'Aptos,sans-serif',fontSize:(ts.fontSize||24)+'px',fontWeight:ts.bold?'700':'400',fontStyle:ts.italic?'italic':'normal',textAlign:ts.align||'left',justifyContent:ts.align==='center'?'center':ts.align==='right'?'flex-end':'flex-start',alignItems:tf.verticalAlign==='top'?'flex-start':tf.verticalAlign==='bottom'?'flex-end':'center',padding:(tf.marginTop??5)+'px '+(tf.marginRight??8)+'px '+(tf.marginBottom??5)+'px '+(tf.marginLeft??8)+'px'})}function rich(e,o){const flow=document.createElement('div');flow.className='content';for(const p of o.textParagraphs||[]){const line=document.createElement('div');line.className='p';line.style.textAlign=p.align||(o.textStyle||{}).align||'left';line.style.paddingLeft=Math.max(0,p.level||0)*24+'px';if(p.lineSpacing)line.style.lineHeight=String(p.lineSpacing);if(p.spaceBefore!=null)line.style.marginTop=p.spaceBefore+'px';if(p.spaceAfter!=null)line.style.marginBottom=p.spaceAfter+'px';if(p.bullet){const b=document.createElement('span');b.className='bullet';b.textContent=p.bullet;line.append(b)}for(const r of p.runs||[]){const n=document.createElement('span');n.className='run';n.textContent=r.text||'';Object.assign(n.style,{fontFamily:r.fontFamily||(o.textStyle||{}).fontFamily,fontSize:(r.fontSize||(o.textStyle||{}).fontSize||24)+'px',color:r.color||(o.textStyle||{}).color,fontWeight:r.bold?'700':'400',fontStyle:r.italic?'italic':'normal',textDecoration:(r.underline?'underline ':'')+(r.strikethrough?'line-through':''),verticalAlign:r.baseline==='super'?'super':r.baseline==='sub'?'sub':'baseline'});line.append(n)}flow.append(line)}e.append(flow)}function add(o,z,parent){const e=document.createElement('div');e.className='o'+(o.kind==='group'?' group':o.kind==='math'?' math formula':'');box(e,o,z);parent.append(e);if(o.kind==='group'){(o.children||[]).forEach((child,index)=>add(child,index,e));return}if(o.kind==='image'&&o.asset){const im=new Image;im.src=o.asset;e.append(im)}else if(o.kind==='math'&&o.formula)e.textContent=o.formula.latex||o.text||'π';else if((o.textParagraphs||[]).length)rich(e,o);else e.textContent=o.text||(['chart','table','smartArt'].includes(o.kind)?o.name:'')}function transition(sl){const t=sl.transition;if(!t)return;const kind=t.kind||'fade',dur=Math.max(1,t.durationMs||700);let frames;if(['circle','diamond','plus','wedge','wheel'].includes(kind))frames=[{clipPath:'circle(0% at 50% 50%)'},{clipPath:'circle(75% at 50% 50%)'}];else if(['wipe','push','pull','cover','strips','split','blinds','checker','randomBar','comb'].includes(kind))frames=[{clipPath:'inset(0 0 0 100%)',opacity:.45},{clipPath:'inset(0 0 0 0)',opacity:1}];else if(kind!=='cut'&&kind!=='none')frames=[{opacity:0},{opacity:1}];if(frames)s.animate(frames,{duration:dur,easing:'ease-out',fill:'both'})}function draw(playTransition){clear();cursor=0;s.replaceChildren();const sl=d.slides[page];s.style.width=d.width+'px';s.style.height=d.height+'px';s.style.background=sl.background||'#fff';s.style.backgroundImage=sl.backgroundAsset?'url('+JSON.stringify(sl.backgroundAsset)+')':'none';s.style.backgroundSize='100% 100%';s.style.backgroundPosition='center';s.style.backgroundRepeat='no-repeat';(sl.objects||[]).forEach((o,z)=>add(o,z,s));for(const a of effects()){if(a.class==='entrance'){const e=s.querySelector('[data-id="'+CSS.escape(a.targetObjectId||'')+'"]');if(e){e.style.visibility='hidden';e.style.opacity='0'}}}scale();status();if(playTransition)transition(sl);if(sl.transition&&sl.transition.advanceAfterMs!=null)timers.push(setTimeout(advance,Math.max(1,sl.transition.advanceAfterMs)))}function scale(){const q=Math.min(innerWidth/d.width,innerHeight/d.height);s.style.transform='translate(-50%,-50%) scale('+q+')'}function status(){const es=effects();count.textContent=(page+1)+' / '+d.slides.length+(es.length?' · '+cursor+'/'+es.length:'');progress.style.width=((page+(es.length?cursor/Math.max(1,es.length):0))/d.slides.length*100)+'%'}function frames(a,e){const base=e.dataset.base||'',exit=a.class==='exit',kind=a.effect||'fade';let f;if(kind==='flyIn'){const dir=a.direction||'left',x=dir==='left'?'-18%':dir==='right'?'18%':'0',y=dir==='up'?'-18%':dir==='down'?'18%':'0';f=[{opacity:0,transform:base+' translate('+x+','+y+')'},{opacity:1,transform:base+' translate(0,0)'}]}else if(kind==='wipe')f=[{clipPath:'inset(0 0 0 100%)',opacity:1},{clipPath:'inset(0 0 0 0)',opacity:1}];else if(kind==='zoom')f=[{opacity:0,transform:base+' scale(.25)'},{opacity:1,transform:base+' scale(1)'}];else if(kind==='spin')f=[{transform:base+' rotate(0deg)'},{transform:base+' rotate(360deg)'}];else if(kind==='growShrink')f=[{transform:base+' scale(1)'},{transform:base+' scale(1.28)'},{transform:base+' scale(1)'}];else if(kind==='motionPath')f=[{transform:base+' translate(0,0)'},{transform:base+' translate(15%,0)'}];else f=[{opacity:0},{opacity:1}];return exit?f.reverse():f}function play(a,offset){const e=s.querySelector('[data-id="'+CSS.escape(a.targetObjectId||'')+'"]');if(!e)return 0;const delay=(a.delayMs||0)+offset,dur=Math.max(1,a.durationMs||500);const timer=setTimeout(()=>{e.style.visibility='visible';const player=e.animate(frames(a,e),{duration:dur,easing:'ease',fill:'forwards'});player.onfinish=()=>{if(a.class==='exit'){e.style.visibility='hidden';e.style.opacity='0'}else e.style.opacity='1'}},delay);timers.push(timer);return delay+dur}function advance(){const es=effects();if(cursor>=es.length){if(page<d.slides.length-1){page++;draw(true)}return}let end=0;const first=es[cursor++];end=Math.max(end,play(first,0));while(cursor<es.length&&es[cursor].trigger!=='onClick'){const a=es[cursor++],offset=a.trigger==='afterPrevious'?end:0;end=Math.max(end,play(a,offset))}status()}prev.onclick=()=>{if(page>0){page--;draw(true)}};next.onclick=advance;s.onclick=advance;onkeydown=e=>{if(['ArrowRight','ArrowDown','PageDown',' '].includes(e.key)){e.preventDefault();advance()}if(['ArrowLeft','ArrowUp','PageUp'].includes(e.key)){e.preventDefault();prev.click()}};onresize=scale;draw(false)})();
</script></body></html>"##;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_transport::materialize_deck_assets;
    use std::io::Cursor;
    use unippt_core::EmbeddedFont;

    fn sample_pptx(marker: &[u8]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "[Content_Types].xml",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(marker).unwrap();
        writer.finish().unwrap().into_inner()
    }

    fn embedded_payload(html: &[u8]) -> HtmlPayload {
        let html = std::str::from_utf8(html).unwrap();
        let start = html.find(MARKER_OPEN).unwrap() + MARKER_OPEN.len();
        let end = start + html[start..].find(MARKER_CLOSE).unwrap();
        serde_json::from_str(&html[start..end]).unwrap()
    }

    #[test]
    fn lossless_html_v4_round_trip_contains_opc_parts_without_native_pptx_blob() {
        let mut deck = Deck::demo();
        deck.extensions.insert(
            "org.unippt.ai".into(),
            serde_json::json!({"schemaVersion": 1, "slides": {"slide-1": {"role": "cover"}}}),
        );
        deck.fonts.push(EmbeddedFont {
            family: "UniPPT Test Face".into(),
            weight: 700,
            style: "italic".into(),
            data_uri: "data:font/ttf;base64,AAEAAA==".into(),
            mime_type: "font/ttf".into(),
            format: "truetype".into(),
            sha256: None,
            md5: None,
            byte_size: None,
            postscript_name: None,
            face_index: None,
            source_part_name: Some("/ppt/fonts/font1.fntdata".into()),
            source_relationship_id: Some("rIdFont1".into()),
            font_key: None,
        });
        let pptx = sample_pptx(b"native presentation");
        let html = encode(&deck, &pptx).unwrap();
        let text = std::str::from_utf8(&html).unwrap();
        assert!(text.contains("application/x-unippt+json"));
        assert!(text.contains("\"unidocType\":\"pptx\""));
        assert!(text.contains("\"documentFormat\":\"udoc\""));
        assert!(text.contains("\"unidocVersion\":3"));
        assert!(text.contains("\"version\":4"));
        assert!(text.contains("\"sceneSha256\":"));
        assert!(text.contains("\"opcPackage\":"));
        assert!(text.contains("\"blobs\":["));
        assert!(!text.contains("\"pptxSha256\":"));
        assert!(!text.contains("\"pptxBr\":"));
        assert!(!text.contains("\"pptxBase64\":"));
        assert!(text.contains("\"assets\":["));
        assert!(text.contains("\"dataUriPrefix\":\"data:font/ttf;base64,\""));
        assert!(text.contains("\"originalBlob\":\"blobs/sha256/"));
        assert!(!text.contains("\"originalBase64\":"));
        assert!(!text.contains("\"dataUri\":\"data:font/ttf;base64,AAEAAA==\""));
        assert!(text.contains("global.UniPptPresentationScene = Object.freeze"));
        assert!(text.contains("UniPptPresentationScene.renderSlide(sl,s,{clear:false"));
        assert!(!text.contains(
            "[...(sl.masterObjects||[]),...(sl.layoutObjects||[]),...(sl.objects||[])].forEach((o,z)=>add(o,z,s));"
        ));
        assert!(text.contains(
            "UniPptPresetAnimation?.frames(a,{node:e,slideWidth:d.width,slideHeight:d.height,baseTransform:base})"
        ));
        assert!(!text.contains("__PRESENTATION_SCENE_RUNTIME__"));
        assert!(!text.contains("return exit=f.reverse():f"));
        assert!(text.contains("return exit?f.reverse():f"));
        assert!(text.contains("function dash(v)"));
        assert!(text.contains("boxShadow:shadow(st.shadow)"));
        assert!(text.contains("textDecorationStyle:"));
        assert!(text.contains("opacity:r.alpha??1"));
        assert!(text.contains("function textGradient(n,r)"));
        assert!(text.contains("textGradient(n,r);wire(n,r);line.append(n)"));
        assert!(text.contains("webkitTextFillColor='transparent'"));
        assert!(text.contains("function safe(u)"));
        assert!(text.contains("wire(e,o);custom(e,o);parent.append(e)"));
        assert!(text.contains("wire(n,r);line.append(n)"));
        assert!(!text.contains("javascript:|data:"));
        assert!(text.contains("function table(e,o)"));
        assert!(text.contains("if(c.hMerge||c.vMerge)return"));
        assert!(text.contains("o.kind==='table'&&o.table"));
        assert!(text.contains(".table-grid{width:100%;height:100%;display:grid"));
        assert!(text.contains("function control(e,a)"));
        assert!(text.contains("function media(e,o)"));
        assert!(text.contains("globalThis.UniPptMedia?.controlNode(e,a)"));
        assert!(text.contains("globalThis.UniPptMedia?.attach(e,o,false"));
        assert!(text.contains("audio.native-media,video.native-media"));
        assert!(text.contains("m.playbackAsset||m.asset"));
        assert!(!text.contains("n.src=m.asset"));
        assert!(text.contains("a.mediaAction||'play'"));
        assert!(text.contains("UniPptPresentationFlow"));
        assert!(text.contains("UniPptPresentationTransitions"));
        assert!(text.contains("function scheduleAnimationBatch(es,startIndex=0)"));
        assert!(text.contains("groupStart + delay"));
        assert!(text.contains("groupEnd + delay"));
        assert!(text.contains("function scheduleAnimationNavigation(es,startIndex=0)"));
        assert!(text.contains("function startAutomaticBatch(startOffset=0)"));
        assert!(text.contains("function finishAnimationBatch()"));
        assert!(text.contains(
            "function advance(){hideLosslessEndNotice();const es=effects();if(finishAnimationBatch())return"
        ));
        assert!(text.contains("function rollback(){if(cancelAnimationBatch())return"));
        assert!(text.contains("function draw(playTransition=false,options={})"));
        assert!(text.contains("runtime?.captureUnderlay(s)"));
        assert!(text.contains("function previousPlayerStep()"));
        assert!(text.contains("function jumpPlayerPage(delta)"));
        assert!(text.contains("function showLosslessEndNotice()"));
        assert!(text.contains("已经到底了"));
        assert!(text.contains("installLosslessPlayerControls();onresize=scale;draw(false)"));
        assert!(text.contains("id=\"prevSlide\""));
        assert!(text.contains("id=\"nextSlide\""));
        assert!(text.contains("id=\"playerMute\""));
        assert!(text.contains("id=\"playerVolume\""));
        assert!(text.contains("id=\"playerZoomOut\""));
        assert!(text.contains("id=\"playerZoomValue\""));
        assert!(text.contains("id=\"playerZoomIn\""));
        assert!(text.contains("id=\"playerClose\""));
        assert!(text.contains("function installLosslessBrowserGuards()"));
        assert!(text.contains("touch-action:none"));
        assert!(text.contains("typeof playerZoom==='number'?playerZoom:1"));
        assert!(text.contains("event.ctrlKey && event.key === \"Home\""));
        assert!(text.contains("event.ctrlKey && event.key === \"End\""));
        assert!(!text.contains("function preview(lead)"));
        assert!(!text.contains("directNext(true)"));
        assert!(text.contains("unippt-persistent-media"));
        assert!(text.contains("media.dataset.playAcrossSlides = descriptor.playAcrossSlides"));
        assert!(text.contains("activeAcrossByOwner"));
        assert!(text.contains("command === \"toggle\" && !media.paused"));
        assert!(text.contains("effectivePlaybackEnd"));
        assert!(text.contains("token!==epoch"));
        assert!(text.contains("players.forEach"));
        assert!(!text.contains("forEach(n=>n.pause())"));
        assert!(text.contains("d.slides[page].inheritedAnimations||[]"));
        assert!(text.contains("UniPptPresentationScene.renderSlide(sl,s,{clear:false"));
        assert!(text.contains("cropImage(im,o.imageCrop)"));
        assert!(text.contains("function imageEffects(e,n,o)"));
        assert!(text.contains("o.imageEffects&&o.imageEffects.duotone"));
        assert!(text.contains("effects.softEdgeRadius"));
        assert!(text.contains("maskComposite:'intersect'"));
        assert!(text.contains("webkitMaskComposite:'source-in'"));
        assert!(text.contains("imageEffects(e,im,o)"));
        assert!(text.contains("function applyImageFill(e,o)"));
        assert!(text.contains("function imageFillMetrics(crop,fillRect"));
        assert!(text.contains("applyImageFill(e,o);media(e,o)"));
        assert!(text.contains("n.className='shape-fill-image'"));
        assert!(text
            .contains("imageFillMetrics(o.imageCrop,o.imageFillRect,Math.max(1,w),Math.max(1,h))"));
        assert!(text.contains("ts.fontFamily||ts.nativeFontFamily"));
        assert!(text.contains("r.fontFamily||r.nativeFontFamily"));
        assert!(text.contains("globalThis.UniPPTChartRuntime.render(e,o.chart)"));
        assert!(text.contains("native-chart-svg"));
        assert!(text.contains("function custom(e,o)"));
        assert!(text.contains("o.customGeometry"));
        assert!(text.contains("o.shapeFillAsset"));
        assert!(text.contains("custom-geometry"));
        assert!(!text.contains("!(g.height>0)"));
        assert!(text.contains("!(w>0||h>0)"));
        assert!(text.contains("(h>0?0:-.5)"));
        assert!(text.contains("Math.max(1,h)"));
        assert!(text.contains("e.style.padding='0'"));
        assert!(text.contains("function fontFaces()"));
        assert!(text.contains("@font-face{font-family:"));
        assert!(text.contains("function resolveDeckAssets(deck,assets,blobs)"));
        assert!(text.contains("URL.createObjectURL(new Blob"));
        assert!(text.contains("src.url?new URL(src.url,document.baseURI).href"));
        assert!(text.contains("startsWith('blob:')"));
        assert!(text.contains("trustedAssetUrls.add(value)"));
        assert!(text.contains("URL.revokeObjectURL(value)"));
        assert!(text.contains("if(!f.family||!assetSafe(u))return null"));
        assert!(text.contains(
            "globalThis.UniPptPresetAnimation?.frames(a,{node:e,slideWidth:d.width,slideHeight:d.height,baseTransform:base})"
        ));
        assert!(text
            .contains("globalThis.UniPptMotionPath?.frames(a.motionPath,d.width,d.height,base)"));
        assert!(text.contains("randomBars"));
        assert!(text.contains("function parse(source)"));
        assert!(text.contains("单击启用音频"));
        assert!(text.contains("Number(sl.transition.advanceAfterMs)>0)later(advance"));
        assert!(!text.contains("advanceAfterMs!=null)timers.push"));
        let decoded = decode(&html).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(
            decoded.deck.extensions["org.unippt.ai"]["slides"]["slide-1"]["role"],
            "cover"
        );
        assert_eq!(
            opc_snapshot::explode(&decoded.presentation)
                .unwrap()
                .digest(),
            opc_snapshot::explode(&pptx).unwrap().digest()
        );
    }

    #[test]
    fn lossless_html_rejects_scene_or_content_addressed_blob_tampering() {
        let deck = Deck::demo();
        let pptx = sample_pptx(b"integrity protected presentation");
        let html = encode(&deck, &pptx).unwrap();
        let payload = embedded_payload(&html);
        let text = std::str::from_utf8(&html).unwrap();

        let scene_hash = payload.scene_sha256.unwrap();
        let damaged_scene = text.replacen(&scene_hash, &"0".repeat(64), 1);
        assert!(matches!(
            decode(damaged_scene.as_bytes()),
            Err(HtmlError::Invalid(message)) if message.contains("scene SHA-256 mismatch")
        ));

        let blob_base64 = payload.blobs[0].base64.clone();
        let mut damaged_base64 = blob_base64.clone().into_bytes();
        damaged_base64[0] = if damaged_base64[0] == b'A' {
            b'B'
        } else {
            b'A'
        };
        let damaged_pptx = text.replacen(
            &blob_base64,
            std::str::from_utf8(&damaged_base64).unwrap(),
            1,
        );
        assert!(matches!(
            decode(damaged_pptx.as_bytes()),
            Err(HtmlError::Invalid(message)) if message.contains("blob integrity mismatch")
        ));
    }

    #[test]
    fn decoder_accepts_legacy_ppt_type() {
        let deck = Deck::demo();
        let pptx = sample_pptx(b"legacy native presentation");
        let html = encode(&deck, &pptx).unwrap();
        let legacy = std::str::from_utf8(&html)
            .unwrap()
            .replace("\"unidocType\":\"pptx\"", "\"unidocType\":\"ppt\"");
        let decoded = decode(legacy.as_bytes()).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(
            opc_snapshot::explode(&decoded.presentation)
                .unwrap()
                .digest(),
            opc_snapshot::explode(&pptx).unwrap().digest()
        );
    }

    #[test]
    fn duplicate_assets_are_inlined_once_and_restore_every_reference() {
        let raw = vec![0x5a; 256 * 1024];
        let data_uri = format!("data:image/png;base64,{}", STANDARD.encode(raw));
        let mut deck = Deck::demo();
        deck.slides[0].background_asset = Some(data_uri.clone());
        for object in &mut deck.slides[0].objects {
            object.asset = Some(data_uri.clone());
            object.shape_fill_asset = Some(data_uri.clone());
            object.style.fill = format!("url(\"{data_uri}\") no-repeat");
        }
        let materialized_size = serde_json::to_vec(&deck).unwrap().len();
        let pptx = sample_pptx(b"duplicate asset presentation");
        let html = encode(&deck, &pptx).unwrap();
        let payload = embedded_payload(&html);

        assert_eq!(payload.version, 4);
        assert_eq!(payload.assets.len(), 1);
        let asset_blob = payload.assets[0].original_blob.as_deref().unwrap();
        let encoded_asset = &payload
            .blobs
            .iter()
            .find(|blob| blob.path == asset_blob)
            .unwrap()
            .base64;
        assert_eq!(
            std::str::from_utf8(&html)
                .unwrap()
                .matches(encoded_asset)
                .count(),
            1
        );
        assert!(html.len() * 3 < materialized_size);
        assert!(payload.deck.slides[0]
            .background_asset
            .as_deref()
            .unwrap()
            .starts_with(PORTABLE_ASSET_PREFIX));

        let decoded = decode(&html).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(
            opc_snapshot::explode(&decoded.presentation)
                .unwrap()
                .digest(),
            opc_snapshot::explode(&pptx).unwrap().digest()
        );
    }

    #[test]
    fn decoder_accepts_version_two_data_uri_payload() {
        let mut deck = Deck::demo();
        deck.fonts.push(EmbeddedFont {
            family: "Legacy Data URI".into(),
            weight: 400,
            style: "normal".into(),
            data_uri: "data:font/ttf;base64,AAEAAA==".into(),
            mime_type: "font/ttf".into(),
            format: "truetype".into(),
            sha256: None,
            md5: None,
            byte_size: None,
            postscript_name: None,
            face_index: None,
            source_part_name: None,
            source_relationship_id: None,
            font_key: None,
        });
        let pptx = sample_pptx(b"version two presentation");
        let payload = HtmlPayload {
            format: "unippt-html".into(),
            version: 2,
            document_format: default_document_format(),
            unidoc_version: default_unidoc_version(),
            unidoc_type: CURRENT_UNIDOC_TYPE.into(),
            app: "UniPPT".into(),
            deck: deck.clone(),
            baseline_deck: None,
            assets: Vec::new(),
            opc_package: None,
            blobs: Vec::new(),
            pptx_base64: None,
            pptx_br: Some(STANDARD.encode(br_compress(&pptx).unwrap())),
            pptx_size: Some(pptx.len()),
            scene_sha256: None,
            baseline_sha256: None,
            opc_sha256: None,
            pptx_sha256: None,
        };
        let legacy = render_html(&deck.title, &payload).unwrap();

        let decoded = decode(&legacy).unwrap();
        assert_eq!(decoded.deck, deck);
        assert_eq!(decoded.presentation, pptx);
    }

    #[test]
    fn decoder_accepts_legacy_version_three_pptx_br_payload() {
        let deck = Deck::demo();
        let pptx = sample_pptx(b"version three presentation");
        let payload = HtmlPayload {
            format: "unippt-html".into(),
            version: 3,
            document_format: default_document_format(),
            unidoc_version: default_unidoc_version(),
            unidoc_type: CURRENT_UNIDOC_TYPE.into(),
            app: "UniPPT".into(),
            deck: deck.clone(),
            baseline_deck: None,
            assets: Vec::new(),
            opc_package: None,
            blobs: Vec::new(),
            pptx_base64: None,
            pptx_br: Some(STANDARD.encode(br_compress(&pptx).unwrap())),
            pptx_size: Some(pptx.len()),
            scene_sha256: None,
            baseline_sha256: None,
            opc_sha256: None,
            pptx_sha256: Some(sha256_hex(&pptx)),
        };
        let legacy = render_html(&deck.title, &payload).unwrap();
        let reopened = decode_compact(&legacy).unwrap();
        assert_eq!(reopened.version, 3);
        assert_eq!(reopened.deck, deck);
        assert_eq!(reopened.presentation.as_deref(), Some(pptx.as_slice()));
    }

    #[test]
    fn version_four_preserves_dirty_scene_and_immutable_baseline() {
        let mut baseline = Deck::demo();
        baseline.source_import_id = Some("source-cache".into());
        let mut current = baseline.clone();
        current.title = "AI edited title".into();
        let catalog = AssetCatalog::default();
        let pptx = sample_pptx(b"dirty baseline presentation");
        let opc = opc_snapshot::explode(&pptx).unwrap();
        let html = encode_cached(&current, &baseline, "source-cache", &catalog, &opc).unwrap();
        let payload = embedded_payload(&html);
        assert_eq!(payload.version, 4);
        assert!(payload.baseline_deck.is_some());
        assert!(payload.baseline_sha256.is_some());

        let reopened = decode_compact(&html).unwrap();
        assert_eq!(reopened.deck.title, "AI edited title");
        assert_eq!(reopened.original_deck.title, baseline.title);
        assert_eq!(reopened.opc.digest(), opc.digest());
        assert!(reopened.presentation.is_none());
    }

    #[test]
    fn compact_decode_rebinds_without_materializing_repeated_assets() {
        let data_uri = format!(
            "data:image/png;base64,{}",
            STANDARD.encode(vec![0x33; 192 * 1024])
        );
        let mut original = Deck::demo();
        original.slides[0].background_asset = Some(data_uri.clone());
        for object in &mut original.slides[0].objects {
            object.asset = Some(data_uri.clone());
            object.shape_fill_asset = Some(data_uri.clone());
            object.style.fill = format!("url({data_uri})");
        }
        let materialized_size = serde_json::to_vec(&original).unwrap().len();
        let mut compact = original.clone();
        let mut catalog = AssetCatalog::default();
        externalize_deck_assets(&mut compact, "source-cache", &mut catalog).unwrap();
        let pptx = sample_pptx(b"compact reopen presentation");
        let opc = opc_snapshot::explode(&pptx).unwrap();
        let html = encode_cached(&compact, &compact, "source-cache", &catalog, &opc).unwrap();

        let mut decoded = decode_compact(&html).unwrap();
        assert_eq!(decoded.version, 4);
        assert_eq!(decoded.assets.len(), 1);
        assert!(serde_json::to_vec(&decoded.deck).unwrap().len() * 4 < materialized_size);
        assert!(decoded.deck.slides[0]
            .background_asset
            .as_deref()
            .unwrap()
            .starts_with(PORTABLE_ASSET_PREFIX));

        bind_cached_asset_refs(&mut decoded.deck, "reopened-cache", &decoded.assets).unwrap();
        let restored =
            materialize_deck_assets(&decoded.deck, "reopened-cache", &decoded.assets).unwrap();
        assert_eq!(restored, original);
        assert!(decoded.presentation.is_none());
        assert_eq!(decoded.opc.digest(), opc.digest());
    }

    #[test]
    #[ignore = "set UNIPPT_BENCH_PPTX to run the real corpus benchmark"]
    fn real_pptx_compact_html_benchmark() {
        let path = std::env::var("UNIPPT_BENCH_PPTX").expect("UNIPPT_BENCH_PPTX");
        let pptx = std::fs::read(path).unwrap();
        let mut deck = unippt_core::import_pptx(&pptx).unwrap();
        let cache_id = "html-corpus-cache";
        deck.source_import_id = Some(cache_id.into());
        let mut assets = AssetCatalog::default();
        externalize_deck_assets(&mut deck, cache_id, &mut assets).unwrap();
        let compact_json_size = serde_json::to_vec(&deck).unwrap().len();

        let started = std::time::Instant::now();
        let opc = opc_snapshot::explode(&pptx).unwrap();
        let encoded = encode_cached(&deck, &deck, cache_id, &assets, &opc).unwrap();
        let encode_elapsed = started.elapsed();
        let decode_started = std::time::Instant::now();
        let reopened = decode_compact(&encoded).unwrap();
        let decode_elapsed = decode_started.elapsed();
        eprintln!(
            "compact_json={} unique_assets={} html={} encode_ms={} decode_ms={}",
            compact_json_size,
            assets.len(),
            encoded.len(),
            encode_elapsed.as_millis(),
            decode_elapsed.as_millis(),
        );

        assert!(reopened.presentation.is_none());
        assert_eq!(reopened.opc.digest(), opc.digest());
        assert_eq!(reopened.assets.len(), assets.len());
        assert_eq!(reopened.deck.slides.len(), deck.slides.len());
        assert!(serde_json::to_vec(&reopened.deck).unwrap().len() < 10 * 1024 * 1024);
        assert!(encoded.len() < 128 * 1024 * 1024);
        assert!(encode_elapsed < std::time::Duration::from_secs(60));
        assert!(decode_elapsed < std::time::Duration::from_secs(60));
    }
}
