import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const root = new URL('../', import.meta.url);
const read = path => readFile(new URL(path, root), 'utf8');
const load = async name => {
  const source = await read(`src/${name}.ts`);
  const javascript = ts.transpileModule(source, {compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ES2022}}).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const {windowShortcut, altChord} = await load('keyboard');
const {fileShortcut, FileOpsUI} = await load('file-ops');
const {listShortcut} = await load('list-commands');
const {TagPanel} = await load('tag-panel');
const paths = ['macos','windows','linux'];
const pages = Object.fromEntries(await Promise.all(paths.map(async name=>[name,(await read(`docs/user-guide/platforms/${name}.md`)).replace(/\r\n/g,'\n')])));
const manifest = JSON.parse(await read('docs/user-guide/_meta/coverage/desktop-v0.2.13-release-manifest.json'));
const inspected = JSON.parse(await read('docs/user-guide/_meta/coverage/desktop-v0.2.13-inspection.json'));
const section = (page,name)=>page.split(`<!-- ${name}:start -->`)[1].split(`<!-- ${name}:end -->`)[0];
const rows = (page,name)=>section(page,name).split(/\r?\n/).map(s=>s.trimStart()).filter(s=>s.startsWith('|')).slice(2).map(s=>s.replace(/^\||\|$/g,'').split('|').map(s=>s.trim()));
const inline = s=>s.slice(1,-1);
const block = (page,name)=>section(page,name).split(/\r?\n/).filter(s=>s.trim()&&!s.trim().startsWith('```')).join('\n');
function key(chord) {
  const values=chord.split('+'),name=values.pop(),mods=new Set(values);
  return {key:name.length===1?name.toLowerCase():name,code:name.length===1?`Key${name.toUpperCase()}`:name,
    ctrlKey:mods.has('Control')||mods.has('Ctrl'),metaKey:mods.has('Command'),shiftKey:mods.has('Shift'),altKey:mods.has('Option')||mods.has('Alt')||mods.has('AltGraph'),getModifierState:name=>name==='AltGraph'&&mods.has('AltGraph'),preventDefault(){this.defaultPrevented=true;}};
}
const labels={reload:'Reload tags','dir-add':'Add directory','dir-favorite':'First favorite directory',extended:'Extended tags','pane-next':'Next pane',folder:'Change directory','file-delete':'Move files to Recycle Bin or trash','remove-from-list':'Remove rows from list'};
test('all20written platform shortcut cases execute actual focus-sensitive keyboard/file/panel routing under LF/CRLF',()=>{
  let total=0;
  for(const platform of paths)for(const page of [pages[platform],pages[platform].replace(/\n/g,'\r\n')]) {
    const mac=platform==='macos',prefix=mac?'mac':platform;
    const cases=rows(page,`${prefix}-key-cases`);assert.equal(cases.length,mac?8:6);
    for(const [chord,focus,expected] of cases) {
      const event=key(inline(chord)),typing=focus==='Text field';
      let actual=labels[windowShortcut(event,{typing,mac})]||labels[fileShortcut(event,typing,mac)]||labels[listShortcut(event,typing,mac)];
      let handler,toggled=0;
      const panel=new TagPanel({altShortcut:e=>altChord(e,{typing,mac})});panel.toggle=()=>{toggled++;};
      const previousWindow=globalThis.window;
      try {
        globalThis.window={addEventListener(){}};
        panel.install({addEventListener:(name,callback)=>{if(name==='keydown')handler=callback;},querySelector:()=>null});
      } finally {
        if(previousWindow===undefined)delete globalThis.window;else globalThis.window=previousWindow;
      }
      if(!actual)handler(event);
      if(toggled)actual='Toggle tag panel';
      assert.equal(actual||'Text entry',expected,`${platform}: ${chord} ${focus}`);total++;
    }
  }
  assert.equal(total,40);
});
test('desktop package examples preserve the inspected format and architecture coverage with current branding',()=>{
  assert.equal(manifest.tag,'v0.2.13');assert.equal(manifest.commit,'325007c47bef4ee53a4aa9b086038157ee3e62dc');
  const currentName=name=>name.replace(/^OpenTag-v[\d.]+/, 'TagGroove-vVERSION');
  const written=[];
  for(const platform of paths) {
    const prefix=platform==='macos'?'mac':platform;
    const names=[...section(pages[platform],`${prefix}-package-assets`).matchAll(/`([^`]+)`/g)].map(m=>m[1]);
    for(const name of names){assert(manifest.assets.some(a=>currentName(a.name)===name));written.push(name);}
  }
  const expected=manifest.assets.map(asset=>currentName(asset.name)).filter(name=>/^TagGroove-vVERSION-(windows|macos|linux)-/.test(name));
  assert.deepEqual([...written].sort(),expected.sort());assert.equal(written.length,11);
  for(const [platform,marker] of [['macos','mac-checksum'],['windows','windows-checksum'],['linux','linux-checksum'],['linux','linux-appimage-run']]) {
    const names=block(pages[platform],marker).match(/TagGroove-v[\w.-]+/g);assert(names?.length);
    for(const name of names)assert(manifest.assets.some(a=>currentName(a.name)===name),name);
  }
  for(const asset of inspected.downloaded)assert.deepEqual(asset,manifest.assets.find(a=>a.name===asset.name));
});
test('Linux dependency evidence remains historical and launch examples use the current executable',()=>{
  const control=Object.fromEntries(inspected.linux.control.split('\n').filter(line=>/^[A-Za-z-]+: /.test(line)).map(line=>{const at=line.indexOf(': ');return[line.slice(0,at),line.slice(at+2)];}));
  assert.equal(control.Package,'open-tag');assert.equal(control.Version,'0.2.13');assert.equal(control.Architecture,'amd64');
  assert.deepEqual(block(pages.linux,'linux-deb-dependencies').split('\n').sort(),control.Depends.split(', ').sort());
  const desktop=Object.values(inspected.linux.desktop_files);assert.equal(desktop.length,1);
  const executable=/^Exec=(.+)$/m.exec(desktop[0])[1];assert.equal(executable,'opentag');assert(!/MimeType=|%F/.test(desktop[0]));
  assert(block(pages.linux,'linux-launch').startsWith('taggroove '));
});
test('written folder launch destinations run through actual frontend replacement/pending-edit decisions without launching an OS process',async()=>{
  for(const platform of paths) {
    const prefix=platform==='macos'?'mac':platform;
    const command=block(pages[platform],platform==='macos'?'mac-open-document':`${prefix}-launch`);
    const path=platform==='macos'?/open -a TagGroove '([^']+)'/.exec(command)[1]:/'\/fp:([^']+)'/.exec(command)[1];
    const loads=[],requests=[],host={mac:platform==='macos',invoke:async(command,args)=>{requests.push([command,args]);if(command==='remember_directories')return{version:1,recent:args.paths,favorites:[]};},selection:()=>[],pendingCount:()=>0,openReason:()=>'',load:async(...args)=>{loads.push(args);return true;},changed:()=>{},preferences:()=>({subfolders:true,startup:'empty'})};
    const ui=new FileOpsUI(host);await ui.launch({directories:[path],files:[],add:false});
    assert.equal(loads.length,1);assert.deepEqual(loads[0],[[path],{replace:true,recursive:true,location:path}]);
    host.pendingCount=()=>1;host.confirm=async()=>false;await ui.launch({directories:[path],files:[],add:false});assert.equal(loads.length,1);
  }
});
