export type TagTagPreview = { path: string; value: string };
export type TagTagRequest = { field: string; template: string; paths: string[] };
export type TagTagDeps = {
  modal: (title: string, content: string, buttons: string) => HTMLDialogElement;
  modalError: (dialog: HTMLDialogElement, error: unknown) => void;
  esc: (value: unknown) => string;
  paths: () => string[];
  fieldNames: () => string[];
  displayName: (path: string) => string;
  canApply: () => boolean;
  /** `filename` evaluates as Tag → Filename does, for _FILENAME and _DIRECTORY. */
  preview: (paths: string[], template: string, filename: boolean) => Promise<TagTagPreview[]>;
  apply: (request: TagTagRequest) => Promise<void>;
  storage?: { getItem(key: string): string|null; setItem(key: string, value: string): void };
};

export const tagTagCommonFields = ['TITLE','ARTIST','ALBUM','ALBUMARTIST','TRACK','DISCNUMBER','YEAR','GENRE','COMMENT','COMPOSER','_FILENAME','_DIRECTORY'];
const identity = (name: string) => name.replace(/[a-z]/g, letter => letter.toUpperCase());

/** Common fields first, then fields present in the selection, without case duplicates. */
export function tagTagFieldOptions(present: string[]) {
  const seen = new Set(tagTagCommonFields.map(identity));
  const extra = [...new Set(present.filter(name => name && !name.startsWith('_')))].filter(name => !seen.has(identity(name)) && seen.add(identity(name))).sort();
  return [...tagTagCommonFields, ...extra];
}

/** Validate a Format Value target: a tag field, or _FILENAME/_DIRECTORY to rename. */
export function validateTagTarget(field: string) {
  const name = field.trim();
  if (!name) throw new Error('Enter a field name.');
  if (/[=\0\r\n]/.test(name)) throw new Error('Field names cannot contain =, line breaks or NUL characters.');
  if (name.startsWith('_') && !['_FILENAME','_DIRECTORY'].includes(identity(name))) throw new Error('Only _FILENAME and _DIRECTORY can be used as special target fields.');
  return name.startsWith('_') ? identity(name) : name;
}

/** Describe one preview row the way the converter will apply it. */
export function tagTagOutcome(field: string, value: string, currentName = '') {
  const target = identity(field);
  if (target === '_FILENAME') {
    if (!value.trim()) return { label: 'New filename', value: 'Error: the filename is empty', removed: true };
    // Like Tag → Filename, a format string that already spells the extension keeps one.
    const extension = /\.[^./\\]+$/.exec(currentName)?.[0] ?? '';
    return { label: 'New filename', value: extension && !value.toLowerCase().endsWith(extension.toLowerCase()) ? value + extension : value, removed: false };
  }
  if (target === '_DIRECTORY') return value ? { label: 'New folder name', value, removed: false } : { label: 'New folder name', value: 'Error: the folder name is empty', removed: true };
  // Match Format Value's str::trim only for delimited metadata output.
  // Unicode White_Space also keeps the Rust/JS differences (NEL and BOM) aligned.
  const values = value.includes('\\\\')
    ? value.split('\\\\').map(part => part.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '')).filter(Boolean)
    : value ? [value] : [];
  return values.length ? { label: field, value: values.join(' \\\\ '), removed: false } : { label: field, value: 'Field will be removed', removed: true };
}

export const tagTagKey = (request: TagTagRequest) => JSON.stringify([request.field, request.template, request.paths]);
const storageKey = 'opentag.tag-tag';

/** Convert → Tag - Tag. Previews update as the inputs change; Apply writes the
 * reviewed field and format to the same files immediately. */
