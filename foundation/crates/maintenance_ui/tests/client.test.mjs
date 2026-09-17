import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createApiClient, validateSnapshot} from '../src/client.mjs';
test('status is validated before enabling controls', () => {
  const s = {plan:{mode:'plan',build_ready:false,packet:{summary:'blocked'},criteria:[]},status:{checkout:'explicit',local_model:'unavailable',results:[]},research:{summary:'none'},review_results:[]};
  assert.equal(validateSnapshot(s),s);
  for (const bad of [null,{}, {...s,plan:{...s.plan,mode:'verified'}},{...s,review_results:[{}]}]) assert.throws(() => validateSnapshot(bad));
});
test('requests are read-only by default and response size is bounded', async () => {
  const calls=[];
  const client=createApiClient('session',{fetchImpl:async(path,options)=>{calls.push({path,options});return new Response('{"ok":true}');}});
  assert.deepEqual(await client.request('/api/status'),{ok:true});
  assert.equal(calls[0].options.method,undefined);
  assert.equal(calls[0].options.cache,'no-store');
  assert.equal(calls[0].options.headers['X-Devtools-Token'],'session');
  await client.request('/api/packet',{packet:'readiness'});
  assert.equal(calls[1].options.method,'POST');
  const oversized=createApiClient('session',{maxBytes:2,fetchImpl:async()=>new Response('123')});
  await assert.rejects(oversized.request('/api/status'),/stale/);
});
test('invalid data and HTTP errors never become success',async()=>{
  const bad=createApiClient('session',{fetchImpl:async()=>new Response('not json')});
  await assert.rejects(bad.request('/api/status'),/invalid response/);
  const denied=createApiClient('session',{fetchImpl:async()=>new Response('{}',{status:403})});
  await assert.rejects(denied.request('/api/review',{}),/rejected \(403\)/);
  const failed=createApiClient('session',{fetchImpl:async()=>new Response('{}',{status:500})});
  await assert.rejects(failed.request('/api/review',{}),e=>e.unknownOutcome===true);
});
test('timeouts distinguish unknown mutation outcomes and never retry',async()=>{
  let calls=0;
  const fetchImpl=(_path,{signal})=>{calls++;return new Promise((_resolve,reject)=>signal.addEventListener('abort',()=>reject(new Error('aborted')),{once:true}));};
  const client=createApiClient('session',{timeoutMs:10,fetchImpl});
  await assert.rejects(client.request('/api/review',{}),e=>e.unknownOutcome===true&&/before repeating/.test(e.message));
  assert.equal(calls,1);
  await assert.rejects(client.request('/api/status'),/actions are disabled/);
  assert.equal(calls,2);
});
test('page close cancels active requests',async()=>{
  const client=createApiClient('session',{fetchImpl:(_path,{signal})=>new Promise((_resolve,reject)=>signal.addEventListener('abort',()=>reject(new Error('closed')),{once:true}))});
  const request=client.request('/api/status');client.stop();
  await assert.rejects(request,/unavailable/);
});
