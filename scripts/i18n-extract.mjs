#!/usr/bin/env node
// Collects the user-visible English strings of the frontend (src/*.ts) for the
// translation catalogs in src/i18n/catalogs. Keys are the English text as the
// DOM shows it, with `{name}` for values filled in at run time:
//
//   `${count} file${count===1?'':'s'} saved` → "{count} file saved", "{count} files saved"
//
// Usage: node scripts/i18n-extract.mjs           list keys missing from en.json
//        node scripts/i18n-extract.mjs --write   rewrite en.json (keeps nothing stale)
//        node scripts/i18n-extract.mjs --backend also refresh the backend message snapshot
//        node scripts/i18n-extract.mjs --check   exit 1 when en.json is out of date
import ts from 'typescript';
import {readFileSync, readdirSync, statSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const SRC = `${root}src/`;
export const CATALOGS = `${SRC}i18n/catalogs/`;

// Placeholder sentinels while variants are assembled: text values and markup (element boundaries).
const TEXT_OPEN = '\u0001', TEXT_CLOSE = '\u0002', MARKUP = '\u0003';
const MAX_VARIANTS = 64;

// Calls whose string arguments are selectors, identifiers, storage keys or other non-UI text.
const SKIP_METHODS = new Set(['querySelector', 'querySelectorAll', 'closest', 'matches', 'addEventListener', 'removeEventListener', 'getAttribute', 'hasAttribute', 'removeAttribute', 'toggleAttribute', 'getItem', 'setItem', 'removeItem', 'createElement', 'createElementNS', 'split', 'join', 'replace', 'replaceAll', 'startsWith', 'endsWith', 'includes', 'indexOf', 'lastIndexOf', 'test', 'exec', 'match', 'matchAll', 'padStart', 'padEnd', 'add', 'remove', 'toggle', 'contains', 'has', 'get', 'set', 'delete', 'postMessage', 'setProperty', 'getPropertyValue', 'getElementById', 'insertAdjacentHTML', 'warn', 'log', 'error', 'info', 'debug', 'trace', 'execCommand', 'setData', 'getData', 'emit', 'emitTo', 'mark', 'measure', 'scrollIntoView', 'focus', 'normalize', 'toLocaleString', 'localeCompare', 'escape', 'supports', 'setPointerCapture', 'dispatch', 'getContext']);
const SKIP_FUNCTIONS = new Set(['invoke', 'coreInvoke', 'listen', 'once', 'timing', 'matchMedia', 'RegExp', 'Event', 'KeyboardEvent', 'MouseEvent', 'CustomEvent', 'PointerEvent', 'DragEvent', 'FocusEvent', 'InputEvent', 'ClipboardEvent', 'URL', 'Intl', 'Collator', 'NumberFormat', 'DateTimeFormat', 'Blob', 'File', 'TextDecoder', 'TextEncoder', 'fetch', 'convertFileSrc', 'getPath', 'setPath', 'pageValues', 'registerOptionsPage', 'perfRecord', 'require', 'Symbol', 'ResizeObserver', 'parseInt', 'parseFloat', 'Number', 'Date']);
// Object properties that hold identifiers rather than text.
const SKIP_PROPERTIES = new Set(['id', 'path', 'kind', 'type', 'key', 'op', 'className', 'class', 'href', 'src', 'field', 'code', 'format', 'extension', 'extensions', 'mime', 'icon', 'event', 'action', 'role', 'selector', 'pattern', 'template', 'default', 'separator', 'tag_type', 'source', 'target', 'mode', 'sort', 'dock', 'side', 'url', 'storage', 'storageKey', 'cursor', 'display', 'width', 'height', 'template_id', 'page', 'parent', 'shortcut', 'keys', 'accelerator', 'platform', 'state', 'scope', 'tool', 'util', 'operation', 'expression', 'placeholder_value', 'prefix', 'suffix_value', 'defaultPath', 'filename', 'element', 'preset', 'column', 'columns', 'colour', 'color', 'style', 'mask', 'regex', 'flags', 'variable', 'binding', 'dataKey', 'data', 'hash', 'digest', 'algorithm', 'encoding', 'locale', 'lang', 'mapping', 'frame', 'atom', 'tagType', 'file', 'folder', 'directory', 'version', 'release', 'tag']);
// Text and assignments of these attributes is shown to people.
const UI_ATTRIBUTES = ['title', 'aria-label', 'aria-description', 'aria-roledescription', 'aria-valuetext', 'placeholder', 'alt', 'label', 'data-hint', 'data-tooltip'];
const SKIP_ASSIGNMENTS = new Set(['className', 'id', 'type', 'href', 'src', 'value', 'cssText', 'cursor', 'display', 'width', 'height', 'download', 'scrollBehavior', 'colorScheme', 'theme', 'lang', 'dir', 'name', 'accept', 'pattern', 'autocomplete', 'inputMode', 'role', 'returnValue', 'checked', 'key', 'hash', 'search', 'pathname']);
// Call results that render markup (element boundaries) rather than text.
const MARKUP_NAMES = /(HTML|Html|Markup|markup|Bar|bar)$|^(glyph|button|menu|icon|check|radio|control|controls|row|rows|item|items|section|field|fields|options|choices|tabs|chips|toolbar|toolbars|list|table|header|footer|join|map|svg|swatch|preview|thumbnail|badge|kbd|link|cell|cells|line|lines|group|groups|page|pages|body|content|editor|inputs|input|select|summary|details|entriesMarkup)$/;
// Wrappers whose result is their argument as text: the placeholder takes the argument's name.
const TEXT_WRAPPERS = new Set(['esc', 'escape', 'String', 'publicMessage', 'displayName', 'basename', 'trim', 'toString', 'toFixed', 'toLocaleString', 'format', 'formatBytes', 'formatDuration', 'encodeURIComponent', 'attr', 'html']);

const isStringish = node => ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node) || ts.isTemplateExpression(node);
const nameOf = node => {
  while (ts.isParenthesizedExpression(node) || ts.isNonNullExpression(node) || ts.isAsExpression(node)) node = node.expression;
  if (ts.isIdentifier(node) || ts.isPrivateIdentifier(node)) return node.text;
  if (ts.isPropertyAccessExpression(node)) return node.name.text;
  if (ts.isElementAccessExpression(node)) return nameOf(node.expression);
  if (ts.isCallExpression(node)) {
    const callee = nameOf(node.expression);
    if (TEXT_WRAPPERS.has(callee) && node.arguments.length) return nameOf(ts.isPropertyAccessExpression(node.expression) && !node.arguments.length ? node.expression.expression : node.arguments[0]);
    if (TEXT_WRAPPERS.has(callee) && ts.isPropertyAccessExpression(node.expression)) return nameOf(node.expression.expression);
    return callee;
  }
  if (ts.isPrefixUnaryExpression(node) || ts.isPostfixUnaryExpression(node)) return nameOf(node.operand);
  if (ts.isBinaryExpression(node)) return nameOf(node.left);
  return 'value';
};
const camel = name => (name || 'value').replace(/^[^A-Za-z]+/, '').replace(/[^A-Za-z0-9]+(.)?/g, (_, c) => c ? c.toUpperCase() : '') || 'value';

