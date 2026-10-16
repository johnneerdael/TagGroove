// The UI language: bundled catalogs (English, Dutch, German), catalogs people
// add to the configuration folder (languages/<locale>.json), the choice in
// Options → Language and the translation of what is on screen.
import {Translator, catalogMeta, languageSet, resolveLanguage, validateCatalog, type Catalog, type Language} from './core';
import {DomTranslator} from './dom';
import en from './catalogs/en.json';
import nl from './catalogs/nl.json';
import de from './catalogs/de.json';

export {Translator, resolveLanguage, validateCatalog, type Catalog, type Language};

/** The English catalog: every key the UI uses. */
export const ENGLISH = en as Catalog;
const BUNDLED: Language[] = ([en, nl, de] as Catalog[]).map(catalog => ({...catalogMeta(catalog)!, catalog, source: 'bundled'}));
let languages = languageSet(BUNDLED, [], ENGLISH).languages;

let translator = new Translator('en');
/** The catalog the translator was built from, to notice a replaced or removed catalog file. */
let shownCatalog: Catalog | undefined;
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
/** Whether text is being translated: any language but the shipped English, including an English catalog from the languages folder. */
export function translationActive() {return translator.active;}
export function languageChoice() {return choice;}
const systemLanguages = () => typeof navigator === 'undefined' ? [] : navigator.languages?.length ? navigator.languages : [navigator.language];
/** What 'system' resolves to on this computer. */
export function systemLanguage() {return resolveLanguage('system', systemLanguages(), [...languages.keys()]);}

/**
 * The catalog files of the configuration folder as read now: they replace the
 * previous set, so a removed file is no longer offered and a removed override
 * brings the bundled catalog back. Call setLanguage afterwards to apply.
 */
export function setUserCatalogs(files: {file: string; catalog: unknown}[]): string[] {
  const result = languageSet(BUNDLED, files, ENGLISH);
  languages = result.languages;
  return result.warnings;
}

/** Show the language chosen in Options ('system' or a locale). Returns whether it changed. */
export function setLanguage(next: string): boolean {
  choice = next || 'system';
  try {localStorage.setItem(STORAGE_KEY, choice);} catch {/* storage may be unavailable */}
  // Missing explicit locales use English; only system choices match preferred languages.
  const locale = resolveLanguage(choice, systemLanguages(), [...languages.keys()]);
  const catalog = locale === 'en' && languages.get(locale)?.source !== 'user' ? undefined : languages.get(locale)?.catalog;
  if (locale === translator.locale && catalog === shownCatalog) return false;
  shownCatalog = catalog;
  translator = new Translator(locale, catalog ?? {});
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
// `translated` tells a shown translation ("Medium" in German) from English text that happens to be a key too.
(globalThis as {__OPENTAG_I18N__?: unknown}).__OPENTAG_I18N__ = {
  language: () => translator.locale,
  translate: (text: string) => translator.translate(text),
  translated: (text: string) => Object.values(languages.get(translator.locale)?.catalog ?? {}).includes(text),
};

/** Run `listener` after the language changes (window title, native menus). */
export function onLanguageChange(listener: () => void) {listeners.add(listener);return () => listeners.delete(listener);}

/** `key` in the current language: `t('Saved {count} files', {count})`. */
export const t = (key: string, params?: Record<string, unknown>) => translator.t(key, params);
/** English UI text as rendered (an exact key or one with values filled in) in the current language. */
export const translate = (text: string) => translator.translate(text);
/** Translate a subtree right away, before measuring text that was just rendered. */
export const translateNow = (node: Node) => dom?.translateTree(node);
