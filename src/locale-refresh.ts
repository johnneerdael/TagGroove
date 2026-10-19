type LocaleHost={window:Window;document:Document;state:()=>Promise<string>;refresh:()=>Promise<void>;ready:()=>boolean;fail:(error:unknown)=>void};
/** Browser language events miss region-only OS changes. Poll the native formatter
 * fingerprint while visible and also check immediately on focus/visibility. */
export function observeLocale(host:LocaleHost){
  let current:string|undefined,running=false,stopped=false;
  const check=async()=>{
    if(stopped||running||host.document.visibilityState==='hidden'||!host.ready())return;
    running=true;
    try{const next=await host.state();if(stopped)return;if(next!==current){await host.refresh();current=next;}}
    catch(error){host.fail(error);}finally{running=false;}
  };
  const changed=()=>{void check();};
  host.window.addEventListener('languagechange',changed);host.window.addEventListener('focus',changed);host.document.addEventListener('visibilitychange',changed);
  const timer=host.window.setInterval(changed,1000);changed();
  return()=>{stopped=true;host.window.clearInterval(timer);host.window.removeEventListener('languagechange',changed);host.window.removeEventListener('focus',changed);host.document.removeEventListener('visibilitychange',changed);};
}
