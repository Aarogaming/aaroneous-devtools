const {test}=require('node:test'),assert=require('node:assert/strict');
const {createIndex}=require('./topic-path-core.cjs');
function fixture(){return {schema_version:'topic-projection-experiment-v1',topics:[
 {id:'root',title:'Root',parent:null,children:['a','b']},
 {id:'a',title:'Same label',parent:'root',children:['c'],tags:['shared']},
 {id:'b',title:'Same label',parent:'root',children:[],tags:['shared']},
 {id:'c',title:'Child',parent:'a',children:[]}
],relations:[{id:'e1',from:'c',to:'b',type:'contradicts',status:'advisory-unverified'}]};}
test('paths use identity despite duplicate titles',()=>assert.deepEqual(createIndex(fixture()).pathTo('c'),['root','a','c']));
test('enter and return preserve parent path',()=>{const p=['root','a'];const entered=createIndex(fixture()).enter(p,'c');assert.deepEqual(entered.slice(0,-1),p);assert.deepEqual(p,['root','a']);});
test('shared tags and relevance cannot establish containment',()=>assert.throws(()=>createIndex(fixture()).enter(['root','a'],'b'),/not a contained/));
test('forged prefix cannot enter a valid child',()=>assert.throws(()=>createIndex(fixture()).enter(['missing','a'],'c'),/invalid topic path/));
test('cross-path dissent preserves direction and identity',()=>assert.deepEqual(createIndex(fixture()).related('c')[0],{assertionId:'e1',type:'contradicts',direction:'outgoing',target:'b',resolved:true,status:'advisory-unverified',containment:false}));
test('dangling relationship remains visible',()=>{const f=fixture();f.relations[0].to='missing';assert.equal(createIndex(f).related('c')[0].resolved,false);});
test('cycles fail explicitly',()=>{const f=fixture();f.topics[0].parent='c';assert.throws(()=>createIndex(f),/cycle|containment/);});
test('missing parents fail explicitly',()=>{const f=fixture();f.topics[3].parent='missing';assert.throws(()=>createIndex(f),/child|parent/);});
test('duplicate identity fails explicitly',()=>{const f=fixture();f.topics.push({...f.topics[1]});assert.throws(()=>createIndex(f),/duplicate/);});
test('navigation cannot mutate captured projection',()=>{const f=fixture(),before=JSON.stringify(f),i=createIndex(f);i.pathTo('c');i.enter(['root'],'a');i.related('b');assert.equal(JSON.stringify(f),before);});

const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
test('captured topic fixture has valid paths and no metadata child records',()=>{
 const p=JSON.parse(fs.readFileSync(path.join(__dirname,'topic-projection-fixture.json'),'utf8'));
 const index=createIndex(p);
 assert.equal(p.synthetic,true);
 assert.equal(p.topics.length,13);
 for(const t of p.topics) {assert.equal(index.pathTo(t.id).at(-1),t.id);for(const e of index.related(t.id))assert.equal(e.resolved,true);}
 assert(p.topics.some(t=>t.metadata.length>0));
});
test('explorer embeds exactly the published fixture and path adapter',()=>{
 const html=fs.readFileSync(path.join(__dirname,'explorer.html'),'utf8');
 const encoded=html.slice(html.indexOf('const projection=')+17,html.indexOf(';\n// Isolated projection adapter'));
 assert.deepEqual(JSON.parse(encoded),JSON.parse(fs.readFileSync(path.join(__dirname,'topic-projection-fixture.json'),'utf8')));
 const adapter=fs.readFileSync(path.join(__dirname,'topic-path-core.cjs'),'utf8').replace('module.exports={createIndex};','');
 assert(html.includes(adapter));
 assert(html.includes('path=topicIndex.enter(path,id)'));
 assert(html.includes('if(wheel< -80)enter();else if(wheel>80)back();'));
});
test('explorer script parses without starting the browser or server',()=>{
 const html=fs.readFileSync(path.join(__dirname,'explorer.html'),'utf8');
 new vm.Script(html.match(/<script>([\s\S]*?)<\/script>/)[1]);
});
