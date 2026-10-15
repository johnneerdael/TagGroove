// Translation core: catalogs keyed by the English text as the UI shows it.
// `{name}` marks a value filled in at run time; a translation may choose a form
// by number with `{count, plural, one {# file} other {# files}}` (Intl.PluralRules
// categories or `=N`, `#` is the number). Format strings, field names, placeholders
// and scripting functions stay as they are in every language.

/** A catalog file: English key → translation. `@meta` names the language. */
export type Catalog = {'@meta'?: {name?: string; locale?: string; [key: string]: unknown}} & Record<string, unknown>;
export type CatalogMeta = {name: string; locale: string};

const PLACEHOLDER = /\{([A-Za-z][A-Za-z0-9]*)\}/g;
const PLURAL_CATEGORIES = new Set(['zero', 'one', 'two', 'few', 'many', 'other']);
/** Values that name files, folders or people's own text are never looked up themselves. */
const VERBATIM = /^(name|path|filename|fileName|location|outputPath|destination|sourcePath|directory|folder|value|values|names|text|field|current|query|term|pattern|format|template|title)\d*$/;

// Key combinations as menus and tooltips show them: ⌘/Ctrl ⇧ S, Alt+Enter, Ctrl Num +.
const SHORTCUT = /^(?:(?:⌘|⌥|⇧|⌃|⌫|Ctrl|Alt|Shift|Option|Cmd|Del|Num|Enter|Esc|Tab|Space|Page Up|Page Down|Home|End|Ins|F\d{1,2}|[A-Z0-9]|[←→↑↓+\/·]|,)[\s+/]*)+$/;
/** Values with these names are counts: they only match digits, so "{count} file" never matches "Choose a file". */
const NUMERIC = /^(length|count|size|total|done|restored|missing|unmatched|pending|saved|failed|skipped|selectedCount|tracks|discs|chapters|selected|shown|started|removed|files|n|i|index|position|lastHits|lastStored|max)\d*$/;
const NUMBER = '(\\d(?:[\\d.,\\u00a0\\u202f\' ]*\\d)?)';
export const normalize = (text: string) => text.replace(/\s+/g, ' ').trim();

type Node = string | {name: string} | {name: string; plural: [string, Node[]][]};

