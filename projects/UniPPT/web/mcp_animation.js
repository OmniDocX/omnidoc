(function (global) {
  'use strict';

  const EFFECTS = ['appear','fade','flyIn','wipe','randomBars','dissolve','wheel','circle','split','zoom','spin','growShrink','motionPath','media'];
  const CLASSES = ['entrance','emphasis','exit','motionPath','media'];
  const TRIGGERS = ['onClick','withPrevious','afterPrevious'];
  const TRANSITIONS = ['blinds','checker','circle','comb','cover','cut','diamond','dissolve','fade','newsflash','plus','pull','push','random','randomBar','split','strips','wedge','wheel','wipe','zoom'];
  const DIRECTIONS = ['left','right','up','down','horizontal','vertical','in','out','inHorizontal','outHorizontal','1','2','3','4'];
  const TRANSITION_DIRECTIONS = ['left','right','up','down'];
  const TRANSITION_DIR_XML = {left:'l', right:'r', up:'u', down:'d'};
  const UPDATE_KEYS = new Set(['effect','class','trigger','durationMs','delayMs','direction','motionPath','repeatCount','repeatDurationMs','autoReverse','presetId','presetSubtype','acceleration','deceleration','speed','timeFilter','mediaAction']);
  const finite = (value, label, fallback) => {
    if (value == null && fallback !== undefined) return fallback;
    if (typeof value !== 'number' || !Number.isFinite(value)) throw Error(`Invalid ${label}`);
    return value;
  };
  const own = (obj, key) => Object.prototype.hasOwnProperty.call(obj, key);
  function schema() {
    return {
      version:'1.0.0', effects:EFFECTS, classes:CLASSES, triggers:TRIGGERS,
      transitions:TRANSITIONS, directions:DIRECTIONS, transitionDirections:TRANSITION_DIRECTIONS,
      limits:{steps:64,targetsPerStep:20,updates:64,removals:64,operations:100,durationMs:[10,120000],delayMs:[0,120000],iterations:[1,100]},
      step:{objectIds:'observed object IDs',effect:'preset effect',class:'entrance|emphasis|exit|motionPath|media',trigger:'onClick|withPrevious|afterPrevious',durationMs:'10–120000',delayMs:'0–120000',staggerMs:'0–120000',direction:'object preset direction',motionPath:'absolute M/L/C/Q/Z path for motionPath',iterations:'1–100',autoReverse:'boolean',mediaAction:'play|pause|stop'},
      transition:{kind:TRANSITIONS,durationMs:'10–120000',advanceOnClick:'boolean',advanceAfterMs:'optional non-negative milliseconds',direction:TRANSITION_DIRECTIONS},
      example:{slideId:'slide-1',steps:[{objectIds:['title'],effect:'fade',trigger:'onClick',durationMs:500},{objectIds:['diagram'],effect:'wipe',trigger:'afterPrevious',direction:'left'}],transition:{kind:'fade',durationMs:650,advanceOnClick:true}}
    };
  }
  function allowed(value, values, label) { if (value != null && !values.includes(value)) throw Error(`${label} is not supported`); return value; }
  function integer(value, label, min, max, fallback) { const n = finite(value,label,fallback); if (!Number.isInteger(n) || n < min || n > max) throw Error(`${label} must be ${min}–${max}`); return n; }
  function path(value) {
    if (value == null) return null;
    if (typeof value !== 'string' || value.length < 3 || value.length > 10000 || !/^M(?:\s|,|[-+0-9.])/.test(value) || /[^\s,MLCQZ0-9.+-]/.test(value)) throw Error('motionPath accepts a bounded absolute M/L/C/Q/Z path');
    const tokenPattern = /[MLCQZ]|[-+]?(?:\d+(?:\.\d*)?|\.\d+)/g;
    if (value.replace(tokenPattern,'').replace(/[\s,]/g,'')) throw Error('Invalid motionPath syntax');
    const tokens=value.match(tokenPattern),arity={M:2,L:2,C:6,Q:4,Z:0};
    if (!tokens || tokens[0] !== 'M') throw Error('motionPath must begin with absolute M');
    for(let i=0;i<tokens.length;){const command=tokens[i++];if(!own(arity,command))throw Error('Invalid motionPath command');let count=0;while(i<tokens.length&&!own(arity,tokens[i])){if(Math.abs(Number(tokens[i++]))>1000000)throw Error('motionPath coordinate exceeds limits');count++;}if(arity[command]===0?count!==0:count===0||count%arity[command]!==0)throw Error('Invalid motionPath coordinate count');}
    return value;
  }
  function objectMap(slide) {
    const map = new Map();
    (function walk(objects) { for (const object of objects || []) { if (object && typeof object.id === 'string') map.set(object.id, object); walk(object && object.children); } })(slide.objects);
    return map;
  }
  function compile(deck, args) {
    if (!deck || !Array.isArray(deck.slides) || !args || typeof args !== 'object') throw Error('Invalid document or timeline');
    const slide = deck.slides.find(s => s.id === args.slideId); if (!slide) throw Error('Unknown slide ID');
    const steps = args.steps == null ? [] : args.steps, updates = args.updates == null ? [] : args.updates, removals = args.removeAnimationIds == null ? [] : args.removeAnimationIds;
    if (!Array.isArray(steps) || !Array.isArray(updates) || !Array.isArray(removals)) throw Error('steps, updates and removeAnimationIds must be arrays');
    if (steps.length > 64 || updates.length > 64 || removals.length > 64 || steps.length + updates.length + removals.length < 1 && !own(args,'transition') && args.animationOrder == null) throw Error('Timeline must contain at least one operation');
    const map = objectMap(slide), existing = new Map((slide.animations || []).map(a => [a.id,a])), operations = [], generated = [], removed = new Set();
    const push = op => { operations.push(op); if (operations.length > 100) throw Error('Timeline exceeds the 100-operation limit'); };
    for (const [index, step] of steps.entries()) {
      if (!step || typeof step !== 'object' || !Array.isArray(step.objectIds) || step.objectIds.length < 1 || step.objectIds.length > 20) throw Error(`Step ${index + 1} needs 1–20 observed objectIds`);
      const ids = new Set(); for (const id of step.objectIds) { if (typeof id !== 'string' || ids.has(id) || !map.has(id)) throw Error('Each step needs unique observed object IDs on the slide'); ids.add(id); }
      const effect = allowed(step.effect == null ? 'fade' : step.effect,EFFECTS,'Animation effect');
      const klass = allowed(step.class == null ? (effect === 'spin' || effect === 'growShrink' ? 'emphasis' : effect === 'motionPath' ? 'motionPath' : effect === 'media' ? 'media' : 'entrance') : step.class,CLASSES,'Animation class');
      if (effect === 'motionPath' && klass !== 'motionPath') throw Error('motionPath effect requires motionPath class');
      if (effect === 'media' && klass !== 'media') throw Error('media effect requires media class');
      const trigger = allowed(step.trigger == null ? (index ? 'afterPrevious' : 'onClick') : step.trigger,TRIGGERS,'Animation trigger');
      const durationMs = integer(step.durationMs,'durationMs',10,120000,500), delayMs = integer(step.delayMs,'delayMs',0,120000,0), staggerMs = integer(step.staggerMs,'staggerMs',0,120000,0);
      const direction = allowed(step.direction,DIRECTIONS,'Animation direction'), motionPath = path(step.motionPath);
      if (effect === 'motionPath' && !motionPath) throw Error('motionPath effect requires motionPath');
      if (step.autoReverse != null && typeof step.autoReverse !== 'boolean') throw Error('autoReverse must be boolean');
      const iterations = step.iterations == null ? null : integer(step.iterations,'iterations',1,100);
      const mediaAction = effect === 'media' ? (step.mediaAction == null ? 'play' : step.mediaAction) : null;
      if (mediaAction != null && !['play','pause','stop'].includes(mediaAction)) throw Error('mediaAction must be play, pause or stop');
      step.objectIds.forEach((targetObjectId,targetIndex) => {
        const animation = {effect,class:klass,trigger:targetIndex ? 'withPrevious' : trigger,durationMs,delayMs:targetIndex ? staggerMs : delayMs,direction:direction || null,motionPath,autoReverse:step.autoReverse === true,mediaAction};
        if (iterations != null) animation.repeatCount = String(iterations * 1000);
        push({op:'addAnimation',slideId:slide.id,targetObjectId,animation}); generated.push({step:index,targetObjectId});
      });
    }
    for (const update of updates) {
      if (!update || typeof update !== 'object' || typeof update.animationId !== 'string' || !existing.has(update.animationId) || removed.has(update.animationId)) throw Error('Update requires an observed animationId');
      if (!update.patch || typeof update.patch !== 'object' || Array.isArray(update.patch)) throw Error('Animation update requires a patch object');
      const patch = {...update.patch}; for (const key of Object.keys(patch)) { if (!UPDATE_KEYS.has(key)) throw Error(`Animation field ${key} cannot be updated`); }
      if (patch.effect != null) allowed(patch.effect,EFFECTS,'Animation effect'); if (patch.class != null) allowed(patch.class,CLASSES,'Animation class'); if (patch.trigger != null) allowed(patch.trigger,TRIGGERS,'Animation trigger'); if (patch.direction != null) allowed(patch.direction,DIRECTIONS,'Animation direction'); if (patch.motionPath != null) path(patch.motionPath);
      if (patch.durationMs != null) integer(patch.durationMs,'durationMs',10,120000); if (patch.delayMs != null) integer(patch.delayMs,'delayMs',0,120000); if (patch.autoReverse != null && typeof patch.autoReverse !== 'boolean') throw Error('autoReverse must be boolean');
      push({op:'updateAnimation',slideId:slide.id,animationId:update.animationId,patch});
    }
    for (const id of removals) { if (typeof id !== 'string' || removed.has(id) || !existing.has(id)) throw Error('removeAnimationIds must contain observed animation IDs exactly once'); removed.add(id); push({op:'removeAnimation',slideId:slide.id,animationId:id}); }
    if (args.animationOrder != null) {
      if (steps.length) throw Error('animationOrder cannot be combined with new steps; use returned IDs in the next call');
      if (!Array.isArray(args.animationOrder)) throw Error('animationOrder must be an array');
      const remaining = [...existing.keys()].filter(id => !removed.has(id));
      if (args.animationOrder.length !== remaining.length || new Set(args.animationOrder).size !== args.animationOrder.length || args.animationOrder.some(id => !existing.has(id) || removed.has(id))) throw Error('animationOrder must list every remaining observed animation ID exactly once');
      args.animationOrder.forEach((id,order) => push({op:'updateAnimation',slideId:slide.id,animationId:id,patch:{order}}));
    }
    if (own(args,'transition')) {
      const t = args.transition;
      if (t !== null) {
        if (!t || typeof t !== 'object' || Array.isArray(t)) throw Error('transition must be an object or null');
        const kind = allowed(t.kind == null ? 'fade' : t.kind,TRANSITIONS,'Transition kind');
        const durationMs = integer(t.durationMs,'transition durationMs',10,120000,700);
        if (t.advanceOnClick != null && typeof t.advanceOnClick !== 'boolean') throw Error('advanceOnClick must be boolean');
        if (t.advanceAfterMs != null) integer(t.advanceAfterMs,'advanceAfterMs',0,86400000);
        const direction = allowed(t.direction,TRANSITION_DIRECTIONS,'Transition direction');
        push({op:'setTransition',slideId:slide.id,transition:{kind,durationMs,advanceOnClick:t.advanceOnClick !== false,advanceAfterMs:t.advanceAfterMs == null ? null : t.advanceAfterMs,direction:direction ? TRANSITION_DIR_XML[direction] : null}});
      } else push({op:'setTransition',slideId:slide.id,transition:null});
    }
    return {operations,generated,slideId:slide.id,requested:{steps:steps.length,updates:updates.length,removals:removals.length}};
  }
  const api = {schema,compile}; global.UniPptMcpAnimation = api; if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
