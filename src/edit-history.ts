export type HistoryStep = { label: string; files: number };
export type HistoryState = { undo: HistoryStep|null; redo: HistoryStep|null; notice: string|null };
export const emptyHistory: HistoryState = { undo: null, redo: null, notice: null };

/** Edit menu text, e.g. "Undo Save tags (3 files)". */
export function historyLabel(kind: 'Undo'|'Redo', step: HistoryStep|null|undefined) {
  return step ? `${kind} ${step.label} (${step.files} file${step.files === 1 ? '' : 's'})` : kind;
}

export type EditShortcut = 'undo'|'redo'|'copy-tags'|'cut-tags'|'paste-tags'|'tags-filename'|'filename-tags'|'filename-filename'|'text-tags'|'tag-tag';
type Key = { key: string; code?: string; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean };
const converters: EditShortcut[] = ['tags-filename','filename-tags','filename-filename','text-tags','tag-tag'];

/** Mp3tag Edit and Convert shortcuts. Text fields keep native undo and clipboard
 * behaviour, as does text selected on the page; on macOS, Option+digit keeps
 * typing its character while a text field has focus. */
export function editShortcut(event: Key, context: { typing: boolean; textSelected: boolean; mac: boolean }): EditShortcut|undefined {
  const digit = /^Digit([1-5])$/.exec(event.code || '')?.[1] ?? (/^[1-5]$/.test(event.key) ? event.key : undefined);
  if (event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey && digit && !(context.typing && context.mac)) return converters[Number(digit) - 1];
  if (!(event.ctrlKey || event.metaKey) || event.altKey || context.typing) return;
  const key = event.key.toLowerCase();
  if (key === 'z') return event.shiftKey ? 'redo' : 'undo';
  if (key === 'y' && !event.shiftKey) return 'redo';
  if (event.shiftKey || context.textSelected) return;
  return key === 'c' ? 'copy-tags' : key === 'x' ? 'cut-tags' : key === 'v' ? 'paste-tags' : undefined;
}
