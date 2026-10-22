// Deterministic guide workflow, using the actual source transformation module.
// This does not execute a dialog, native save, image download or provider request.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {loadProductionModule} from './helpers/load-production-module.mjs';

const fixture=JSON.parse(await readFile(new URL('./fixtures/guide-musicbrainz-lesson.json',import.meta.url),'utf8'));
const lesson=await readFile(new URL('../docs/user-guide/learn/tag-from-online-sources.md',import.meta.url),'utf8');
const {autoMatch,buildApplyItems,editField,moveSlotTo,searchPrefill}=await loadProductionModule('tag-sources.ts');

// The Rust MusicBrainz unit test checks this whole expected model against the
// real provider parser. Keeping both stages on one independently authored
// fixture avoids either a duplicate JS parser or a stale handwritten release.
const release=fixture.expected_release;
const files=fixture.selected_files;
const fieldStep=lesson.split(/^## 4\./m)[1].split(/^## 5\./m)[0];
const selectedFields=new Set([...fieldStep.matchAll(/`([A-Z][A-Z0-9_]*)`/g)].map(match=>match[1]));
const selectionOrder=(tracks,input)=>autoMatch(tracks,input.map(file=>({...file,track:'',title:'',name:''})));

test('online lesson imports its actual Markdown field selection into the checked file order',()=>{
  assert.deepEqual(searchPrefill(files),{artist:'Juniper Vale',album:'Harbour Lights'});
  const slots=selectionOrder(release.tracks,files);
  assert.deepEqual(slots,files.map(file=>file.path));
  const items=buildApplyItems(release,slots,selectedFields);
  assert.deepEqual(items,[
    {path:files[0].path,updates:{ALBUM:['Harbour Lights'],ALBUMARTIST:['Juniper Vale'],TITLE:['Paper Boats'],ARTIST:['Juniper Vale']}},
    {path:files[1].path,updates:{ALBUM:['Harbour Lights'],ALBUMARTIST:['Juniper Vale'],TITLE:['Lanterns on the Water'],ARTIST:['Juniper Vale']}},
  ]);
  // Contradictory old numbers/names are deliberately ignored by Selection order.
  assert.deepEqual(autoMatch(release.tracks,files),[files[1].path,files[0].path]);
});

test('online lesson correction moves file assignments and skips an extra selected file',()=>{
  const extra={...files[0],path:'/practice/Unrelated bonus.flac',name:'Unrelated bonus.flac'};
  const slots=selectionOrder(release.tracks,[...files].reverse().concat(extra));
  const corrected=moveSlotTo(slots,1,0);
  assert.deepEqual(corrected,[files[0].path,files[1].path,extra.path]);
  const items=buildApplyItems(release,corrected,selectedFields);
  assert.deepEqual(items.map(item=>[item.path,item.updates.TITLE[0]]),[
    [files[0].path,'Paper Boats'],[files[1].path,'Lanterns on the Water'],
  ]);
});

test('online lesson empty values do not construct removals and artwork has an independent write choice',()=>{
  const edited=structuredClone(release);
  const slots=selectionOrder(edited.tracks,files);
  editField(edited,'TITLE','',0);
  const items=buildApplyItems(edited,slots,selectedFields);
  assert.equal(items.length,2,'other checked values still need writing');
  assert.equal('TITLE' in items[0].updates,false,'an empty proposal omits the field instead of clearing the existing title');
  assert.deepEqual(items[1].updates.TITLE,['Lanterns on the Water']);
  assert.deepEqual(buildApplyItems(edited,slots,new Set(['TITLE'])),[
    {path:files[1].path,updates:{TITLE:['Lanterns on the Water']}},
  ]);
  assert.deepEqual(buildApplyItems(edited,slots,new Set()),[]);
  assert.deepEqual(buildApplyItems(edited,slots,new Set(),true),files.map(file=>({path:file.path,updates:{}})));
});
