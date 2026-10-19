import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {extractBackendSource} from '../scripts/i18n-extract.mjs';

const diagnostic='The audio tag publication could not be completed.';
const body=`fn publish() -> Result<(), String> { Err("${diagnostic}".into()) }`;

test('backend extraction excludes a whole Rust file explicitly gated to tests',()=>{
  assert.deepEqual(extractBackendSource(`//! Private test protocol.\n#![cfg(test)]\n${body}`),[]);
});

test('file test gating is recognized after other crate attributes and comments',()=>{
  assert.deepEqual(extractBackendSource(`/* Test support. */\n#![allow(dead_code)]\n// More documentation.\n#![cfg(test)]\n${body}`),[]);
});

test('a quoted test cfg and a nested-module cfg do not hide live diagnostics',()=>{
  const quoted=`const HELP: &str = r##"#![cfg(test)]"##;\n${body}`;
  assert.ok(extractBackendSource(quoted).includes(diagnostic));
  const nested=`mod probes { #![cfg(test)] fn probe() {} }\n${body}`;
  assert.ok(extractBackendSource(nested).includes(diagnostic));
});

test('ordinary crate attributes retain production diagnostics',()=>{
  assert.ok(extractBackendSource(`#![allow(dead_code)]\n${body}`).includes(diagnostic));
});

test('the separately compiled source replay protocol exposes no application messages',async()=>{
  const source=await readFile(new URL('../src-tauri/src/sources/web/workflow_replay.rs',import.meta.url),'utf8');
  assert.ok(/#!\[cfg\(test\)\]/.test(source),'test-only file must state its compilation boundary');
  assert.deepEqual(extractBackendSource(source),[]);
});


test('nested Rust block comments cannot turn a commented cfg into a file gate',()=>{
  const source=`/* outer\n /* nested */\n #![cfg(test)]\n*/\n${body}`;
  assert.ok(extractBackendSource(source).includes(diagnostic));
});
