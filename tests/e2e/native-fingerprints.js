// Offline real-app smoke test. Uses the embedded backend and bundled notices;
// never executes a media program, queries a provider, or writes audio files.
(async () => {
  const invoke = (command,args) => window.__TAURI_INTERNALS__.invoke(command,args);
  const report = message => invoke('e2e_report',{message});
  const waitFor = async (read,what) => { const end=Date.now()+10000; while(Date.now()<end){const value=read();if(value)return value;await new Promise(resolve=>setTimeout(resolve,100));}throw new Error('timeout: '+what); };
  let passed=0,failed=0;
  const check = async (condition,message) => { if(condition)passed++;else failed++;await report(`${condition?'PASS':'FAIL'} ${message}`); };
  try {
    await check(window.OPENTAG_E2E_OFFLINE===true,'offline native fingerprint UI run');
    const info=await invoke('audio_fingerprint_backend');
    await check(info.chromaprint==='1.6.2'&&info.ffmpeg==='9.0.2','actual linked backend versions');
    await check(info.audio_decoders===218&&info.non_audio_decoders===0&&info.protocols===0,'audio-only decoder registration with no URL protocols');
    await waitFor(()=>document.querySelector('button[data-op="about"]'),'About command');
    document.querySelector('button[data-op="about"]').click();
    const about=await waitFor(()=>[...document.querySelectorAll('dialog[open]')].find(dialog=>dialog.textContent.includes('Built-in audio fingerprinting')),'About notices');
    await check(about.textContent.includes('LGPL 2.1-or-later')&&about.textContent.includes('modifying and relinking'),'component license and relink notice visible offline');
    await check(!!about.querySelector('#audio-source-release'),'matching release/source route available');
    const license=[...about.querySelectorAll('details')].find(detail=>detail.querySelector('summary')?.textContent.includes('FFmpeg 9.0.2 — LGPL 2.1 license'));
    await check(!!license&&license.textContent.includes('GNU LESSER GENERAL PUBLIC LICENSE'),'complete LGPL text bundled in About');
    if(license)license.open=true;
    await check(about.scrollWidth<=about.clientWidth+1,'long license text fits the dialog without horizontal overflow');
    await report('VISUAL_READY About native component notices');
    // An optional bounded visual pause is used by local screenshot inspection.
    if(window.OPENTAG_NATIVE_VISUAL_PAUSE)await new Promise(resolve=>setTimeout(resolve,15000));
    about.querySelector('[data-close]').click();
    document.querySelector('button[data-op="source-options"]').click();
    const options=await waitFor(()=>document.querySelector('dialog.options-dialog[open]'),'Tag Sources options');
    options.querySelector('[data-page="tag-sources-online"]').click();
    const override=await waitFor(()=>options.querySelector('[data-option="pages.tag-sources-online.fpcalc"]'),'optional override');
    await check(override.value===''&&override.placeholder==='Leave empty for built-in fingerprinting','default needs no executable path');
    await check(options.textContent.includes('no extra program is needed')&&!options.textContent.includes('OpenTag does not include it'),'settings teach the included producer');
    await report('VISUAL_READY Optional desktop override settings');
    if(window.OPENTAG_NATIVE_VISUAL_PAUSE)await new Promise(resolve=>setTimeout(resolve,15000));
    options.querySelector('#options-cancel').click();
  }catch(error){failed++;await report('FAIL '+String(error?.stack||error));}
  await report(`SUMMARY native-fingerprints ${passed} passed, ${failed} failed`);
  await invoke('e2e_exit',{code:failed?1:0});
})();
