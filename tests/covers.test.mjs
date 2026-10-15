import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/cover-model.ts',import.meta.url),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
const model=await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const {adjustSource,applyAdjusted,coverDetails,formatSize,imageFormat,pictureTypeLabel,selectionCovers,keepPlan,distinctPlan,managedFromSet,newManaged,replaceManaged,moveManaged,dropManaged,planItems,planChanged,presence,coverMenu,coverFileName,coverExtension,isImagePath,insideRect,PICTURE_TYPES}=model;

const cover=(digest,type='CoverFront',description='',extra={})=>({index:0,mime:'image/jpeg',description,picture_type:type,data_base64:'',size:250880,width:1200,height:1200,digest,...extra});

test('cover details read like the cover window: dimensions, size and format',()=>{
 assert.equal(coverDetails(cover('a')),'1200×1200, 245 KB, JPEG');
 assert.equal(coverDetails({mime:'image/png',size:2_400_000,width:null,height:null}),'2.3 MB, PNG');
 assert.equal(coverDetails({mime:'image/webp',size:900}),'900 B, WEBP');
 assert.equal(formatSize(1023),'1023 B');assert.equal(imageFormat('image/x-icon'),'X-ICON');
 assert.equal(pictureTypeLabel('CoverBack'),'Back Cover');assert.equal(pictureTypeLabel('Custom'),'Custom');
 assert.equal(PICTURE_TYPES.length,21);
});

test('selection shows covers only when every file has the same list',()=>{
 assert.equal(selectionCovers([]).state,'none');
 assert.equal(selectionCovers([[],[]]).state,'none');
 const same=selectionCovers([[cover('a'),cover('b','CoverBack')],[cover('a'),cover('b','CoverBack')]]);
 assert.deepEqual([same.state,same.files,same.withCovers,same.distinct.length],['same',2,2,2]);
 const reordered=selectionCovers([[cover('a'),cover('b','CoverBack')],[cover('b','CoverBack'),cover('a')]]);
 assert.equal(reordered.state,'varying');
 const partial=selectionCovers([[cover('a')],[]]);
 assert.deepEqual([partial.state,partial.withCovers],['varying',1]);
 // Same image with another description is a different cover.
 assert.equal(selectionCovers([[cover('a')],[cover('a','CoverFront','x')]]).state,'varying');
 // Browser previews without digests compare MIME and size.
 assert.equal(selectionCovers([[cover(undefined)],[cover(undefined,'CoverFront','',{size:1})]]).state,'varying');
});

test('context plans keep other covers and edit the displayed one or every distinct cover',()=>{
 const list=[cover('a'),cover('b','CoverBack','rear')];
 assert.deepEqual(keepPlan(list,(item,i)=>i===0?null:item),[{original:{digest:'b',picture_type:'CoverBack',description:'rear'},image:null,picture_type:'CoverBack',description:'rear',all_files:false,occurrence:0}]);
 // Identical copies are addressed one by one.
 assert.deepEqual(keepPlan([cover('a'),cover('a')],(item,i)=>i===0?null:item).map(item=>item.occurrence),[1]);
 const retyped=keepPlan(list,(item,i)=>i===1?{...item,picture_type:'Media'}:item);
 assert.deepEqual(retyped.map(item=>[item.original.picture_type,item.picture_type]),[['CoverFront','CoverFront'],['CoverBack','Media']]);
 const distinct=selectionCovers([[cover('a')],[cover('c','Artist')]]).distinct;
 assert.deepEqual(distinctPlan(distinct,item=>({...item,description:'x'})).map(item=>[item.original.digest,item.description]),[['a','x'],['c','x']]);
 assert.deepEqual(distinctPlan(distinct,()=>null),[]);
});

const set=()=>({files:3,errors:[],covers:[
 {digest:'a',picture_type:'CoverFront',description:'',mime:'image/jpeg',size:10,width:2,height:2,files:[0,1,2],index:0,data_base64:'AA=='},
 {digest:'b',picture_type:'CoverBack',description:'',mime:'image/png',size:20,width:3,height:3,files:[1],index:1,data_base64:'AA=='},
 {digest:'c',picture_type:'Artist',description:'Band',mime:'image/png',size:30,width:4,height:4,files:[0,2],index:1,data_base64:'AA=='},
]});
const staged={token:'t1',mime:'image/png',size:5,width:1,height:1,data_base64:'AA==',label:'new.png'};

test('manage list reports presence and turns into an ordered plan',()=>{
 const list=managedFromSet(set());
 assert.deepEqual(list.map(item=>presence(item).text),['In all 3 files','In 1 of 3 files','In 2 of 3 files']);
 assert.equal(planChanged(list,list),false);
 const all=list.map(item=>item.id==='c1'?{...item,all_files:true}:item);
 assert.equal(presence(all[1]).full,true);assert.equal(planChanged(list,all),true);
 assert.deepEqual(planItems(all).map(item=>[item.original.digest,item.all_files]),[['a',false],['b',true],['c',false]]);
});

test('new and replaced images carry staged data; repeated types get a description',()=>{
 const list=managedFromSet(set());
 const added=newManaged(list,staged,'n0',3);
 assert.deepEqual([added.picture_type,added.description,added.all_files,added.image],['CoverFront','new.png',true,{staged:'t1'}]);
 const lone=newManaged([],staged,'n1',1);assert.equal(lone.description,'');
 const twice=newManaged([...list,added],staged,'n2',3);assert.equal(twice.description,'new.png (2)');
 const replaced=replaceManaged(list[1],staged);
 assert.deepEqual([replaced.original.digest,replaced.picture_type,replaced.image.staged,replaced.width],['b','CoverBack','t1',1]);
 assert.deepEqual(planItems([added])[0],{original:null,image:{staged:'t1'},picture_type:'CoverFront',description:'new.png',all_files:true});
});