/** Find a directly declared helper in the call's enclosing lexical blocks. */
function helperBody(call, name) {
  if (!ts.isIdentifier(call.expression)) return;
  for (let scope = call.parent; scope; scope = scope.parent) {
    if (!ts.isBlock(scope) && !ts.isSourceFile(scope)) continue;
    for (const statement of scope.statements) {
      if (ts.isFunctionDeclaration(statement) && statement.name?.text === name) return statement.body;
      if (ts.isVariableStatement(statement)) for (const declaration of statement.declarationList.declarations) {
        if (ts.isIdentifier(declaration.name) && declaration.name.text === name) {
          const value = declaration.initializer;
          return value && (ts.isArrowFunction(value) || ts.isFunctionExpression(value)) ? value.body : undefined;
        }
      }
    }
  }
}

/** Mark actual HTML origins, not plain values sharing a renderer's name. */
function markupOrigin(node, visiting = new Set()) {
  if (!node || visiting.has(node)) return false;
  visiting.add(node);
  try {
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return /<[A-Za-z!/]/.test(node.text);
    if (ts.isTemplateExpression(node) && [node.head, ...node.templateSpans.map(span => span.literal)].some(part => /<[A-Za-z!/]/.test(part.text))) return true;
    if (ts.isCallExpression(node)) {
      const name = nameOf(node.expression);
      if (TEXT_WRAPPERS.has(name)) return false;
      if (ts.isPropertyAccessExpression(node.expression)) {
        const receiver = node.expression.expression;
        if (name === 'join') return markupOrigin(receiver, visiting) || node.arguments.some(argument => markupOrigin(argument, visiting));
        if (name === 'map') return node.arguments.some(argument => (ts.isArrowFunction(argument) || ts.isFunctionExpression(argument)) && markupOrigin(argument.body, visiting));
        if (name === 'filter' || name === 'slice') return markupOrigin(receiver, visiting);
      }
      if (!MARKUP_NAMES.test(name)) return false;
      const body = helperBody(node, name);
      return body ? markupOrigin(body, visiting) : true;
    }
    // Unknown variables/properties carry text. Renderer bodies and literal
    // markup are inspected where declared, without treating a name as HTML.
    if (ts.isIdentifier(node) || ts.isPropertyAccessExpression(node)) return false;
    let found = false;
    ts.forEachChild(node, child => {if (markupOrigin(child, visiting)) found = true;});
    return found;
  } finally {visiting.delete(node);}
}

function placeholder(node, consumed) {
  let inner = node;
  while (ts.isParenthesizedExpression(inner) || ts.isNonNullExpression(inner) || ts.isAsExpression(inner)) inner = inner.expression;
  const name = nameOf(inner);
  // Markup comes from rendering helpers: element boundaries in the DOM.
  if (ts.isCallExpression(inner) && markupOrigin(inner)) return [MARKUP];
  void consumed;
  return [TEXT_OPEN + camel(name) + TEXT_CLOSE];
}