/** Parse a translation: text, `{name}` and `{name, plural, cat {…} …}`. */
export function parse(source: string): Node[] {
  let index = 0;
  const nodes = (inside: boolean): Node[] => {
    const out: Node[] = [];let text = '';
    while (index < source.length) {
      const c = source[index];
      if (c === '}' && inside) break;
      if (c === '{') {
        const close = source.indexOf('}', index), comma = source.indexOf(',', index);
        if (comma > index && (close < 0 || comma < close)) {
          const name = source.slice(index + 1, comma).trim();
          const rest = /^\s*plural\s*,/.exec(source.slice(comma + 1));
          if (rest && /^[A-Za-z][A-Za-z0-9]*$/.test(name)) {
            if (text) out.push(text);text = '';
            index = comma + 1 + rest[0].length;
            const forms: [string, Node[]][] = [];
            for (;;) {
              const head = /^\s*(=\d+|[a-z]+)\s*\{/.exec(source.slice(index));
              if (!head) break;
              index += head[0].length;
              const body = nodes(true);
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
        const simple = /^\{([A-Za-z][A-Za-z0-9]*)\}/.exec(source.slice(index));
        if (simple) {if (text) out.push(text);text = '';out.push({name: simple[1]});index += simple[0].length;continue;}
      }
      text += c;index++;
    }
    if (text) out.push(text);
    return out;
  };
  const result = nodes(false);
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

const number = (value: unknown) => typeof value === 'number' ? value : Number(String(value ?? '').replace(/[\s,.  '](?=\d{3}\b)/g, ''));

/** Fill a parsed translation. */
export function render(nodes: Node[], params: Record<string, unknown>, rules: Intl.PluralRules, hash?: string): string {
  let out = '';
  for (const node of nodes) {
    if (typeof node === 'string') {out += hash === undefined ? node : node.replace(/#/g, hash);continue;}
    const value = params[node.name];
    if (!('plural' in node)) {out += value === undefined ? `{${node.name}}` : String(value);continue;}
    const n = number(value);
    const exact = node.plural.find(([category]) => category === `=${n}`);
    const category = Number.isFinite(n) ? rules.select(n) : 'other';
    const form = exact ?? node.plural.find(([name]) => name === category) ?? node.plural.find(([name]) => name === 'other') ?? node.plural[0];
    out += render(form[1], params, rules, value === undefined ? '#' : String(value));
  }
  return out;
}

type Pattern = {regex: RegExp; names: string[]; value: Node[]; literal: number};
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
    if (keys && typeof keys === 'object') this.keys = Object.entries(keys).filter((entry): entry is [string, string] => typeof entry[1] === 'string' && !!entry[1])
      .sort((a, b) => b[0].length - a[0].length).map(([name, value]) => [new RegExp(`(?<![A-Za-z])${escapeRegex(name)}(?![A-Za-z])`, 'g'), value]);
    for (const [key, value] of Object.entries(catalog)) {
      if (key.startsWith('@') || typeof value !== 'string' || !value) continue;
      let nodes: Node[];
      try {nodes = parse(value);} catch {continue;}
      const normalized = normalize(key);
      this.byKey.set(normalized, nodes);
      const names = [...normalized.matchAll(PLACEHOLDER)].map(match => match[1]);
      if (!names.length) {this.exact.set(normalized, nodes);continue;}
      const literal = normalized.replace(PLACEHOLDER, '');
      // Keys that are mostly values ("{name}…") would match nearly any text.
      if (!/\p{L}{2,}/u.test(literal)) continue;
      const source = normalized.split(PLACEHOLDER).map((part, i) => i % 2 ? NUMERIC.test(part) ? NUMBER : '([\\s\\S]*?)' : escapeRegex(part).replace(/ /g, '\\s+')).join('');
      this.patterns.push({regex: new RegExp(`^${source}$`, 'd'), names, value: nodes, literal: literal.length});
    }
    this.patterns.sort((a, b) => b.literal - a.literal);
  }

  /** Whether this translator changes anything (English has nothing to do). */
  get active() {return this.exact.size > 0 || this.patterns.length > 0 || this.keys.length > 0;}

  /** `key` (English, `{name}` placeholders) in this language, filled with `params`. */
  t(key: string, params: Record<string, unknown> = {}): string {
    return render(this.byKey.get(normalize(key)) ?? parseSafe(key), params, this.rules);
  }

  /**
   * Translate text as rendered in English: an exact key, or a key with
   * placeholders whose values are taken from the text. Values that are
   * themselves UI text ("3 files", an error message) are translated too.
   * Surrounding whitespace is kept; unknown text is returned unchanged.
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
    const cached = !verbatim;
    let result = cached ? this.cache.get(core) : undefined;
    if (result === undefined) {
      result = this.lookup(core, depth, verbatim === true, name);
      if (cached && depth === 0) {if (this.cache.size > 20000) this.cache.clear();this.cache.set(core, result);}
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
      const params: Record<string, string> = {};
      const spacing = new Map<string, {before: string; after: string}>();
      pattern.names.forEach((param, i) => {
        const value = match[i + 1];
        params[param] = !verbatim && depth < 2 && !VERBATIM.test(param) && normalize(value) !== name && /\p{L}/u.test(value) ? this.translate(value, depth + 1) : value;
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
    return this.shortcut(core);
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
    else for (const node of parse(value)) if (typeof node !== 'string' && 'plural' in node && node.plural.some(([category]) => !PLURAL_CATEGORIES.has(category) && !/^=\d+$/.test(category))) result.invalid.push(key);
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
  if (!canonical) return undefined;
  const name = typeof meta?.name === 'string' && meta.name.trim() ? meta.name.trim() : canonical;
  return {locale: canonical, name};
}

/**
 * The language to show: the chosen one when available, otherwise ('system')
 * the first preferred system language with a catalog (exact, then by base
 * language), otherwise English.
 */
export function resolveLanguage(choice: string, preferred: readonly string[], available: readonly string[]): string {
  const lower = new Map(available.map(locale => [locale.toLowerCase(), locale]));
  const find = (wanted: string) => {
    const tag = wanted.toLowerCase();
    if (lower.has(tag)) return lower.get(tag)!;
    const base = tag.split('-')[0];
    return lower.get(base) ?? available.find(locale => locale.toLowerCase().split('-')[0] === base);
  };
  if (choice && choice !== 'system') return find(choice) ?? 'en';
  for (const wanted of preferred) {const found = find(wanted);if (found) return found;}
  return 'en';
}

export type Language = CatalogMeta & {catalog: Catalog; source: 'bundled' | 'user'; file?: string; problems?: number};
/**
 * The languages offered: the bundled catalogs plus the catalog files read from
 * the languages folder now. A file for a bundled language replaces it; a file
 * that disappeared or became unreadable leaves no trace of its earlier contents.
 */
export function languageSet(bundled: readonly Language[], files: readonly {file: string; catalog: unknown}[], english: Catalog): {languages: Map<string, Language>; warnings: string[]} {
  const languages = new Map(bundled.map(language => [language.locale, language]));
  const warnings: string[] = [];
  for (const {file, catalog} of files) {
    if (!catalog || typeof catalog !== 'object' || Array.isArray(catalog)) {warnings.push(`${file}: not a catalog`);continue;}
    const meta = catalogMeta(catalog as Catalog, file);
    if (!meta) {warnings.push(`${file}: unknown language`);continue;}
    const problems = validateCatalog(english, catalog as Catalog);
    const count = problems.missing.length + problems.placeholders.length + problems.invalid.length;
    // Keys with mismatched placeholders would show the wrong values; English stands in for them.
    const usable: Catalog = {...catalog as Catalog};
    for (const key of [...problems.placeholders, ...problems.invalid]) delete usable[key];
    languages.set(meta.locale, {...meta, catalog: usable, source: 'user', file, problems: count});
    if (count) warnings.push(`${file}: ${problems.missing.length} missing, ${problems.placeholders.length + problems.invalid.length} invalid`);
  }
  return {languages, warnings};
}
