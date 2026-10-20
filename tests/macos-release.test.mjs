import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';

test('macOS release signing helper contracts run without Apple tools or credentials', () => {
  const python = process.env.PYTHON || (process.platform === 'win32' ? 'python' : 'python3');
  const checked = spawnSync(python, ['-m', 'unittest', 'discover', '-s', 'scripts/tests', '-p', 'test_macos_release.py', '-v'],
    {encoding: 'utf8'});
  assert.equal(checked.status, 0, checked.stdout + checked.stderr);
});
