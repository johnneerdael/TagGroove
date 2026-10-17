// Translation core: catalogs keyed by the English text as the UI shows it.
// `{name}` marks a value filled in at run time; a translation may choose a form
// by number with `{count, plural, one {# file} other {# files}}` (Intl.PluralRules
// categories or `=N`, `#` is the number). Format strings, field names, placeholders
// and scripting functions stay as they are in every language.

/** A catalog file: English key → translation. `@meta` names the language. */
export type Catalog = {'@meta'?: {name?: string; locale?: string; [key: string]: unknown}} & Record<string, unknown>;
export type CatalogMeta = {name: string; locale: string};
/** English source segments with a code-owned separator; translate parts once before joining. */
export type MessageText = string | {parts: readonly string[]; separator: string};

const PLACEHOLDER = /\{([A-Za-z][A-Za-z0-9_]*)\}/g;
/** Ambiguous frames require producer parts; persisted names must not be guessed as UI. */
const PARTS_ONLY = new Set([
  '{value} / action {value2}: {error}',
  'Changed outside OpenTag after {value}; it was not {verb}',
  'It was not {verb}: {error}',
  'Folders and side files could not be {verb}: {error}',
  'Destination exists or another file maps to {value}',
  'Line {line}: {error}',
  'The cover image could not be saved: {error}',
  'Action {name}',
  'Actions ({count} groups)',
  '{value}: {error}',
  'Show {label}',
  'Show {itemLabel}',
  // These are persisted duplicate names, not standalone UI sentences.
  '{name} copy',
  '{name} copy {count}',
  'Copy of {name}',
  'Copy of {name} ({n})',
]);
/** Canonical English history framing owns one space; padding within names belongs to the name. */
const HISTORY_FRAMES = new Set([
  "{kind} {label} ({files} file)",
  "{kind} {label} ({files} files)",
  "Redo {label} ({files} file)",
  "Redo {label} ({files} files)",
  "Undoing {label}…",
  "Redoing {label}…",
  "Undid {label}: {restored} file",
  "Undid {label}: {restored} file, {length} not undone",
  "Undid {label}: {restored} file, {length} not redone",
  "Undid {label}: {restored} files",
  "Undid {label}: {restored} files, {length} not undone",
  "Undid {label}: {restored} files, {length} not redone",
  "Redid {label}: {restored} file",
  "Redid {label}: {restored} file, {length} not undone",
  "Redid {label}: {restored} file, {length} not redone",
  "Redid {label}: {restored} files",
  "Redid {label}: {restored} files, {length} not undone",
  "Redid {label}: {restored} files, {length} not redone",
]);
/** Native history producers use these fixed labels or the Action name/count scaffolds. */
const HISTORY_LABELS = new Set([
  "Save tags",
  "Remove tags",
  "Add cover",
  "Remove cover",
  "Filename → Tag",
  "Quick action",
  "Tag → Filename",
  "Tag → Tag",
  "Filename → Filename",
  "Text File → Tag",
  "Tag source import",
  "Remove tag types",
  "Remove unsupported fields",
  "Optimize",
  "Strip chapters MP4",
  "Manage covers",
  "Replace cover",
  "Cut cover",
  "Set cover type",
  "Set cover description",
  "Edit covers",
]);
/** Format operation-owned text directly, retaining original names and unknown labels as data. */
export function historyLabelText(label: string, t: (key: string, params?: Record<string, unknown>) => string): string {
  if (label.startsWith('Action ')) return t('Action {name}', {name: label.slice('Action '.length)});
  const groups = /^Actions \((\d+) groups\)$/.exec(label);
  if (groups) return t('Actions ({count} groups)', {count: groups[1]});
  return HISTORY_LABELS.has(label) ? t(label) : label;
}

const PLURAL_CATEGORIES = new Set(['zero', 'one', 'two', 'few', 'many', 'other']);
/** Data identities (files, fields, configuration keys) and authored text are never looked up themselves.
 * The audited backend source role names mapped fields, export configurations and action-context keys. */
