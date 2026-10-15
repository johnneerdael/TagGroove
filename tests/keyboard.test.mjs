import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const load = async name => {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(javascript).toString('base64')}`);
};
const { isMacPlatform, isTextEntry, chordKey, altChord, windowShortcut, nextPane } = await load('keyboard');

const press = (key, code, modifiers = {}) => ({ key, code, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...modifiers });
const mac = { typing: false, mac: true }, macTyping = { typing: true, mac: true };
const pc = { typing: false, mac: false }, pcTyping = { typing: true, mac: false };

test('platform and text-entry detection', () => {
  assert.equal(isMacPlatform('MacIntel'), true);
  assert.equal(isMacPlatform('iPad'), true);
  assert.equal(isMacPlatform('Win32'), false);
  assert.equal(isMacPlatform('Linux x86_64'), false);
  assert.equal(isTextEntry({ tagName: 'INPUT', type: 'text' }), true);
  assert.equal(isTextEntry({ tagName: 'INPUT', type: 'search' }), true);
  assert.equal(isTextEntry({ tagName: 'INPUT' }), true);
  assert.equal(isTextEntry({ tagName: 'TEXTAREA' }), true);
  assert.equal(isTextEntry({ tagName: 'DIV', isContentEditable: true }), true);
  assert.equal(isTextEntry({ tagName: 'INPUT', type: 'checkbox' }), false);
  assert.equal(isTextEntry({ tagName: 'TR' }), false);
  assert.equal(isTextEntry(null), false);
});

test('chord keys survive Option, AltGr and non-Latin layouts', () => {
  assert.equal(chordKey(press('t', 'KeyT')), 't');
  assert.equal(chordKey(press('T', 'KeyT')), 't');
  assert.equal(chordKey(press('†', 'KeyT')), 't');
  assert.equal(chordKey(press('§', 'Digit6')), '6');
  assert.equal(chordKey(press('^', 'Digit6')), '6');
  assert.equal(chordKey(press('е', 'KeyT')), 't');
  assert.equal(chordKey(press('Enter', 'Enter')), 'Enter');
  // AZERTY: the A key reports code KeyQ; the typed Latin letter wins.
  assert.equal(chordKey(press('a', 'KeyQ')), 'a');
});

test('macOS: Option+letter types characters in text fields instead of triggering Alt shortcuts', () => {
  for (const [key, code] of [['ˆ', 'KeyI'], ['Dead', 'KeyI'], ['å', 'KeyA'], ['©', 'KeyG'], ['®', 'KeyR'], ['µ', 'KeyM'], ['§', 'Digit6'], ['†', 'KeyT'], ['¬', 'KeyL']]) {
    assert.equal(altChord(press(key, code, { altKey: true }), macTyping), false, `${key} while typing`);
    assert.equal(altChord(press(key, code, { altKey: true }), mac), true, `${key} outside text fields`);
    assert.equal(altChord(press(key, code, { altKey: true, ctrlKey: true }), macTyping), true, `Control+Option+${code} while typing`);
  }
  assert.equal(windowShortcut(press('§', 'Digit6', { altKey: true }), macTyping), undefined);
  assert.equal(windowShortcut(press('ﬂ', 'Digit6', { altKey: true, shiftKey: true }), macTyping), undefined);
  assert.equal(windowShortcut(press('†', 'KeyT', { altKey: true }), macTyping), undefined);
  assert.equal(windowShortcut(press('§', 'Digit6', { altKey: true }), mac), 'groups');
  assert.equal(windowShortcut(press('†', 'KeyT', { altKey: true }), mac), 'extended');
  assert.equal(windowShortcut(press('6', 'Digit6', { altKey: true, ctrlKey: true }), macTyping), 'groups');
  assert.equal(windowShortcut(press('t', 'KeyT', { altKey: true, ctrlKey: true }), macTyping), 'extended');
  assert.equal(altChord(press('t', 'KeyT', { altKey: true, metaKey: true }), mac), false);
});

test('Windows/Linux: Alt shortcuts work while typing; AltGr chords stay text', () => {
  assert.equal(windowShortcut(press('6', 'Digit6', { altKey: true }), pcTyping), 'groups');
  assert.equal(windowShortcut(press('^', 'Digit6', { altKey: true, shiftKey: true }), pcTyping), 'action');
  assert.equal(windowShortcut(press('t', 'KeyT', { altKey: true }), pcTyping), 'extended');
  // Windows reports AltGr as Ctrl+Alt (German AltGr+M is µ, Polish AltGr+A is ą).
  assert.equal(altChord(press('µ', 'KeyM', { altKey: true, ctrlKey: true }), pcTyping), false);
  assert.equal(altChord(press('ą', 'KeyA', { altKey: true, ctrlKey: true }), pc), false);
  // Linux reports AltGraph as its own modifier.
  assert.equal(altChord(press('ł', 'KeyL', { altKey: true, getModifierState: key => key === 'AltGraph' }), pcTyping), false);
  assert.equal(altChord(press('l', 'KeyL', { altKey: true, getModifierState: () => false }), pcTyping), true);
});

test('Alt+Enter shows the file in the file manager only outside text fields', () => {
  assert.equal(windowShortcut(press('Enter', 'Enter', { altKey: true }), pc), 'file-reveal');
  assert.equal(windowShortcut(press('Enter', 'Enter', { altKey: true }), pcTyping), undefined);
  assert.equal(windowShortcut(press('Enter', 'Enter', { altKey: true }), mac), 'file-reveal');
  assert.equal(windowShortcut(press('Enter', 'Enter'), pc), undefined);
});

test('primary-modifier shortcuts use Ctrl elsewhere and Command on macOS', () => {
  assert.equal(windowShortcut(press('t', 'KeyT', { ctrlKey: true }), pc), 'reload');
  assert.equal(windowShortcut(press('t', 'KeyT', { metaKey: true }), mac), 'reload');
  assert.equal(windowShortcut(press('t', 'KeyT', { ctrlKey: true }), mac), undefined);
  assert.equal(windowShortcut(press('t', 'KeyT', { metaKey: true }), pc), undefined);
  assert.equal(windowShortcut(press('b', 'KeyB', { ctrlKey: true }), pcTyping), 'discard');
  assert.equal(windowShortcut(press('b', 'KeyB', { metaKey: true }), macTyping), 'discard');
  assert.equal(windowShortcut(press('K', 'KeyK', { ctrlKey: true, shiftKey: true }), pc), 'number-folders');
  assert.equal(windowShortcut(press('K', 'KeyK', { ctrlKey: true, shiftKey: true }), pcTyping), undefined);
  assert.equal(windowShortcut(press('k', 'KeyK', { ctrlKey: true }), pc), undefined, 'plain Ctrl+K stays with the numbering wizard binding');
  assert.equal(windowShortcut(press('+', 'NumpadAdd', { ctrlKey: true }), pc), 'autosize-columns');
  assert.equal(windowShortcut(press('+', 'NumpadAdd', { metaKey: true }), mac), 'autosize-columns');
  assert.equal(windowShortcut(press('+', 'Equal', { ctrlKey: true, shiftKey: true }), pc), undefined, 'main-keyboard plus is browser zoom territory');
});

test('Ctrl+Tab keeps the physical Control key on macOS (⌘Tab switches apps)', () => {
  assert.equal(windowShortcut(press('Tab', 'Tab', { ctrlKey: true }), pc), 'pane-next');
  assert.equal(windowShortcut(press('Tab', 'Tab', { ctrlKey: true, shiftKey: true }), pcTyping), 'pane-previous');
  assert.equal(windowShortcut(press('Tab', 'Tab', { ctrlKey: true }), mac), 'pane-next');
  assert.equal(windowShortcut(press('Tab', 'Tab', { metaKey: true }), mac), undefined);
  assert.equal(windowShortcut(press('Tab', 'Tab'), pc), undefined);
});

test('pane cycling follows Tag Panel, Filter, File List and skips missing panes', () => {
  const all = ['panel', 'filter', 'list'];
  assert.equal(nextPane('list', all, false), 'panel');
  assert.equal(nextPane('panel', all, false), 'filter');
  assert.equal(nextPane('filter', all, false), 'list');
  assert.equal(nextPane('panel', all, true), 'list');
  assert.equal(nextPane(undefined, all, false), 'panel');
  assert.equal(nextPane(undefined, all, true), 'list');
  assert.equal(nextPane('list', ['filter', 'list'], false), 'filter');
  assert.equal(nextPane('panel', ['filter', 'list'], false), 'filter');
  assert.equal(nextPane('list', [], false), undefined);
});