/** Every string an expression can produce, with run-time values as placeholders. */
function variants(node, consumed) {
  if (ts.isParenthesizedExpression(node) || ts.isNonNullExpression(node) || ts.isAsExpression(node)) return variants(node.expression, consumed);
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) { consumed.add(node); return [node.text]; }
  if (ts.isNumericLiteral(node)) return [node.text];
  if (ts.isTemplateExpression(node)) {
    consumed.add(node);
    const parts = [[node.head.text]];
    for (const span of node.templateSpans) parts.push(variants(span.expression, consumed), [span.literal.text]);
    return combine(parts, node, consumed);
  }
  if (ts.isConditionalExpression(node)) {
    const values = [...variants(node.whenTrue, consumed), ...variants(node.whenFalse, consumed)];
    return values.length > MAX_VARIANTS ? placeholder(node, consumed) : values;
  }
  if (ts.isBinaryExpression(node)) {
    const op = node.operatorToken.kind;
    if (op === ts.SyntaxKind.PlusToken && (stringy(node.left) || stringy(node.right))) {
      const left = variants(node.left, consumed), right = variants(node.right, consumed), out = [];
      for (const l of left) for (const r of right) out.push(l + r);
      return out.length > MAX_VARIANTS ? placeholder(node, consumed) : out;
    }
    if (op === ts.SyntaxKind.BarBarToken || op === ts.SyntaxKind.QuestionQuestionToken) {
      if (stringy(node.right)) return [...variants(node.left, consumed), ...variants(node.right, consumed)];
    }
    if (op === ts.SyntaxKind.AmpersandAmpersandToken && stringy(node.right)) return ['', ...variants(node.right, consumed)];
  }
  return placeholder(node, consumed);
}
/**
 * Strings of a template's parts. Markup splits the rendered text into separate
 * nodes, so only parts within one stretch of text are combined with each other;
 * each stretch varies on its own while the others keep their first form.
 */
const HAS_MARKUP = /<[A-Za-z!/]|\u0003/;
function combine(parts, node, consumed) {
  const stretches = [];let current = [];
  parts.forEach((alternatives, index) => {
    if (alternatives.every(a => HAS_MARKUP.test(a))) {if (current.length) stretches.push(current);stretches.push([index]);current = [];}
    else current.push(index);
  });
  if (current.length) stretches.push(current);
  const base = parts.map(alternatives => alternatives[0]);
  const out = new Set([base.join('')]);
  for (const stretch of stretches) {
    let combos = [[]];
    for (const index of stretch) {
      let alternatives = parts[index];
      if (combos.length * alternatives.length > MAX_VARIANTS) alternatives = placeholder(templateExpressionAt(node, index), consumed);
      combos = combos.flatMap(combo => alternatives.map(a => [...combo, [index, a]]));
    }
    for (const combo of combos) {const chosen = [...base];for (const [index, a] of combo) chosen[index] = a;out.add(chosen.join(''));}
  }
  return [...out];
}
/** The expression of part `index` of a template (odd parts are expressions). */
const templateExpressionAt = (node, index) => index % 2 ? node.templateSpans[(index - 1) / 2].expression : node;

const stringy = node => {
  while (ts.isParenthesizedExpression(node)) node = node.expression;
  return isStringish(node) || (ts.isConditionalExpression(node) && (stringy(node.whenTrue) || stringy(node.whenFalse))) ||
    (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.PlusToken && (stringy(node.left) || stringy(node.right)));
};

/** The outermost expression that assembles this string. */
function topOf(node) {
  let current = node;
  for (;;) {
    const parent = current.parent;
    if (!parent) return current;
    if (ts.isParenthesizedExpression(parent) || ts.isTemplateSpan(parent) || ts.isTemplateExpression(parent) || ts.isNonNullExpression(parent) || ts.isAsExpression(parent)) { current = parent; continue; }
    if (ts.isConditionalExpression(parent) && parent.condition !== current) { current = parent; continue; }
    if (ts.isBinaryExpression(parent) && [ts.SyntaxKind.PlusToken, ts.SyntaxKind.BarBarToken, ts.SyntaxKind.QuestionQuestionToken].includes(parent.operatorToken.kind)) { current = parent; continue; }
    if (ts.isBinaryExpression(parent) && parent.operatorToken.kind === ts.SyntaxKind.AmpersandAmpersandToken && parent.right === current) { current = parent; continue; }
    return current;
  }
}

