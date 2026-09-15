import test from 'node:test';
import assert from 'node:assert/strict';
import {publishCachedSegment} from './segment_cache.mjs';

test('same-device cache publish does not copy',async()=>{
  const calls=[];
  await publishCachedSegment('/tmp/a','/cache/a',{rename:async(...a)=>calls.push(a)});
  assert.deepEqual(calls,[['/tmp/a','/cache/a']]);
});
test('cross-device copy stages beside destination before atomic publication',async()=>{
  const calls=[];
  const io={
    rename:async(a,b)=>{calls.push(['rename',a,b]);if(a==='/tmp/a')throw Object.assign(Error('cross device'),{code:'EXDEV'});},
    copyFile:async(a,b)=>calls.push(['copy',a,b]),
    unlink:async(a)=>calls.push(['unlink',a]),
  };
  await publishCachedSegment('/tmp/a','/cache/a',io);
  const pending=calls[1][2];assert.match(pending,/^\/cache\/a\..+\.pending$/);
  assert.deepEqual(calls.slice(1),[['copy','/tmp/a',pending],['rename',pending,'/cache/a'],['unlink','/tmp/a'],['unlink',pending]]);
});
test('failed copy does not publish or erase source',async()=>{
  const calls=[];
  await assert.rejects(publishCachedSegment('/tmp/a','/cache/a',{
    rename:async()=>{throw Object.assign(Error('cross device'),{code:'EXDEV'});},
    copyFile:async()=>{throw Error('disk full');},
    unlink:async(a)=>calls.push(a),
  }),/disk full/);
  assert.equal(calls.length,1);assert.ok(calls[0].startsWith('/cache/a.'));assert.ok(calls[0].endsWith('.pending'));
});
