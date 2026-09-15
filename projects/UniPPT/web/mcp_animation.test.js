const test=require('node:test'),assert=require('node:assert/strict');
const {compile,schema}=require('./mcp_animation.js');
const deck={width:1280,height:720,slides:[{id:'s1',objects:[{id:'title',children:[]},{id:'diagram',children:[]}],animations:[{id:'old',targetObjectId:'title',durationMs:500}],transition:null}]};
test('animation schema exposes native effects and bounded timeline',()=>{const s=schema();assert.ok(s.effects.includes('motionPath'));assert.ok(s.transitions.includes('fade'));assert.equal(s.limits.steps,64);});
test('timeline compiles multiple targets and transition atomically',()=>{const result=compile(deck,{slideId:'s1',steps:[{objectIds:['title'],effect:'fade',trigger:'onClick',durationMs:450,staggerMs:40},{objectIds:['diagram'],effect:'wipe'}],transition:{kind:'push',durationMs:650,advanceOnClick:true}});assert.equal(result.operations.length,3);assert.equal(result.generated.length,2);});
test('timeline rejects unknown IDs, duplicate IDs and unsafe timings',()=>{
 assert.throws(()=>compile(deck,{slideId:'s1',steps:[{objectIds:['missing'],effect:'fade'}]}),/observed object IDs/);
 assert.throws(()=>compile(deck,{slideId:'s1',steps:[{objectIds:['title','title'],effect:'fade'}]}),/unique/);
 assert.throws(()=>compile(deck,{slideId:'s1',steps:[{objectIds:['title'],effect:'fade',durationMs:1}]}),/durationMs/);
 assert.throws(()=>compile(deck,{slideId:'s1',updates:[{animationId:'bad',patch:{durationMs:500}}]}),/observed/);
 assert.throws(()=>compile(deck,{slideId:'s1',steps:[{objectIds:['title'],effect:'fade',autoReverse:'false'}]}),/boolean/);
});
test('timeline supports sparse update/remove/order without changing the source',()=>{const before=JSON.stringify(deck);const result=compile(deck,{slideId:'s1',updates:[{animationId:'old',patch:{effect:'spin',trigger:'afterPrevious'}}]});assert.equal(result.operations.length,1);assert.equal(JSON.stringify(deck),before);const removed=compile(deck,{slideId:'s1',removeAnimationIds:['old']});assert.equal(removed.operations.length,1);});
test('timeline allows transition-only and preserves multi-step targets',()=>{const transition=compile(deck,{slideId:'s1',transition:{kind:'push',durationMs:650,direction:'left'}});assert.equal(transition.operations[0].transition.direction,'l');const result=compile(deck,{slideId:'s1',steps:[{objectIds:['title'],effect:'fade'},{objectIds:['title'],effect:'spin',class:'emphasis'}]});assert.equal(result.operations[1].animation.trigger,'afterPrevious');});