/** Whether a string at `top` is code (selectors, identifiers, comparisons) rather than text. */
function skipped(top) {
  const parent = top.parent;
  if (!parent) return false;
  if (ts.isImportDeclaration(parent) || ts.isExportDeclaration(parent) || ts.isLiteralTypeNode(parent) || ts.isExternalModuleReference(parent)) return true;
  if (ts.isCaseClause(parent)) return true;
  if (ts.isElementAccessExpression(parent) && parent.argumentExpression === top) return true;
  if ((ts.isPropertyAssignment(parent) || ts.isPropertyDeclaration(parent) || ts.isMethodDeclaration(parent)) && parent.name === top) return true;
  if (ts.isPropertyAssignment(parent) && parent.initializer === top) {
    const key = parent.name && (ts.isIdentifier(parent.name) || ts.isStringLiteral(parent.name)) ? parent.name.text : '';
    if (SKIP_PROPERTIES.has(key)) return true;
  }
  if (ts.isBinaryExpression(parent)) {
    const op = parent.operatorToken.kind;
    if ([ts.SyntaxKind.EqualsEqualsEqualsToken, ts.SyntaxKind.ExclamationEqualsEqualsToken, ts.SyntaxKind.EqualsEqualsToken, ts.SyntaxKind.ExclamationEqualsToken, ts.SyntaxKind.InKeyword, ts.SyntaxKind.InstanceOfKeyword].includes(op)) return true;
    if (op === ts.SyntaxKind.EqualsToken && parent.right === top && ts.isPropertyAccessExpression(parent.left)) {
      const target = parent.left.name.text, owner = parent.left.expression;
      if (SKIP_ASSIGNMENTS.has(target)) return true;
      if (ts.isPropertyAccessExpression(owner) && ['style', 'dataset'].includes(owner.name.text)) return true;
    }
  }
  if ((ts.isCallExpression(parent) || ts.isNewExpression(parent)) && parent.arguments?.includes(top)) {
    const callee = parent.expression;
    const name = nameOf(callee);
    if (ts.isPropertyAccessExpression(callee) && nameOf(callee.expression) === 'console') return true;
    if (ts.isPropertyAccessExpression(callee) && nameOf(callee.expression) === 'CSS') return true;
    if (name === 'setAttribute') {
      const attribute = parent.arguments[0];
      return parent.arguments.indexOf(top) === 0 || !(attribute && ts.isStringLiteral(attribute) && UI_ATTRIBUTES.includes(attribute.text));
    }
    if (SKIP_METHODS.has(name) && ts.isPropertyAccessExpression(callee)) return true;
    if (SKIP_FUNCTIONS.has(name)) return true;
  }
  return false;
}