export function openTagToTag(deps: TagTagDeps) {
  let remembered = { field: 'ALBUMARTIST', template: '%artist%' };
  try { remembered = { ...remembered, ...JSON.parse(deps.storage?.getItem(storageKey) || '{}') }; } catch { /* Last values are a convenience only. */ }
  const { esc } = deps, count = deps.paths().length;
  const dialog = deps.modal('Tag → Tag', `<p>Set a field of each selected file from a format string using %field% placeholders and scripting functions. _FILENAME renames files; _DIRECTORY renames their folder. Changes are written to the files immediately and can be undone.</p>
    <label class="field"><span>FIELD</span><input id="tag-tag-field" value="${esc(remembered.field)}" autocomplete="off" spellcheck="false" data-helper="field" data-specials="_FILENAME,_DIRECTORY"></label>
    <label class="field"><span>FORMAT STRING</span><input id="tag-tag-template" value="${esc(remembered.template)}" autocomplete="off" spellcheck="false" data-helper="format" data-history="tag-tag" data-history-manage></label>
    <p class="converter-status" role="status" aria-live="polite">Preparing preview…</p><div id="tag-tag-preview" aria-label="Tag → Tag preview"></div>`,
  `<button class="primary" id="tag-tag-apply" disabled>Apply to ${count} file${count === 1 ? '' : 's'}</button>`);
  dialog.setAttribute('aria-label', 'Tag → Tag');
  const field = dialog.querySelector<HTMLInputElement>('#tag-tag-field')!, template = dialog.querySelector<HTMLInputElement>('#tag-tag-template')!;
  const apply = dialog.querySelector<HTMLButtonElement>('#tag-tag-apply')!, progress = dialog.querySelector<HTMLElement>('.converter-status')!, area = dialog.querySelector<HTMLElement>('#tag-tag-preview')!;
  let reviewed = '', generation = 0, timer = 0, applying = false;
  const request = (): TagTagRequest => ({ field: validateTagTarget(field.value), template: template.value, paths: deps.paths() });
  const sync = () => { let current = ''; try { current = tagTagKey(request()); } catch { /* Invalid input keeps Apply disabled. */ } apply.disabled = applying || !current || current !== reviewed || !deps.canApply(); };
  const refresh = async () => {
    const current = ++generation; reviewed = ''; sync(); dialog.querySelector('.dialog-error')?.remove();
    let input: TagTagRequest;
    try { input = request(); if (!input.paths.length) throw new Error('Select files to convert.'); }
    catch (error) { area.replaceChildren(); progress.textContent = 'Complete the field and format string.'; deps.modalError(dialog, error); return; }
    progress.textContent = 'Preparing preview…';
    try {
      const rows = await deps.preview(input.paths, input.template, input.field.startsWith('_'));
      if (!dialog.open || current !== generation) return;
      const outcomes = rows.map(row => ({ row, outcome: tagTagOutcome(input.field, row.value, deps.displayName(row.path)) }));
      area.innerHTML = `<div class="preview" tabindex="0" role="region" aria-label="Tag → Tag results">${outcomes.slice(0, 100).map(({ row, outcome }) => `<div><small translate="no">${esc(deps.displayName(row.path))}</small><span ${input.field.startsWith('_')?'':'translate="no"'}>${esc(outcome.label)}</span>: ${outcome.removed ? `<em>${esc(outcome.value)}</em>` : `<span translate="no">${esc(outcome.value)}</span>`}</div>`).join('')}</div>`;
      const invalid = outcomes.some(({ outcome }) => outcome.removed && input.field.startsWith('_'));
      progress.textContent = invalid ? 'Some files would get an empty name. Change the format string.' : `Previewed ${rows.length} file${rows.length === 1 ? '' : 's'}${rows.length > 100 ? '; showing the first 100' : ''}.`;
      reviewed = invalid ? '' : tagTagKey(input); sync();
    } catch (error) { if (dialog.open && current === generation) { area.replaceChildren(); progress.textContent = 'Preview failed. Review the format string.'; deps.modalError(dialog, error); } }
  };
  const schedule = () => { reviewed = ''; sync(); window.clearTimeout(timer); timer = window.setTimeout(() => void refresh(), 250); };
  field.addEventListener('input', schedule); template.addEventListener('input', schedule);
  dialog.addEventListener('close', () => { generation++; window.clearTimeout(timer); });
  dialog.addEventListener('cancel', event => { if (applying) event.preventDefault(); });
  apply.addEventListener('click', async () => {
    let input: TagTagRequest;
    try { input = request(); if (tagTagKey(input) !== reviewed) throw new Error('The preview is out of date. Wait for it to refresh.'); }
    catch (error) { deps.modalError(dialog, error); return; }
    try { deps.storage?.setItem(storageKey, JSON.stringify({ field: input.field, template: input.template })); } catch { /* Remembering inputs is optional. */ }
    applying = true; sync(); progress.textContent = 'Writing tags…';
    try { await deps.apply(input); dialog.close(); }
    catch (error) { applying = false; reviewed = ''; sync(); progress.textContent = 'Apply did not complete. Review the errors.'; deps.modalError(dialog, error); }
  });
  void refresh(); template.focus({ preventScroll: true }); template.select();
}