const VERBATIM = /^(name|nameOf|path|file|filename|fileName|location|outputPath|destination|source|sourcePath|directory|folder|url|extension|tag|selector|DIRECTORY|SEPARATOR|value|values|names|text|field|op|barcode|find|trackField|trackPosition|key|id|typeId|codepage|mode|item|scheme|byte|flags|current|query|term|pattern|format|template|title)\d*$/;

// Key combinations as menus and tooltips show them: ⌘/Ctrl ⇧ S, Alt+Enter, Ctrl Num +.
const SHORTCUT = /^(?:(?:⌘|⌥|⇧|⌃|⌫|(?<![A-Za-z0-9])(?:Ctrl|Alt|Shift|Option|Cmd|Del|Num|Enter|Esc|Tab|Space|Page Up|Page Down|Home|End|Ins|Insert|F\d{1,2})(?![A-Za-z0-9])|[A-Z0-9]|[←→↑↓+\/·]|,)[\s+/]*)+$/;
/** Number-only count, measurement and limit roles: keep raw data and composed summaries distinct.
 * They only match digits, so "{count} file" never matches "Choose a file". */
const NUMERIC = /^(length|count|size|total|done|restored|missing|unmatched|pending|saved|failed|skipped|selectedCount|tracks|discs|chapters|selected|shown|started|removed|files|line|lines|matched|n|i|index|position|lastHits|lastStored|min|max|channels|bitrate|sampleRate|counts|covers|withCovers|sourceLine|selectionIndex|minimum|maximum|invalid|number|major|code|MAX_COLUMNS|MAX_FIELDS|MAX_VALUES|MIN_WIDTH|MAX_WIDTH|PLAY_LIMIT|TOOL_LIMIT|TOOLLIMIT|TIMEOUTRANGE)\d*$/;
// Anonymous/raw role names stay polymorphic except in these audited source-owned frames.
const NUMERIC_SLOTS: Readonly<Record<string, readonly string[]>> = {
  "Chapter {value}: {error}": ["value"],
  "${name} expects {min}..={max} arguments, received {value}": ["value"],
  "Cannot show {value} files here": ["value"],
  "Configuration version {value} is not supported": ["value"],
  "Cover {value} does not exist": ["value"],
  "Invalid action section at line {value}": ["value"],
  "Invalid action setting at line {value}": ["value"],
  "Setting outside an action section at line {value}": ["value"],
  "New filename references undefined part %{value}": ["value"],
  "Repeated old filename part %{value} is ambiguous": ["value"],
  "Text import has {value} lines for {value2} selected files; no changes planned": ["value", "value2"],
  "The network timeout must be between {value} and {value2} seconds": ["value", "value2"],
  "Track {value} is not in the cue sheet": ["value"],
  "Unexpected filter token at position {value}": ["value"],
  "Unsupported settings version {value}": ["value"],
  "moving {n} lines leaves the input (line {value} of {value2})": ["value", "value2"],
  "the input has only {value} lines": ["value"],
  "{name} returned HTTP {value} for {url}": ["value"],
  "{name}: {value} bytes cannot be stored in a Nero chapter tag": ["value"],
  "{label} for {count} of {length} selected file? {effect} {other} file is skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.": ["other"],
  "{label} for {count} of {length} selected file? {effect} {other} files are skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.": ["other"],
  "{label} for {count} of {length} selected files? {effect} {other} file is skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.": ["other"],
  "{label} for {count} of {length} selected files? {effect} {other} files are skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.": ["other"],
};
// Closed producer domains prevent broader UI frames absorbing rejected numeric prose.
const TAG_KIND_LISTS = new Set(['ID3v1 and ID3v2', 'ID3v1 and APE', 'ID3v2 and APE', 'ID3v1, ID3v2 and APE']);
const UTILITY_CONFIRMATIONS = new Set(["{label} for {count} of {length} selected file? {effect} Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "{label} for {count} of {length} selected file? {effect} {other} file is skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "{label} for {count} of {length} selected file? {effect} {other} files are skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "{label} for {count} of {length} selected files? {effect} Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "{label} for {count} of {length} selected files? {effect} {other} file is skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "{label} for {count} of {length} selected files? {effect} {other} files are skipped: this applies to {applies}. Files that need no change are not rewritten. Edit → Undo restores the files during this session.", "{label} applies to {applies}."]);
const UTILITY_PARTS = [{"label": "Remove unsupported fields", "effect": "ID3v2 frames OpenTag cannot display (listed by %_id3v2_unknown_frames%, such as PRIV or GEOB) are removed permanently.", "applies": "MP3 files (ID3v2 tags, also in AAC, AIFF and WAV)"}, {"label": "Optimize", "effect": "FLAC files get exactly 4 KB of padding. MP4 files lose their free space and get the movie and tag data before the audio data.", "applies": "FLAC and MP4 files"}];
const PICTURE_FRAMES = new Set(["Copied the {pictureTypeLabel} to the clipboard", "Embedded {pictureTypeLabel} artwork", "Set cover type to {pictureTypeLabel}", "{pictureTypeLabel} artwork"]);
const PICTURE_LABELS = new Set(["Other", "File icon", "Other file icon", "Front Cover", "Back Cover", "Leaflet", "Media", "Lead artist", "Artist", "Conductor", "Band or orchestra", "Composer", "Lyricist", "Recording location", "During recording", "During performance", "Screen capture", "A bright colored fish", "Illustration", "Band logo", "Publisher logo", "Cover"]);
const pictureLabel = (value: string) => PICTURE_LABELS.has(value)
  || (/^Undefined\((?:0|[1-9]\d{0,2})\)$/.test(value) && Number(value.slice(10, -1)) <= 255);
const numericParameter = (key: string, name: string) => NUMERIC.test(name) || NUMERIC_SLOTS[key]?.includes(name) === true;
const SIGNED_COUNT_KEY = 'moving {n} lines leaves the input (line {value} of {value2})';
// Han decimal numbering uses these ten symbols rather than Unicode Nd digits.
const HAN_DIGITS = '〇一二三四五六七八九';
const NUMBER_BODY = `[\\p{Nd}${HAN_DIGITS}](?:[\\p{Nd}${HAN_DIGITS}.,\\u00a0\\u202f\\u066b\\u066c'’ ]*[\\p{Nd}${HAN_DIGITS}])?`;
const NUMBER = `(${NUMBER_BODY})`;
const SIGNED_NUMBER = `(-?${NUMBER_BODY})`;
export const normalize = (text: string) => text.replace(/\s+/g, ' ').trim();

type Node = string | {name: string} | {name: string; plural: [string, Node[]][]};

/** Parse a translation: text, `{name}` and `{name, plural, cat {…} …}`. */
export function parse(source: string): Node[] {
  let index = 0;
  const nodes = (): Node[] => {
    const out: Node[] = [];let text = '', literalBraces = 0;
    while (index < source.length) {
      const c = source[index];
      if (c === '}' && literalBraces === 0) break;
      if (c === '{') {
        const close = source.indexOf('}', index), comma = source.indexOf(',', index);
        if (comma > index && (close < 0 || comma < close)) {
          const name = source.slice(index + 1, comma).trim();
          const rest = /^\s*plural\s*,/.exec(source.slice(comma + 1));
          if (rest && /^[A-Za-z][A-Za-z0-9_]*$/.test(name)) {
            if (text) out.push(text);text = '';
            index = comma + 1 + rest[0].length;
            const forms: [string, Node[]][] = [];
            for (;;) {
              const head = /^\s*(=\d+|[a-z]+)\s*\{/.exec(source.slice(index));
              if (!head) break;
              index += head[0].length;
              const body = nodes();
              if (source[index] !== '}') throw new Error(`Unclosed plural form in “${source}”`);
              index++;forms.push([head[1], body]);
            }
            const end = /^\s*\}/.exec(source.slice(index));
            if (!end || !forms.length) throw new Error(`Malformed plural in “${source}”`);
            index += end[0].length;
            out.push({name, plural: forms});
            continue;
          }
        }
        const simple = /^\{([A-Za-z][A-Za-z0-9_]*)\}/.exec(source.slice(index));
        if (simple) {if (text) out.push(text);text = '';out.push({name: simple[1]});index += simple[0].length;continue;}
        literalBraces++;
      } else if (c === '}') literalBraces--;
      text += c;index++;
    }
    if (literalBraces) throw new Error(`Unclosed literal brace in “${source}”`);
    if (text) out.push(text);
    return out;
  };
  const result = nodes();
  if (index < source.length) throw new Error(`Unexpected “}” in “${source}”`);
  return result;
}

/** Names used by a key or translation, including plural selectors. */
export function placeholders(source: string): Set<string> {
  const names = new Set<string>();
  const walk = (list: Node[]) => {for (const node of list) if (typeof node !== 'string') {names.add(node.name);if ('plural' in node) node.plural.forEach(([, body]) => walk(body));}};
  walk(parse(source));
  return names;
}

const numberSeparators = new WeakMap<Intl.PluralRules, {decimal: string; group?: string; secondary: number}>();
const asciiDigits = (value: unknown) => String(value ?? '').replace(/[\p{Nd}〇一二三四五六七八九]/gu, digit => {
    if (digit >= '0' && digit <= '9') return digit;
    const han = HAN_DIGITS.indexOf(digit);
    if (han >= 0) return String(han);
    const code = digit.codePointAt(0)!;
    let zero = code;
    // Decimal digit sets are consecutive 0–9 runs; adjacent sets (math digits)
    // can share a run, so retain the position modulo ten.
    while (zero > 0 && /\p{Nd}/u.test(String.fromCodePoint(zero - 1))) zero--;
    return String((code - zero) % 10);
});
const separatorsFor = (rules: Intl.PluralRules) => {
  let separators = numberSeparators.get(rules);
  if (!separators) {
    const parts = new Intl.NumberFormat(rules.resolvedOptions().locale).formatToParts(123456789.6);
    const integers = parts.filter(part => part.type === 'integer');
    separators = {decimal: parts.find(part => part.type === 'decimal')?.value ?? '.', group: parts.find(part => part.type === 'group')?.value,
      secondary: [...(integers.at(-2)?.value ?? '000')].length};
    numberSeparators.set(rules, separators);
  }
  return separators;
};

/** Check capture syntax before treating text as a count; retain its display glyphs. */
const formattedNumber = (value: string, rules: Intl.PluralRules): string | undefined => {
  const text = asciiDigits(value), negative = text.startsWith('-');
  const unsigned = negative ? text.slice(1) : text;
  const {decimal, group, secondary} = separatorsFor(rules);
  // These grouping glyphs also occur when system number formatting differs
  // from the selected UI language. Never mix them within a single integer.
  const groups = new Set([group, '.', ',', ' ', '\u00a0', '\u202f', '\u066c', "'", '’']);
  for (const point of new Set([decimal, '.', ',', '\u066b'])) {
    const parts = unsigned.split(point);
    if (parts.length > 2 || parts.length === 2 && !/^\d+$/.test(parts[1])) continue;
    let integer = parts[0];
    if (!/^\d+$/.test(integer)) {
      const marks = [...integer.replace(/\d/g, '')], separator = marks[0];
      if (!separator || !groups.has(separator) || marks.some(mark => mark !== separator)) continue;
      const escaped = escapeRegex(separator);
      const western = new RegExp(`^[1-9]\\d{0,2}(?:${escaped}\\d{3})+$`).test(integer);
      // Keep Indian comma/Arabic-grouped system counts supported independently
      // of UI language; other separators follow the locale's grouping width.
      const indian = (secondary === 2 || separator === ',' || separator === '\u066c')
        && new RegExp(`^[1-9]\\d?(?:${escaped}\\d{2})*${escaped}\\d{3}$`).test(integer);
      if (!western && !indian) continue;
      integer = integer.split(separator).join('');
    }
    return (negative ? '-' : '') + integer + (parts.length === 2 ? '.' + parts[1] : '');
  }
  return undefined;
};

/** Decode locale-formatted counts only for plural decisions; displayed values stay untouched. */
const countText = (value: unknown, rules: Intl.PluralRules) => {
  if (typeof value === 'number') return String(value);
  const digits = asciiDigits(value);
  const formatted = formattedNumber(digits, rules);
  if (formatted !== undefined) return formatted;
  // Explicit t() parameters also support scientific and signed numeric forms.
  const separators = separatorsFor(rules);
  const ungrouped = separators.group ? digits.split(separators.group).join('') : digits;
  return ungrouped.split(separators.decimal).join('.').replace(/٫/g, '.').replace(/[\s٬'’]/g, '');
};

/** Canonical integer text without rounding or expanding beyond the selectors. */
const exactInteger = (text: string, limit: number): string | undefined => {
  if (text === '') return '0'; // Preserve the existing empty-count numeric coercion.
  const match = /^([+-]?)(\d*)(?:\.(\d*))?(?:e([+-]?\d+))?$/i.exec(text);
  if (!match || (!match[2] && !match[3])) return undefined;
  const digits = (match[2] + (match[3] ?? '')).replace(/^0+/, '') || '0';
  if (digits === '0') return '0';
  if (match[1] === '-') return undefined;
  const shift = Number(match[4] ?? 0) - (match[3]?.length ?? 0);
  if (shift >= 0) return digits.length + shift <= limit ? digits + '0'.repeat(shift) : undefined;
  const end = digits.length + shift;
  return end > 0 && end <= limit && /^0*$/.test(digits.slice(end)) ? digits.slice(0, end) : undefined;
};

/** Fill a parsed translation. */
export function render(nodes: Node[], params: Record<string, unknown>, rules: Intl.PluralRules, hash?: string): string {
  let out = '';
  for (const node of nodes) {
    if (typeof node === 'string') {out += hash === undefined ? node : node.replace(/#/g, hash);continue;}
    const value = params[node.name];
    if (!('plural' in node)) {out += value === undefined ? `{${node.name}}` : String(value);continue;}
    const decoded = countText(value, rules), n = Number(decoded);
    // Exact forms compare decimal integer text before count conversion to
    // Number for categories; the original value still supplies `#`.
    const limit = node.plural.reduce((length, [category]) => category.startsWith('=') ? Math.max(length, category.length - 1) : length, 0);
    const integer = exactInteger(decoded, limit);
    const exact = node.plural.find(([category]) => integer !== undefined && category === `=${integer}`);
    const category = Number.isFinite(n) ? rules.select(n) : 'other';
    const form = exact ?? node.plural.find(([name]) => name === category) ?? node.plural.find(([name]) => name === 'other') ?? node.plural[0];
    out += render(form[1], params, rules, value === undefined ? '#' : String(value));
  }
  return out;
}

type Pattern = {key: string; regex: RegExp; names: string[]; numeric: Set<string>; value: Node[]; literal: number; sourceError: boolean; history: boolean};
const escapeRegex = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** Compiled catalog of one language. */
export class Translator {
  readonly locale: string;
  private exact = new Map<string, Node[]>();
  private byKey = new Map<string, Node[]>();
  private patterns: Pattern[] = [];
  private rules: Intl.PluralRules;
  private cache = new Map<string, string | null>();
  /** `@keys`: names of keyboard keys (Ctrl → Strg) in shortcut labels such as "⌘/Ctrl ⇧ S". */
  private keys: [RegExp, string][] = [];

  constructor(locale = 'en', catalog: Catalog = {}) {
    this.locale = locale;
    try {this.rules = new Intl.PluralRules(locale);} catch {this.rules = new Intl.PluralRules('en');}
    const keys = catalog['@keys'];
    if (keys && typeof keys === 'object') {
      const entries = Object.entries(keys).filter((entry): entry is [string, string] => typeof entry[1] === 'string' && !!entry[1]);
      const hasInsert = entries.some(([name]) => name === 'Insert');
      this.keys = entries.sort((a, b) => b[0].length - a[0].length).map(([name, value]) => {
        // The Actions dialog emits Insert; use a localized Ins spelling unless explicitly overridden.
        // Identity/absent mappings retain the producer's original English spelling.
        const spelling = name === 'Ins' && value !== name && !hasInsert ? '(?:Ins|Insert)' : escapeRegex(name);
        return [new RegExp(`(?<![A-Za-z0-9])${spelling}(?![A-Za-z0-9])`, 'g'), value];
      });
    }
    for (const [key, value] of Object.entries(catalog)) {
      if (key.startsWith('@') || typeof value !== 'string' || !value) continue;
      let nodes: Node[];
      try {nodes = parse(value);} catch {continue;}
      const normalized = normalize(key);
      this.byKey.set(normalized, nodes);
      if (PARTS_ONLY.has(normalized)) continue;
      const names = [...normalized.matchAll(PLACEHOLDER)].map(match => match[1]);
      if (!names.length) {this.exact.set(normalized, nodes);continue;}
      const literal = normalized.replace(PLACEHOLDER, '');
      // Keys that are mostly values ("{name}…") would match nearly any text.
      if (!/\p{L}{2,}/u.test(literal)) continue;
      const history = HISTORY_FRAMES.has(normalized);
      const numeric = new Set(names.filter(name => numericParameter(normalized, name)));
      const source = normalized.split(PLACEHOLDER).map((part, i) => i % 2 ? numeric.has(part) ? normalized === SIGNED_COUNT_KEY && part === 'n' ? SIGNED_NUMBER : NUMBER : '([\\s\\S]*?)' : escapeRegex(part).replace(/ /g, history ? ' ' : '\\s+')).join('');
      const sourceError = normalized === 'line {line} ({text}): {error}';
      this.patterns.push({key: normalized, regex: new RegExp(`^${source}$`, 'du'), names, numeric, value: nodes, literal: literal.length, sourceError, history});
    }
    // Retain the ordinary legacy frame fallback. Native web-source commands
    // transport structured parts and render them explicitly in TagSourcesUI.
    this.patterns.sort((a, b) => Number(b.sourceError) - Number(a.sourceError) || b.literal - a.literal);
  }

  /** Whether this translator changes anything (English has nothing to do). */
  get active() {return this.byKey.size > 0 || this.keys.length > 0;}

  /** `key` (English, `{name}` placeholders) in this language, filled with `params`. */
  t(key: string, params: Record<string, unknown> = {}): string {
    return render(this.byKey.get(normalize(key)) ?? parseSafe(key), params, this.rules);
  }

  /** Render one of the three producer-owned Show frames without alias guessing. */
  showLabel(role: 'name' | 'label' | 'itemLabel', value: string, verbatim: string | boolean = false): string {
    const raw = role === 'name' || verbatim === true || typeof verbatim === 'string' && normalize(value) === normalize(verbatim);
    return this.t(`Show {${role}}`, {[role]: raw ? value : this.t(value)});
  }

  /**
   * Translate text as rendered in English: an exact key, or a key with
   * placeholders whose values are taken from the text. Values that are
   * themselves UI text ("3 files", an error message) are translated too.
   * Surrounding whitespace is kept; unknown text is returned unchanged.
   * Ambiguous and punctuation-only frames require t(key, source parts); paths
   * and nested errors cannot be recovered safely by splitting rendered text.
   * `verbatim` protects a name someone entered, so a column named "Help" is
   * never shown as "Hilfe": that exact text, and values equal to it read back
   * from a key with placeholders, stay as they are. `true` protects all text:
   * only keys with placeholders apply, and none of their values are translated.
   */
  translate(text: string, depth = 0, verbatim: string | boolean = false): string {
    if (!this.active) return text;
    const core = text.trim();
    if (!core || !/\p{L}/u.test(core)) return text;
    const name = typeof verbatim === 'string' ? normalize(verbatim) : undefined;
    if (name !== undefined && normalize(core) === name) return text;
    const cached = !verbatim && depth === 0;
    let result = cached ? this.cache.get(core) : undefined;
    if (result === undefined) {
      result = this.lookup(core, depth, verbatim === true, name);
      if (cached) {if (this.cache.size > 20000) this.cache.clear();this.cache.set(core, result);}
    }
    if (result === null) return text;
    const lead = /^\s*/.exec(text)![0], trail = /\s*$/.exec(text)![0];
    return lead + result + (trail.length === text.length ? '' : trail);
  }
  private lookup(core: string, depth: number, verbatim: boolean, name?: string): string | null {
    const exact = verbatim ? undefined : this.exact.get(normalize(core));
    if (exact) return render(exact, {}, this.rules);
    for (const pattern of this.patterns) {
      const match = pattern.regex.exec(core);
      if (!match) continue;
      if (pattern.names.some((name, i) => pattern.numeric.has(name) && formattedNumber(match[i + 1], this.rules) === undefined)) continue;
      if (pattern.key === '{operation} failed' && !['Cut tags', 'Paste tags'].includes(normalize(match[pattern.names.indexOf('operation') + 1]))) continue;
      if (pattern.key === '{name} and {names}' && !TAG_KIND_LISTS.has(normalize(core))) continue;
      if (UTILITY_CONFIRMATIONS.has(pattern.key) && !UTILITY_PARTS.some(parts => (['label', 'effect', 'applies'] as const).every(name => {
        const index = pattern.names.indexOf(name);
        return index < 0 || normalize(match[index + 1]) === parts[name];
      }))) continue;
      if (PICTURE_FRAMES.has(pattern.key) && !pictureLabel(normalize(match[pattern.names.indexOf('pictureTypeLabel') + 1]))) continue;
      const params: Record<string, string> = {};
      const spacing = new Map<string, {before: string; after: string}>();
      pattern.names.forEach((param, i) => {
        const value = match[i + 1];
        const protectedValue = verbatim || normalize(value) === name
          || (param === 'pictureTypeLabel' && PICTURE_FRAMES.has(pattern.key) && normalize(value).startsWith('Undefined('));
        params[param] = !protectedValue && pattern.history && param === 'label'
          ? historyLabelText(value, (key, values) => this.t(key, values))
          : !protectedValue && depth < 2 && !VERBATIM.test(param) && !pattern.numeric.has(param) && /\p{L}/u.test(value) ? this.translate(value, depth + 1) : value;
        const [start, end] = match.indices![i + 1];
        spacing.set(param, {before: /\s*$/.exec(core.slice(0, start))![0], after: /^\s*/.exec(core.slice(end))![0]});
      });
      // Catalog keys normalize the UI scaffold; keep paragraph separators next
      // to values (such as a confirmation's file list) from the original text.
      const nodes = pattern.value.map((node, i) => {
        if (typeof node !== 'string') return node;
        const previous = pattern.value[i - 1], next = pattern.value[i + 1];
        const before = previous && typeof previous !== 'string' ? spacing.get(previous.name)?.after : undefined;
        const after = next && typeof next !== 'string' ? spacing.get(next.name)?.before : undefined;
        if (before && /[\r\n]/.test(before)) node = node.replace(/^\s*/, before);
        if (after && /[\r\n]/.test(after)) node = node.replace(/\s*$/, after);
        return node;
      });
      return render(nodes, params, this.rules);
    }
    const shortcut = verbatim ? null : this.shortcut(core);
    if (shortcut !== null) return shortcut;
    // Marks rendered in the same text node as a label stay around its translation.
    const marked = /^([✓✔●•►▸▾★☆]\s*)?(.*?)(\s*[↑↓▲▼›»])?$/su.exec(core)!;
    if ((marked[1] || marked[3]) && marked[2] && marked[2] !== core) {
      const inner = this.translate(marked[2], depth, verbatim || name || false);
      if (inner !== marked[2]) return `${marked[1] ?? ''}${inner}${marked[3] ?? ''}`;
    }
    return null;
  }
  /** A shortcut label with the local names of its keys, or null for other text. */
  private shortcut(text: string): string | null {
    if (!this.keys.length || text.length > 40 || !SHORTCUT.test(text)) return null;
    const local = this.keys.reduce((label, [name, value]) => label.replace(name, value), text);
    return local === text ? null : local;
  }
}
const parseSafe = (text: string): Node[] => {try {return parse(text);} catch {return [text];}};

/** Problems of `catalog` against the English keys: missing keys, unknown keys, placeholder mismatches. */
export function validateCatalog(english: Catalog, catalog: Catalog): {missing: string[]; unknown: string[]; placeholders: string[]; invalid: string[]} {
  const keys = Object.keys(english).filter(key => !key.startsWith('@'));
  const known = new Set(keys);
  const result = {missing: [] as string[], unknown: [] as string[], placeholders: [] as string[], invalid: [] as string[]};
  for (const key of keys) {
    const value = catalog[key];
    if (typeof value !== 'string' || !value.trim()) {result.missing.push(key);continue;}
    let names: Set<string>;
    try {names = placeholders(value);} catch {result.invalid.push(key);continue;}
    const expected = new Set([...key.matchAll(PLACEHOLDER)].map(match => match[1]));
    const same = names.size === expected.size && [...names].every(name => expected.has(name));
    if (!same) result.placeholders.push(key);
    else {
      const invalid = (nodes: Node[]): boolean => nodes.some(node => typeof node !== 'string' && 'plural' in node
        && (!node.plural.some(([category]) => category === 'other')
          || node.plural.some(([category, body]) => (!PLURAL_CATEGORIES.has(category)
            && (!/^=(?:0|[1-9]\d*)$/.test(category) || category !== `=${Number(category.slice(1))}`)) || invalid(body))));
      if (invalid(parse(value))) result.invalid.push(key);
    }
  }
  for (const key of Object.keys(catalog)) if (!key.startsWith('@') && !known.has(key)) result.unknown.push(key);
  return result;
}

/** Language and name of a catalog file, from `@meta` or its file name (`pt-BR.json`). */
export function catalogMeta(catalog: Catalog, file = ''): CatalogMeta | undefined {
  const meta = catalog['@meta'];
  const locale = typeof meta?.locale === 'string' ? meta.locale : file.replace(/\.json$/i, '');
  let canonical: string;
  try {canonical = Intl.getCanonicalLocales(locale)[0];} catch {return undefined;}
  // 'system' is the saved System default choice, never a selectable catalog.
  if (!canonical || canonical === 'system') return undefined;
  const name = typeof meta?.name === 'string' && meta.name.trim() ? meta.name.trim() : canonical;
  return {locale: canonical, name};
}

/**
 * Explicit choices use an exact available locale or English. Only 'system'
 * follows preferred languages with regional/base-language matching.
 */
export function resolveLanguage(choice: string, preferred: readonly string[], available: readonly string[]): string {
  const lower = new Map(available.map(locale => [locale.toLowerCase(), locale]));
  const find = (wanted: string) => {
    const tag = wanted.toLowerCase();
    if (lower.has(tag)) return lower.get(tag)!;
    const base = tag.split('-')[0];
    return lower.get(base) ?? available.find(locale => locale.toLowerCase().split('-')[0] === base);
  };
  if (choice && choice !== 'system') return lower.get(choice.toLowerCase()) ?? 'en';
  for (const wanted of preferred) {const found = find(wanted);if (found) return found;}
  return 'en';
}

/** Independent catalog warning source; filenames/counts never need reparsing. */
export type LanguageWarning = {key:string;params:Record<string,string|number>} | {file:string;error:string} | {message:string};
export type Language = CatalogMeta & {catalog: Catalog; source: 'bundled' | 'user'; file?: string; problems?: number};
/**
 * The languages offered: the bundled catalogs plus the catalog files read from
 * the languages folder now. A file for a bundled language replaces it; a file
 * that disappeared or became unreadable leaves no trace of its earlier contents.
 */
export function languageSet(bundled: readonly Language[], files: readonly {file: string; catalog: unknown}[], english: Catalog): {languages: Map<string, Language>; warnings: LanguageWarning[]} {
  const languages = new Map(bundled.map(language => [language.locale, language]));
  const warnings: LanguageWarning[] = [];
  for (const {file, catalog} of files) {
    if (!catalog || typeof catalog !== 'object' || Array.isArray(catalog)) {warnings.push({key:'{file}: not a catalog',params:{file}});continue;}
    const meta = catalogMeta(catalog as Catalog, file);
    if (!meta) {warnings.push({key:'{file}: unknown language',params:{file}});continue;}
    const problems = validateCatalog(english, catalog as Catalog);
    const count = problems.missing.length + problems.placeholders.length + problems.invalid.length;
    // Keys with mismatched placeholders would show the wrong values; English stands in for them.
    const usable: Catalog = {...catalog as Catalog};
    for (const key of [...problems.placeholders, ...problems.invalid]) delete usable[key];
    languages.set(meta.locale, {...meta, catalog: usable, source: 'user', file, problems: count});
    if (count) warnings.push({key:'{file}: {missing} missing, {invalid} invalid',params:{file,missing:problems.missing.length,invalid:problems.placeholders.length + problems.invalid.length}});
  }
  return {languages, warnings};
}