test('moving covers keeps relative order and stops at the ends',()=>{
 const list=managedFromSet(set()),ids=list.map(item=>item.id);
 assert.deepEqual(moveManaged(list,new Set(['c2']),-1).map(item=>item.id),['c0','c2','c1']);
 assert.deepEqual(moveManaged(list,new Set(['c1','c2']),-1).map(item=>item.id),['c1','c2','c0']);
 assert.deepEqual(moveManaged(list,new Set(['c0']),-1).map(item=>item.id),ids);
 assert.deepEqual(moveManaged(list,new Set(['c0','c1']),1).map(item=>item.id),['c2','c0','c1']);
 assert.deepEqual(moveManaged(list,new Set(['c2']),1).map(item=>item.id),ids);
 assert.deepEqual(dropManaged(list,new Set(['c2']),'c0').map(item=>item.id),['c2','c0','c1']);
 assert.deepEqual(dropManaged(list,new Set(['c0']),null).map(item=>item.id),['c1','c2','c0']);
 assert.deepEqual(dropManaged(list,new Set(['c0']),'c0').map(item=>item.id),ids);
});

test('cover menu mirrors the documented commands and their availability',()=>{
 const base={selected:1,state:'same',count:2,index:0,displayedType:'CoverFront',canEdit:true,canClipboard:true,aspect:true,mac:false};
 const flat=items=>items.filter(item=>item!=='separator');
 const labels=flat(coverMenu(base)).map(item=>item.label);
 for(const label of ['Add cover…','Remove cover','Replace cover…','Extract cover…','Adjust cover…','Manage covers…','Set cover type','Set cover description…','Cut cover','Copy cover','Paste cover','Correct aspect ratio'])assert.ok(labels.includes(label),label);
 const get=(state,label)=>flat(coverMenu({...base,...state})).find(item=>item.label===label);
 assert.equal(get({},'Previous cover').enabled,false);assert.equal(get({},'Next cover').enabled,true);
 assert.equal(get({},'Set cover type').children.find(item=>item.checked).value,'CoverFront');
 assert.equal(get({},'Manage covers…').shortcut,'Ctrl+Alt+M');assert.equal(get({mac:true},'Copy cover').shortcut,'⌘C');
 // Varying covers: only commands that act on every cover remain.
 for(const [label,enabled] of [['Remove cover',true],['Replace cover…',false],['Extract cover…',false],['Copy cover',false],['Set cover type',true],['Paste cover',true]])assert.equal(get({state:'varying'},label).enabled,enabled,label);
 assert.equal(get({state:'none',count:0},'Remove cover').enabled,false);
 assert.equal(get({canEdit:false},'Add cover…').enabled,false);assert.equal(get({canEdit:false},'Copy cover').enabled,true);
 assert.equal(get({canClipboard:false},'Paste cover').enabled,false);
 assert.equal(get({aspect:false},'Correct aspect ratio').checked,false);
});

test('extract names, image paths and drop coordinates',()=>{
 assert.equal(coverFileName('AC/DC: Live?'),'AC_DC_ Live_');assert.equal(coverFileName('  .. '),'folder');assert.equal(coverFileName('folder'),'folder');
 assert.equal(coverExtension('image/jpeg'),'jpg');assert.equal(coverExtension('image/PNG'),'png');assert.equal(coverExtension('image/webp'),'webp');
 assert.ok(isImagePath('/a/Cover.JPEG'));assert.ok(isImagePath('C:\\x\\a.webp'));assert.ok(!isImagePath('/a/song.mp3'));assert.ok(!isImagePath('/a/png'));
 const rect={left:10,top:20,right:110,bottom:120};
 assert.ok(insideRect({x:40,y:80},rect,2));assert.ok(!insideRect({x:5,y:80},rect,1));assert.ok(!insideRect({x:300,y:80},rect,2));
});

test('Manage Covers adjusts staged images or the first file holding a cover, and only replaces changed ones',()=>{
 const set={files:3,errors:[],covers:[{...cover('a'),files:[1,2],index:1,data_base64:'AAA'},{...cover('b','CoverBack'),files:[0],index:0,data_base64:'BBB'}]};
 const paths=['/one.mp3','/two.mp3','/three.mp3'];
 const list=managedFromSet(set);
 assert.deepEqual(adjustSource(list[0],set,paths),{cover:{path:'/two.mp3',index:1}});
 assert.deepEqual(adjustSource(list[1],set,paths),{cover:{path:'/one.mp3',index:0}});
 const staged={token:'t1',mime:'image/png',size:3,width:10,height:10,data_base64:'NEW',label:'Adjusted cover'};
 const added=newManaged(list,staged,'n0',3);
 assert.deepEqual(adjustSource(added,set,paths),{staged:'t1'});
 assert.equal(adjustSource({...added,image:{path:'/cover.jpg'}},set,paths),null);
 assert.equal(adjustSource({...list[0],original:{digest:'zzz',picture_type:'CoverFront',description:''}},set,paths),null);
 const result=applyAdjusted(list,new Map([[list[0].id,{...staged,data_base64:'AAA'}],[list[1].id,staged]]));
 assert.equal(result.changed,1);
 assert.equal(result.list[0],list[0]);
 assert.deepEqual([result.list[1].image,result.list[1].picture_type,result.list[1].width],[{staged:'t1'},'CoverBack',10]);
 assert.ok(planChanged(list,result.list));
});
