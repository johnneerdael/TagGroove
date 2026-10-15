// Native dialogs in the UI language: titles, messages, buttons and file type
// names are translated at this boundary, so callers keep passing English.
import * as dialog from '@tauri-apps/plugin-dialog';
import {currentLanguage, t, translate} from './index';

type Filters = {filters?: {name: string; extensions: string[]}[]};
const filters = <T extends Filters>(options: T): T => options.filters ? {...options, filters: options.filters.map(filter => ({...filter, name: translate(filter.name)}))} : options;
const titled = <T extends {title?: string}>(options: T): T => options.title ? {...options, title: translate(options.title)} : options;
const english = () => currentLanguage() === 'en';

export function open<T extends dialog.OpenDialogOptions>(options?: T) {
  return dialog.open<T>(options && !english() ? filters(titled(options)) : options);
}
export function save(options?: dialog.SaveDialogOptions) {
  return dialog.save(options && !english() ? filters(titled(options)) : options);
}
type Ask = string | dialog.ConfirmDialogOptions;
const options = (value: Ask | undefined): dialog.ConfirmDialogOptions => typeof value === 'string' ? {title: value} : value ?? {};
/** Yes/No question; the system's own button labels are English on some platforms. */
export function ask(message: string, value?: Ask) {
  if (english()) return dialog.ask(message, value);
  const opts = titled(options(value));
  return dialog.ask(translate(message), {...opts, okLabel: translate(opts.okLabel ?? '') || t('Yes'), cancelLabel: translate(opts.cancelLabel ?? '') || t('No')});
}
export function confirm(message: string, value?: Ask) {
  if (english()) return dialog.confirm(message, value);
  const opts = titled(options(value));
  return dialog.confirm(translate(message), {...opts, okLabel: translate(opts.okLabel ?? '') || t('OK'), cancelLabel: translate(opts.cancelLabel ?? '') || t('Cancel')});
}
export function message(text: string, value?: string | dialog.MessageDialogOptions) {
  if (english()) return dialog.message(text, value);
  const opts: dialog.MessageDialogOptions = titled(typeof value === 'string' ? {title: value} : value ?? {});
  return dialog.message(translate(text), opts.buttons || opts.okLabel ? opts : {...opts, buttons: {ok: t('OK')}});
}
