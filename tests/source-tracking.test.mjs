import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/source-tracking.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const {SourceRootTracker}=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const deferred=()=>{let resolve;const promise=new Promise(done=>resolve=done);return{promise,resolve};};
function backend(){
  const state={objects:new Map(),calls:[],serial:0,failBegin:false,failRelease:0,gate:undefined,entered:deferred()};
  const capture=path=>{const token='object-'+(++state.serial);state.objects.set(token,{path});return{path,token,error:null};};
  const invoke=async(command,args)=>{
    state.calls.push([command,structuredClone(args)]);
    if(command==='begin_source_tracking'){if(state.failBegin){state.failBegin=false;throw new Error('Tracking temporarily unavailable');}return 'owned-session';}
    assert.equal(args.session,'owned-session');
    if(command==='prepare_source_roots'){const sources=args.paths.map(capture);return{key:'capture-'+(++state.serial),sources,acquired:sources.map(source=>source.token)};}
    if(command==='release_source_roots'){
      if(state.gate){state.entered.resolve();await state.gate.promise;}
      if(state.failRelease){state.failRelease--;throw new Error('Cleanup temporarily unavailable');}
      args.tokens.forEach(token=>state.objects.delete(token));return;
    }
    if(command==='resolve_source_roots'){
      const sources=args.sources.map(source=>{const object=state.objects.get(source.token);assert.ok(object,'resolution must retain its captured object');return{...source,path:object.path};});
      return{key:'resolution-'+(++state.serial),sources,acquired:[],paths:sources.map(source=>source.path),renamed:sources.flatMap((source,index)=>source.path===args.sources[index].path?[]:[{from:args.sources[index].path,to:source.path}])};
    }
    assert.fail(command);
  };
  const tracker=new SourceRootTracker({invoke});
  const load=async paths=>{const prepared=await tracker.prepare(paths);tracker.sync(paths,prepared);await tracker.ready();return state.calls.filter(([name])=>name==='prepare_source_roots').at(-1)[1];};
  return{state,tracker,load};
}
test('a cancelled capture releases only its new objects and leaves accepted sources owned',async()=>{
  const {state,tracker,load}=backend();await load(['/existing.wav']);const accepted=[...state.objects.keys()][0];
  const prepared=await tracker.prepare(['/new.wav']);assert.equal(state.objects.size,2);
  tracker.discard(prepared);await tracker.ready();assert.deepEqual([...state.objects.keys()],[accepted]);
  const resolved=await tracker.resolve(['/existing.wav']);tracker.sync(resolved.paths,resolved.prepared);await tracker.ready();
  assert.equal(state.calls.filter(([name])=>name==='resolve_source_roots').at(-1)[1].sources[0].token,accepted);
});
test('source add and external rename preserve existing tokens after cleanup failure',async()=>{
  const {state,tracker,load}=backend();await load(['/remove.wav','/keep.wav']);const [,kept]=[...state.objects.keys()];
  state.failRelease=2;tracker.sync(['/keep.wav']);await assert.rejects(tracker.ready(),/Cleanup temporarily unavailable/);
  state.objects.get(kept).path='/moved/keep.wav';const prepared=await tracker.prepare(['/added.wav']);tracker.sync(['/keep.wav','/added.wav'],prepared);await tracker.ready();
  const result=await tracker.resolve(['/keep.wav','/added.wav']);tracker.sync(result.paths,result.prepared);await tracker.ready();
  assert.deepEqual(result.paths,['/moved/keep.wav','/added.wav']);assert.equal(state.objects.size,2);assert.ok(state.objects.has(kept));
  assert.equal(state.calls.filter(([name])=>name==='prepare_source_roots').length,2,'adding a source never recaptures an accepted object by name');
});
test('known renames keep tokens through cleanup failure and another outside move',async()=>{
  const {state,tracker,load}=backend();await load(['/old.wav','/remove.wav']);const [kept]=[...state.objects.keys()];
  state.objects.get(kept).path='/known.wav';tracker.rebase(['/old.wav','/remove.wav'],['/known.wav','/remove.wav']);
  state.failRelease=2;tracker.sync(['/known.wav']);await assert.rejects(tracker.ready(),/Cleanup/);
  state.objects.get(kept).path='/outside.wav';const result=await tracker.resolve(['/known.wav']);tracker.sync(result.paths,result.prepared);await tracker.ready();
  assert.deepEqual(result.paths,['/outside.wav']);assert.deepEqual([...state.objects.keys()],[kept]);assert.equal(state.calls.filter(([name])=>name==='prepare_source_roots').length,1);
});
test('Clear queued during earlier cleanup releases obsolete tokens without changing their ownership',async()=>{
  const {state,tracker,load}=backend();await load(['/one.wav','/two.wav']);state.gate=deferred();
  tracker.sync(['/two.wav']);await state.entered.promise;tracker.sync([]);const cleared=tracker.ready();state.gate.resolve();await cleared;
  assert.equal(state.objects.size,0);assert.deepEqual((await tracker.resolve([])).paths,[]);
});
test('explicitly loading a path again replaces its token and stale discard never closes the accepted capture',async()=>{
  const {state,tracker,load}=backend();await load(['/same.wav']);const old=[...state.objects.keys()][0];const prepared=await tracker.prepare(['/same.wav']);
  tracker.sync(['/same.wav'],prepared);tracker.discard(prepared);await tracker.ready();assert.equal(state.objects.size,1);assert.ok(!state.objects.has(old));
  const current=[...state.objects.keys()][0];state.objects.get(current).path='/renamed.wav';assert.deepEqual((await tracker.resolve(['/same.wav'])).paths,['/renamed.wav']);
});
test('failed session creation does not accept a root and preparation can be retried',async()=>{
  const {state,tracker}=backend();state.failBegin=true;await assert.rejects(tracker.prepare(['/one.wav']),/temporarily unavailable/);assert.equal(state.objects.size,0);
  const prepared=await tracker.prepare(['/one.wav']);tracker.sync(['/one.wav'],prepared);await tracker.ready();assert.deepEqual((await tracker.resolve(['/one.wav'])).paths,['/one.wav']);
});