const ENTITIES = {amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", '#39': "'", nbsp: ' ', hellip: '…', times: '×', larr: '←', rarr: '→', mdash: '—', ndash: '–', middot: '·', bull: '•'};
const decode = text => text.replace(/&(#\d+|#x[0-9a-f]+|[a-z]+);/gi, (match, name) => {
  if (name[0] === '#') return String.fromCodePoint(name[1] === 'x' || name[1] === 'X' ? parseInt(name.slice(2), 16) : Number(name.slice(1)));
  return ENTITIES[name] ?? match;
});
/** Normalized key: sentinels become `{name}` (numbered when repeated) and whitespace collapses. */
function finish(text) {
  const seen = new Map();
  text = text.replace(new RegExp(`${TEXT_OPEN}([^${TEXT_CLOSE}]*)${TEXT_CLOSE}`, 'g'), (_, name) => {
    const count = (seen.get(name) ?? 0) + 1;seen.set(name, count);
    return `{${count > 1 ? name + count : name}}`;
  });
  return decode(text).replace(/\s+/g, ' ').trim();
}
const bare = key => key.replace(/\{[A-Za-z0-9]+\}/g, '');

// Format strings, scripting functions, mnemonics (`Tit&le`) and shortcuts are not translated.
const FORMAT_TOKEN = /%[A-Za-z0-9_]+%|\$[A-Za-z_]+\(|%\d/g;
const SHORTCUT = /^(?:(?:⌘\/Ctrl|⌘|Ctrl|Alt|Shift|Option|Opt|Cmd|⇧|⌥|⌃|Del|Num|Enter|Esc|Tab|Space|F\d{1,2}(?![A-Za-z])|[A-Z0-9](?![A-Za-z0-9])|[←→↑↓⌫]|\+|\/|\{[A-Za-z0-9]+\})\s*)+$/u;
const MARKUPISH = /="|\[data-|^[\w-]+=/;
function common(key) {
  const rest = bare(key).trim();
  if (!/\p{L}{2,}/u.test(rest)) return false;
  if (SHORTCUT.test(key) || MARKUPISH.test(key)) return false;
  if (/(?:^|[^\s&])&\p{L}/u.test(rest)) return false;
  const words = rest.replace(FORMAT_TOKEN, ' ').trim();
  if (new RegExp(FORMAT_TOKEN.source).test(rest) && !/\p{L}{2,}\s+\p{L}{2,}/u.test(words)) return false;
  return true;
}
/** Whether a text segment from markup is worth translating. */
function textWorthy(key) {
  const rest = bare(key);
  if (!common(key)) return false;
  // Host names, URL paths and file names in placeholders are examples, not text.
  if (!/\s/.test(rest.trim()) && /[./]/.test(rest)) return false;
  if (/^[\p{L}\d_.-]+\.(mp3|flac|m4a|txt|json|csv|xml|html|m3u8?|cue|jpg|png|svg)$/iu.test(rest.trim())) return false;
  return true;
}
/** Whether a plain string literal reads as prose shown to people. */
function proseWorthy(key) {
  if (!common(key)) return false;
  const rest = bare(key).replace(FORMAT_TOKEN, 'x').trim();
  if (/^https?:|^mailto:|^data:|^file:|^\/\/|^\.\.?\//.test(rest)) return false;
  // Sentences may quote code-like characters (`;`, `<<`); short strings with them are code.
  const sentence = /^[\p{Lu}"'(“‘]/u.test(rest) && (rest.match(/\p{L}{2,}/gu) ?? []).length >= 5 && !/=>|&&|\|\||::|\$\{/.test(rest);
  if (!sentence && /[<>{}\[\]=#$\\@^`|;]|=>|&&|\|\||::|^\.|^%|%[A-Za-z_]+%|\$[a-z]+\(|^[a-z]+:[a-z]/.test(rest)) return false;
  // A template that puts a value next to a word ("{count} selected") is text as well.
  if (/\{[A-Za-z0-9]+\}/.test(key) && / /.test(key) && /\p{L}{3,}/u.test(rest) && !/^[\w-]+\(|\b(px|em|rem|deg|ms|solid|rgba?|var)\b/.test(key)) return true;
  if (/^[a-z0-9_.\-/:*+ ]+$/.test(rest) && !/[a-z]{2,} [a-z]{2,} [a-z]{2,}/.test(rest)) return false;
  if (/^[A-Z0-9_ ]+$/.test(rest) && !/ [A-Z]{3,} /.test(rest)) return false;
  if (/^[A-Z][a-z]+(?:[A-Z][a-z0-9]*)+$/.test(rest)) return false;
  if (/^(Key|Digit|Arrow|Numpad)[A-Z0-9]/.test(rest)) return false;
  if (/^[\w-]+\/[\w.+-]+$/.test(rest)) return false;
  if (/^\d|px$|em$|%$/.test(rest) && !/ /.test(rest)) return false;
  return /^[\p{Lu}¿¡"'(“‘…]/u.test(rest) || /\p{L}+ \p{L}+/u.test(rest);
}

/** Text nodes and UI attributes of a markup variant. */
function fromMarkup(text, add) {
  // Tags (and markup placeholders) split the text; attributes carry text of their own.
  const tag = /<!--[\s\S]*?-->|<\/?[A-Za-z][^>]*>|\u0003/g;
  let last = 0, match;
  const segment = value => { const key = finish(value); if (key && textWorthy(key)) add(key, 'text'); };
  let skipDepth = 0, skipTag = '', frozen = false;
  // UI text written inside translate="no" would never be translated: reported as 'frozen'.
  const attributesOf = (markup, kind) => {
    const attributes = /([\w:-]+)\s*=\s*("([^"]*)"|'([^']*)')/g;let attribute;
    while ((attribute = attributes.exec(markup))) {
      const attributeName = attribute[1].toLowerCase(), value = attribute[3] ?? attribute[4] ?? '';
      const input = /^<input\b/i.test(markup) && /\btype="(button|submit|reset)"/i.test(markup);
      if (UI_ATTRIBUTES.includes(attributeName) || (input && attributeName === 'value')) {
        const key = finish(value);
        if (key && textWorthy(key)) add(key, kind);
      }
    }
  };
  while ((match = tag.exec(text))) {
    if (!skipDepth) segment(text.slice(last, match.index));
    else if (frozen) { const key = finish(text.slice(last, match.index)); if (key && textWorthy(key)) add(key, 'frozen'); }
    last = tag.lastIndex;
    const markup = match[0];
    const name = /^<\/?([A-Za-z][\w-]*)/.exec(markup)?.[1]?.toLowerCase();
    // Text marked translate="no", styles and scripts are not translated.
    if (skipDepth) {
      if (name === skipTag && markup.startsWith('</')) skipDepth--;
      else if (name === skipTag && !markup.endsWith('/>')) skipDepth++;
      if (frozen && skipDepth) attributesOf(markup, 'frozen');
      continue;
    }
    if (name && !markup.startsWith('</') && (/\btranslate="no"/.test(markup) || ['style', 'script', 'svg'].includes(name))) {
      skipDepth = 1; skipTag = name; frozen = /\btranslate="no"/.test(markup);
      if (frozen) attributesOf(markup, 'frozen');
      continue;
    }
    attributesOf(markup, 'attribute');
  }
  if (!skipDepth) segment(text.slice(last));
}

export function extractFile(file, source, add) {
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TS);
  const consumed = new Set();
  const visit = node => {
    // `// i18n-ignore` before a statement keeps its strings (tag values such as genres) out of the catalog.
    if (ts.isStatement(node) && (ts.getLeadingCommentRanges(source, node.pos) ?? []).some(range => source.slice(range.pos, range.end).includes('i18n-ignore'))) return;
    if (isStringish(node) && !consumed.has(node)) {
      const top = topOf(node);
      const values = variants(top, consumed);
      consumed.add(node);
      if (!skipped(top)) {
        for (const value of values) {
          if (/<[A-Za-z!/]|\u0003/.test(value)) fromMarkup(value, (key, kind) => add(key, kind, file));
          else if (/\b(?:title|aria-label|placeholder|alt|data-hint)="/.test(value)) fromMarkup(`<x ${value}>`, (key, kind) => add(key, kind, file));
          else { const key = finish(value); if (key && proseWorthy(key)) add(key, 'string', file); }
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
}

export function readIgnore() {
  const ignore = JSON.parse(readFileSync(`${SRC}i18n/extract-ignore.json`, 'utf8'));
  return {exact: new Set(ignore.exact), patterns: ignore.patterns.map(p => new RegExp(p, 'u'))};
}

/** Keys of every user-visible string in src/*.ts, with the files they come from. */
export function extract() {
  const ignore = readIgnore();
  const keys = new Map();
  const files = readdirSync(SRC).filter(name => name.endsWith('.ts') && !name.endsWith('.d.ts')).sort();
  for (const name of files) {
    extractFile(name, readFileSync(SRC + name, 'utf8'), (key, kind, file) => {
      if (kind === 'frozen') return;
      if (ignore.exact.has(key) || ignore.patterns.some(p => p.test(key))) return;
      if (!keys.has(key)) keys.set(key, new Set());
      keys.get(key).add(file);
    });
  }
  return new Map([...keys].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0));
}

// Backend messages shown in the UI: error texts of the Rust commands, with `{}` values as placeholders.
// They are a snapshot (src/i18n/backend-keys.json, `--backend` refreshes it): a message added later shows in English.
const BACKEND_DIRS = ['src-tauri/src', 'crates/tag-core/src', 'crates/library/src', 'crates/document-access/src', 'crates/audio-fingerprint/src'];
const BACKEND_MESSAGE = /(?:\bErr\(|\.ok_or\(|ok_or_else\(\|\|\s*|map_err\(\|[^|]*\|\s*)\s*(?:\{\s*)?(?:format!\(\s*)?"((?:[^"\\]|\\.)*)"/g;
// Exclude test-injection prose, not production diagnostics with technical tokens.
const INTERNAL = /Injected|fixture/i;
// Keep offsets while ignoring quoted text/comments when locating error calls
// and local helper bodies. This only balances delimiters, not Rust grammar.
const RUST_TOKENS = /r(#+)?"[\s\S]*?"\1|"(?:[^"\\]|\\.)*"|'(?:\\(?:u\{[\da-fA-F]+\}|x[\da-fA-F]{2}|.)|[^'\\])'|\/\/[^\n]*|\/\*[\s\S]*?\*\//gu;
const rustCode = source => {
  const tokens=new RegExp(RUST_TOKENS);let code='',from=0,match;
  while((match=tokens.exec(source))){
    let end=tokens.lastIndex;
    if(match[0].startsWith('/*')){
      let depth=1;end=match.index+2;
      while(end<source.length&&depth){
        if(source.startsWith('/*',end)){depth++;end+=2;}
        else if(source.startsWith('*/',end)){depth--;end+=2;}
        else end++;
      }
      tokens.lastIndex=end;
    }
    code+=source.slice(from,match.index)+source.slice(match.index,end).replace(/[^\n]/g,' ');from=end;
  }
  return code+source.slice(from);
};
function rustEnd(code, start, open='(', close=')') {
  let depth=1;for(let index=start+1;index<code.length;index++){if(code[index]===open)depth++;else if(code[index]===close&&!--depth)return index;}
  return code.length;
}
function rustArguments(code,start,end){
  const args=[];let from=start+1,depth=0;
  for(let index=from;index<end;index++){if('([{'.includes(code[index]))depth++;else if(')]}'.includes(code[index]))depth--;else if(code[index]===','&&!depth){args.push([from,index]);from=index+1;}}
  args.push([from,end]);return args;
}
function rustMessages(source, returned=true){
  const code=rustCode(source),messages=new Set(),macros=[];
  const dataField=at=>{const field=/\b([A-Za-z_]\w*)\s*(?::|=)\s*$/.exec(code.slice(0,at));return field&&!['error','message','what','reason'].includes(field[1]);};
  for(const match of code.matchAll(/\b(?:format|write)!\s*\(/g)){
    const start=match.index+match[0].lastIndexOf('('),end=rustEnd(code,start);macros.push([start,end]);
    if(returned&&dataField(match.index))continue;
    const first=[...source.slice(start+1,end).matchAll(RUST_TOKENS)].find(token=>token[0].startsWith('"'));if(first)messages.add(first[0].slice(1,-1));
  }
  for(const match of source.matchAll(RUST_TOKENS)){
    if(!match[0].startsWith('"'))continue;
    if(macros.some(([start,end])=>match.index>start&&match.index<end))continue;
    if(returned&&dataField(match.index))continue;
    const after=match.index+match[0].length;
    if(!returned||/^\s*\.(?:into|to_string|to_owned)\s*\(/.test(code.slice(after)))messages.add(match[0].slice(1,-1));
  }
  return [...messages];
}
const backendKey = text => {
  const used=new Map();
  return text.replace(/\\n/g,' ').replace(/\\"/g,'"').replace(/\\\\/g,'\\')
    .replace(/\{\{|\}\}|\{([A-Za-z_][A-Za-z0-9_]*)?(?::[^}]*)?\}/g,(all,name)=>{
      if(all==='{{')return'{';if(all==='}}')return'}';
      const base=name?name.replace(/_([a-z])/g,(_,c)=>c.toUpperCase()):'value',count=(used.get(base)??0)+1;
      used.set(base,count);return`{${count>1?base+count:base}}`;
    }).replace(/\s+/g,' ').trim();
};
// Explicit typed causes carry owned keys in their first argument. Parameters,
// external values and arbitrary calls passed to owned_text remain data.
function typedCauseMessages(source) {
  const code=rustCode(source),messages=[];
  for(const match of code.matchAll(/\bErrorCause::(?:owned|owned_text|message|wrap|children)\s*\(/g)){
    const start=match.index+match[0].lastIndexOf('('),end=rustEnd(code,start),first=rustArguments(code,start,end)[0];
    const argument=first?source.slice(...first).trim():'';
    if(/^(?:"|format!\s*\()/.test(argument))messages.push(...rustMessages(argument,false));
  }
  return messages;
}
/** Direct errors plus static local factories reached from actual error contexts. */
export function extractBackendSource(source){
  // A separately compiled test-support file may not contain `mod tests`.
  // Read only leading inner attributes; a cfg in a string or nested module
  // must never suppress the containing file's production diagnostics.
  const leading=rustCode(source);let at=0;
  for(;;){
    const attribute=/^\s*#!\s*\[/.exec(leading.slice(at));if(!attribute)break;
    const start=at+attribute[0].lastIndexOf('['),end=rustEnd(leading,start,'[',']');
    if(/^\s*cfg\s*\(\s*test\s*\)\s*$/.test(leading.slice(start+1,end)))return [];
    at=end+1;
  }
  const tests=source.search(/#\[cfg\(test\)\]\s*mod tests/);if(tests>=0)source=source.slice(0,tests);
  const code=rustCode(source),keys=new Set(),roots=[],names=new Set(),errorTypes=new Set();
  for(const match of code.matchAll(/\bimpl\s+std::error::Error\s+for\s+([A-Za-z_]\w*)\s*\{/g))errorTypes.add(match[1]);
  // Error prose may begin with a function token, quote, header or lowercase word.
  const add=(message,wrapped=false)=>{const key=backendKey(message);if(!INTERNAL.test(key)&&common(key)&&(wrapped||/\p{L}{2,}[\s,]+\p{L}{2,}/u.test(bare(key))))keys.add(key);};
  for(const match of source.matchAll(BACKEND_MESSAGE))if(code.slice(match.index,match.index+2).trim())add(match[1]);
  typedCauseMessages(source).forEach(message=>add(message,true));
  for(const match of code.matchAll(/\bErr\s*\(|\.(?:map_err|ok_or|ok_or_else)\s*\(/g)){
    const start=match.index+match[0].lastIndexOf('('),end=rustEnd(code,start);roots.push([start,end]);
    const body=code.slice(start+1,end),alias=/^\s*([A-Za-z_]\w*)\s*$/.exec(body);if(alias)names.add(alias[1]);
    for(const call of body.matchAll(/\b([A-Za-z_]\w*)\s*\(/g))names.add(call[1]);
    // Direct typed errors need no factory call. Read only diagnostic fields;
    // command/path/source fields and format arguments remain authored data.
    for(const literal of body.matchAll(/\b(\w*Error)\s*\{/g)){
      const at=start+1+literal.index+literal[0].lastIndexOf('{'),last=rustEnd(code,at,'{','}');
      errorTypes.add(literal[1]);
      for(const [from,to] of rustArguments(code,at,last))if(/^\s*(?:message|error|what|reason)\s*:/.test(code.slice(from,to)))rustMessages(source.slice(from,to)).forEach(message=>add(message,true));
    }
  }
  const factories=new Map();
  const register=(name,header,body)=>{
    const messages=[...rustMessages(body),...typedCauseMessages(body)];messages.forEach(message=>add(message,true));
    const types=[...rustCode(body).matchAll(/\b(\w*Error)\s*\{/g)].map(match=>match[1]);types.forEach(type=>errorTypes.add(type));
    if(!messages.length&&!types.length)return;
    const parameters=header.split(',').map((parameter,index)=>/^\s*(?:mut\s+)?(error|message|what)\s*:\s*(?:&\s*)?(?:str|String)\b/.test(parameter)?index:-1).filter(index=>index>=0);
    factories.set(name,new Set([...(factories.get(name)??[]),...parameters]));
  };
  for(const name of names){
    for(const match of code.matchAll(new RegExp(`\\bfn\\s+${name}\\s*\\(`,'g'))){
      const args=match.index+match[0].lastIndexOf('('),end=rustEnd(code,args),start=code.indexOf('{',end);if(start<0)continue;
      register(name,source.slice(args+1,end),source.slice(start+1,rustEnd(code,start,'{','}')));
    }
    for(const match of code.matchAll(new RegExp(`\\blet\\s+${name}\\s*=\\s*\\|([^|]*)\\|`,'g'))){
      const from=match.index+match[0].length,start=code.slice(from).search(/\S/)+from,end=code[start]==='{'?rustEnd(code,start,'{','}'):code.indexOf(';',from);
      register(name,source.slice(match.index,from).match(/\|([^|]*)\|/)[1],source.slice(from,end<0?source.length:end));
    }
  }
  for(const [start,end] of roots)for(const match of code.slice(start+1,end).matchAll(/\b([A-Za-z_]\w*)\s*\(/g)){
    const parameters=factories.get(match[1]);if(!parameters?.size)continue;
    const at=start+1+match.index+match[0].lastIndexOf('('),args=rustArguments(code,at,rustEnd(code,at));
    for(const index of parameters)if(args[index])rustMessages(source.slice(...args[index]),false).forEach(message=>add(message,true));
  }
  for(const type of errorTypes)for(const match of code.matchAll(new RegExp(`\\bimpl\\s+(?:\\w+::)*Display\\s+for\\s+${type}\\s*\\{`,'g'))){
    const start=match.index+match[0].lastIndexOf('{');rustMessages(source.slice(start+1,rustEnd(code,start,'{','}')),false).forEach(message=>add(message,true));
  }
  return [...keys].sort();
}
export function extractBackend() {
  const files = [];
  const walk = dir => {for (const name of readdirSync(dir)) {const path = join(dir, name);if (statSync(path).isDirectory()) walk(path);else if (name.endsWith('.rs') && name !== 'tests.rs') files.push(path);}};
  for (const dir of BACKEND_DIRS) walk(root + dir);
  const keys = new Set();
  for (const file of files.sort()) {
    let source = readFileSync(file, 'utf8');
    for (const key of extractBackendSource(source)) keys.add(key);
  }
  return [...keys].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
}

/** UI text written inside translate="no" markup, where it would stay English: `file: text`. */
export function extractFrozen() {
  const found = [];
  for (const name of readdirSync(SRC).filter(name => name.endsWith('.ts') && !name.endsWith('.d.ts')).sort())
    extractFile(name, readFileSync(SRC + name, 'utf8'), (key, kind, file) => {if (kind === 'frozen') found.push(`${file}: ${key}`);});
  return [...new Set(found)];
}

/** The English catalog: every key maps to itself, plus hand-added keys and backend messages. */
export function englishCatalog(keys = extract()) {
  const extra = JSON.parse(readFileSync(`${SRC}i18n/extra-keys.json`, 'utf8'));
  const backend = JSON.parse(readFileSync(`${SRC}i18n/backend-keys.json`, 'utf8'));
  const all = [...new Set([...keys.keys(), ...extra, ...backend])].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  return Object.fromEntries(all.map(key => [key, key]));
}

if (import.meta.url === `file://${process.argv[1]}` || process.argv[1]?.endsWith('i18n-extract.mjs')) {
  const mode = process.argv[2];
  if (mode === '--backend') writeFileSync(`${SRC}i18n/backend-keys.json`, JSON.stringify(extractBackend(), null, 1) + '\n');
  const keys = extract();
  const catalog = englishCatalog(keys);
  const path = `${CATALOGS}en.json`;
  const text = JSON.stringify({'@meta': {name: 'English', locale: 'en'}, ...catalog}, null, 1) + '\n';
  let current = '';
  try { current = readFileSync(path, 'utf8'); } catch { /* first run */ }
  if (mode === '--write' || mode === '--backend') { writeFileSync(path, text); console.log(`${Object.keys(catalog).length} keys written to src/i18n/catalogs/en.json`); }
  else if (mode === '--check') {
    if (current !== text) { console.error('src/i18n/catalogs/en.json is out of date: run node scripts/i18n-extract.mjs --write and translate the new keys.'); process.exit(1); }
  } else if (mode === '--list') {
    for (const [key, files] of keys) console.log(`${[...files].join(',')}\t${key}`);
  } else {
    const known = current ? JSON.parse(current) : {};
    const missing = Object.keys(catalog).filter(key => !(key in known)), stale = Object.keys(known).filter(key => key !== '@meta' && !(key in catalog));
    console.log(`${Object.keys(catalog).length} keys; ${missing.length} missing from en.json, ${stale.length} no longer used`);
    for (const key of missing) console.log(`+ ${key}`);
    for (const key of stale) console.log(`- ${key}`);
  }
}
