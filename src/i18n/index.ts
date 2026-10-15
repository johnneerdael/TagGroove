// The UI language: bundled catalogs (English, Dutch, German), catalogs people
// add to the configuration folder (languages/<locale>.json), the choice in
// Options → Language and the translation of what is on screen.
import {Translator, catalogMeta, resolveLanguage, validateCatalog, type Catalog, type CatalogMeta} from './core';
import {DomTranslator} from './dom';
import en from './catalogs/en.json';
import nl from './catalogs/nl.json';
import de from './catalogs/de.json';

export {Translator, resolveLanguage, validateCatalog, type Catalog};

export type Language = CatalogMeta & {catalog: Catalog; source: 'bundled' | 'user'; file?: string; problems?: number};
/** The English catalog: every key the UI uses. */
export const ENGLISH = en as Catalog;
const languages = new Map<string, Language>();
for (const catalog of [en, nl, de] as Catalog[]) {
  const meta = catalogMeta(catalog)!;
  languages.set(meta.locale, {...meta, catalog, source: 'bundled'});
}

let translator = new Translator('en');
let dom: DomTranslator | undefined;
let choice = 'system';
const listeners = new Set<() => void>();
// Remembered on this device so the first paint already uses the language.
const STORAGE_KEY = 'opentag.language';

/** Languages to choose from, English first and the others by name. */
export function availableLanguages(): Language[] {
  return [...languages.values()].sort((a, b) => a.locale === 'en' ? -1 : b.locale === 'en' ? 1 : a.name.localeCompare(b.name));
}
/** The language shown now (a locale such as `nl`). */
export function currentLanguage() {return translator.locale;}
export function languageChoice() {return choice;}
const systemLanguages = () => typeof navigator === 'undefined' ? [] : navigator.languages?.length ? navigator.languages : [navigator.language];
/** What 'system' resolves to on this computer. */
export function systemLanguage() {return resolveLanguage('system', systemLanguages(), [...languages.keys()]);}

/** Catalog files from the configuration folder; a file for a bundled language replaces it. */
export function addUserCatalogs(files: {file: string; catalog: unknown}[]): string[] {
  const warnings: string[] = [];
  for (const {file, catalog} of files) {
    if (!catalog || typeof catalog !== 'object' || Array.isArray(catalog)) {warnings.push(`${file}: not a catalog`);continue;}
    const meta = catalogMeta(catalog as Catalog, file);
    if (!meta) {warnings.push(`${file}: unknown language`);continue;}
    const problems = validateCatalog(ENGLISH, catalog as Catalog);
    const count = problems.missing.length + problems.placeholders.length + problems.invalid.length;
    // Keys with mismatched placeholders would show the wrong values; English stands in for them.
    const usable: Catalog = {...catalog as Catalog};
    for (const key of [...problems.placeholders, ...problems.invalid]) delete usable[key];
    languages.set(meta.locale, {...meta, catalog: usable, source: 'user', file, problems: count});
    if (count) warnings.push(`${file}: ${problems.missing.length} missing, ${problems.placeholders.length + problems.invalid.length} invalid`);
  }
  return warnings;
}

/** Show the language chosen in Options ('system' or a locale). Returns whether it changed. */
export function setLanguage(next: string): boolean {
  choice = next || 'system';
  try {localStorage.setItem(STORAGE_KEY, choice);} catch {/* storage may be unavailable */}
  const locale = resolveLanguage(choice, systemLanguages(), [...languages.keys()]);
  const language = languages.get(locale);
  if (locale === translator.locale && (!language || language.source === 'bundled')) return false;
  translator = new Translator(locale, locale === 'en' && language?.source !== 'user' ? {} : language?.catalog ?? {});
  document.documentElement.lang = locale;
  dom?.apply();
  listeners.forEach(listener => listener());
  return true;
}

/** Translate the document from now on, starting with the language remembered on this device. */
export function installTranslation(root: HTMLElement = document.body) {
  let remembered = 'system';
  try {remembered = localStorage.getItem(STORAGE_KEY) || 'system';} catch {/* storage may be unavailable */}
  dom = new DomTranslator(root, () => translator);
  setLanguage(remembered);
  dom.apply();
}

// Native UI suites (tests/e2e/i18n.js) check the shown language and look for text left untranslated.
(globalThis as {__OPENTAG_I18N__?: unknown}).__OPENTAG_I18N__ = {language: () => translator.locale, translate: (text: string) => translator.translate(text)};

/** Run `listener` after the language changes (window title, native menus). */
export function onLanguageChange(listener: () => void) {listeners.add(listener);return () => listeners.delete(listener);}

/** `key` in the current language: `t('Saved {count} files', {count})`. */
export const t = (key: string, params?: Record<string, unknown>) => translator.t(key, params);
/** English UI text as rendered (an exact key or one with values filled in) in the current language. */
export const translate = (text: string) => translator.translate(text);
/** Translate a subtree right away, before measuring text that was just rendered. */
export const translateNow = (node: Node) => dom?.translateTree(node);
