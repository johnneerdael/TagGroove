import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

test('Linux package and source bindings reject altered, detached and unsafe archive material',()=>{
  const python=process.env.PYTHON||(process.platform==='win32'?'python':'python3');
  const checked=spawnSync(python,['-m','unittest','discover','-s','scripts/tests','-p','test_linux_*.py','-v'],{
    cwd:fileURLToPath(new URL('../',import.meta.url)),encoding:'utf8',
  });
  const output=(checked.stdout||'')+(checked.stderr||'');
  assert.equal(checked.status,0,checked.error?.message||output);
  assert.match(output,/Ran [1-9][0-9]* tests/,'No source-material regressions were discovered');
});
