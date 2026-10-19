import {build} from 'esbuild';
/** Bundle actual production imports for controlled-DOM tests; CSS has no layout here. */
export async function loadProductionModule(file){
  const result=await build({entryPoints:[new URL(`../../src/${file}`,import.meta.url).pathname],bundle:true,write:false,format:'esm',loader:{'.css':'empty'}});
  return import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString('base64')}`);
}
