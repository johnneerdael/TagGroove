// Options → Language (https://docs.mp3tag.de/customization/options/language/):
// the language of the user interface. OpenTag ships its own catalogs (English,
// Dutch, German); a catalog file in the configuration folder's languages folder
// adds another language or replaces a shipped one. Unlike Mp3tag, the choice
// applies at once.
import type {OptionsPage} from './options-ui';
import {availableLanguages, systemLanguage, type Language} from './i18n';

export const LANGUAGE_DEFAULTS = {language: 'system'};
/** The saved choice: 'system' or a locale such as `nl`. */
export function languageSetting(values: Record<string, unknown>) {return typeof values.language === 'string' && values.language ? values.language : 'system';}

const nameOf = (locale: string, languages: Language[]) => languages.find(language => language.locale === locale)?.name ?? locale;
/** Choices of the language list: the system language first, then every catalog by its own name. */
export function languageChoices(languages: Language[] = availableLanguages(), system = systemLanguage()): [string, string][] {
  return [['system', `System default (${nameOf(system, languages)})`], ...languages.map(language => [language.locale, language.source === 'user' ? `${language.name} (${language.file})` : language.name] as [string, string])];
}

export type LanguageHost = {
  /** Opens the languages folder of the configuration folder, with a current template (desktop only). */
  openFolder?: () => Promise<void>;
};

export function languageOptionsPage(host: LanguageHost): OptionsPage & {refresh: (warnings: string[]) => void} {
  const choices = languageChoices();
  const status = {kind: 'note' as const, text: ''};
  const page = {
    id: 'language', label: 'Language', order: 47, defaults: {...LANGUAGE_DEFAULTS},
    description: 'Choose the language of the user interface. The change applies right away.',
    controls: [
      {kind: 'select' as const, path: 'pages.language.language', label: 'Language', choices},
      {kind: 'note' as const, text: 'Format strings, field names, placeholders and scripting functions stay the same in every language. Names you gave columns, tag panel fields, action groups and tools are shown as you entered them.'},
      {kind: 'group' as const, label: 'Other languages', controls: [
        {kind: 'note' as const, text: 'To add a language, open the languages folder, copy _template.json to a file named after the language (for example fr.json or pt-BR.json), set its name and translate the values. Keep the placeholders in braces as they are. A file named nl.json or de.json replaces the shipped translation. OpenTag reads the folder at startup and when you choose OK.'},
        status,
        ...(host.openFolder ? [{kind: 'button' as const, id: 'open-languages-folder', label: 'Open languages folder', run: () => host.openFolder!()}] : []),
      ]},
    ],
    /** Update the list after reading the languages folder, with problems found in its files. */
    refresh(warnings: string[]) {
      choices.splice(0, choices.length, ...languageChoices());
      const detail = warnings.join('; ');
      status.text = warnings.length ? `Some language files are incomplete; English is shown for what they lack. ${detail}` : '';
    },
  };
  return page;
}
