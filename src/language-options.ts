// Options → Language (https://docs.mp3tag.de/customization/options/language/):
// the language of the user interface. TagGroove ships its own catalogs (English,
// Dutch, German); a catalog file in the configuration folder's languages folder
// adds another language or replaces a shipped one. Unlike Mp3tag, the choice
// applies at once.
import type {OptionControl, OptionsPage, SelectChoice} from './options-ui';
import {availableLanguages, systemLanguage, type Language, type LanguageWarning} from './i18n';

export const LANGUAGE_DEFAULTS = {language: 'system'};
/** The saved choice: 'system' or a locale such as `nl`. */
export function languageSetting(values: Record<string, unknown>) {return typeof values.language === 'string' && values.language ? values.language : 'system';}

const nameOf = (locale: string, languages: Language[]) => languages.find(language => language.locale === locale)?.name ?? locale;
/** Choices of the language list: the system language first, then every catalog by its own name. */
export function languageChoices(languages: Language[] = availableLanguages(), system = systemLanguage()): SelectChoice[] {
  return [['system', `System default (${nameOf(system, languages)})`], ...languages.map((language):SelectChoice => language.source === 'user' ? [language.locale, `${language.name} (${language.file})`, true] : [language.locale, language.name])];
}

/** File-read reasons from the backend retain their own filename field. */
export type CatalogReadError = {file:string;error:string;kind:'too_large'|'not_object'|'external'};
export function languageReadWarnings(errors: CatalogReadError[]):LanguageWarning[] {
  return errors.map(({file,error,kind}) => kind === 'external' ? {file,error} : {key:kind === 'too_large' ? '{file}: the file is too large' : '{file}: not a JSON object',params:{file}});
}
/** Explicit parameters protect filenames and external parser/I/O reasons. */
export function languageWarningText(warning:LanguageWarning,t:(key:string,params?:Record<string,unknown>)=>string = (key,params) => key.replace(/\{(\w+)\}/g,(_,name)=>String(params?.[name] ?? `{${name}}`))):string {
  return 'key' in warning ? t(warning.key,warning.params) : 'file' in warning ? `${warning.file}: ${warning.error}` : warning.message;
}
const warningHTML = (text:string) => text.replace(/[&<>"']/g,char=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]!));

export type LanguageHost = {
  warningText?: (warning:LanguageWarning) => string;
  onLanguageChange?: (listener:()=>void) => ()=>void;
  /** Opens the languages folder of the configuration folder, with a current template (desktop only). */
  openFolder?: () => Promise<void>;
  /** Initial folder-opening capability; false until the native platform is known. */
  folderSupported?: boolean;
};

export function languageOptionsPage(host: LanguageHost): OptionsPage & {refresh: (warnings: readonly LanguageWarning[]) => void; setFolderSupport: (supported: boolean) => void} {
  const choices = languageChoices();
  let warnings: readonly LanguageWarning[] = [], shown:HTMLElement|undefined, stopLanguage:(()=>void)|undefined;
  const renderWarnings = () => warnings.length ? `<p>Some language files are incomplete; English is shown for what they lack.</p>${warnings.map(warning => `<p data-language-warning translate="no">${warningHTML((host.warningText ?? languageWarningText)(warning))}</p>`).join('')}` : '';
  const refreshShown = () => { if(shown?.isConnected) shown.innerHTML = renderWarnings(); };
  const status:OptionControl = {kind:'custom',id:'language-warnings',paths:[],render:renderWarnings,bind(root) {
    stopLanguage?.();shown = root;stopLanguage = host.onLanguageChange?.(refreshShown);
    root.closest('dialog')?.addEventListener('close',() => {if(shown === root){stopLanguage?.();stopLanguage = undefined;shown = undefined;}},{once:true});
  }};
  const folderButton:OptionControl|undefined=host.openFolder?{kind:'button',id:'open-languages-folder',label:'Open languages folder',run:()=>host.openFolder!()}:undefined;
  const otherLanguages:OptionControl[]=[
    {kind: 'note', text: 'To add a language, open the languages folder, copy _template.json to a file named after the language (for example fr.json or pt-BR.json), set its name and translate the values. Keep the placeholders in braces as they are. A file named nl.json or de.json replaces the shipped translation. TagGroove reads the folder at startup and when you choose OK.'},
    status,
  ];
  const setFolderSupport=(supported:boolean)=>{otherLanguages.splice(2,otherLanguages.length-2,...(supported&&folderButton?[folderButton]:[]));};
  setFolderSupport(host.folderSupported!==false);
  const page = {
    id: 'language', label: 'Language', order: 47, defaults: {...LANGUAGE_DEFAULTS},
    description: 'Choose the language of the user interface. The change applies right away.',
    controls: [
      {kind: 'select' as const, path: 'pages.language.language', label: 'Language', choices,
        missingChoiceLabel: (name:string) => `Unavailable: ${name} (using English)`,
        matchesChoice: (stored:unknown,value:string) => stored === value || value !== 'system' && typeof stored === 'string' && stored.toLowerCase() === value.toLowerCase()},
      {kind: 'note' as const, text: 'Format strings, field names, placeholders and scripting functions stay the same in every language. Names you gave columns, tag panel fields, action groups and tools are shown as you entered them.'},
      {kind: 'group' as const, label: 'Other languages', controls: otherLanguages},
    ],
    setFolderSupport,
    /** Update the list after reading the languages folder, with problems found in its files. */
    refresh(next: readonly LanguageWarning[]) {
      choices.splice(0, choices.length, ...languageChoices());
      warnings = [...next];refreshShown();
    },
  };
  return page;
}
