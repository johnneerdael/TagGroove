import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

test('provider native script reports its summary and exits after a guarded failure', async () => {
  const script = readFileSync(new URL('./e2e/sources-4.js', import.meta.url), 'utf8');
  const calls = [];
  let clock = 0;
  await vm.runInNewContext(script, {
    window: {
      OPENTAG_E2E_OFFLINE: true,
      __TAURI_INTERNALS__: { invoke: async (command, args) => { calls.push({ command, args }); } },
    },
    // Advance the existing wait loop beyond its deadline before it needs a DOM.
    Date: { now: () => { clock += 20000; return clock; } },
    setTimeout: callback => callback(),
  });
  assert.ok(calls.some(call => call.command === 'e2e_report' && call.args.message.startsWith('FAIL ')));
  assert.deepEqual(calls.slice(-2).map(call => ({ command: call.command, ...call.args })), [
    { command: 'e2e_report', message: 'SUMMARY done' },
    { command: 'e2e_exit', code: 0 },
  ]);
});
