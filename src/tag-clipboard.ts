type Fields = Record<string, string[]>;
export type ClipboardItem = { name: string; fields: Fields; covers: number };
export type TagClipboard = { items: ClipboardItem[] };
export type PasteTarget = { path: string; fields: Fields };
export type PasteUpdate = { path: string; updates: Fields };

// Same ASCII-insensitive identity as tag-edits (kept local so this module stands alone).
const fieldNameIdentity = (name: string) => name.replace(/[a-z]/g, letter => letter.toUpperCase());
const present = (fields: Fields) => Object.entries(fields).filter(([, values]) => values.length);

/** Copy each selected file's tags in table order. Removed (empty) fields are not copied. */
export function copyTags(sources: ClipboardItem[]): TagClipboard {
  if (!sources.length) throw new Error('Select files to copy their tags.');
  return { items: sources.map(source => ({ name: source.name, covers: source.covers, fields: Object.fromEntries(present(source.fields).map(([key, values]) => [key, [...values]])) })) };
}

/** Plain-text form for the system clipboard: one line per file. */
export function clipboardText(clipboard: TagClipboard) {
  const flat = (value: string) => value.replace(/\r\n|[\r\n\t]/g, ' ');
  return clipboard.items.map(item => [flat(item.name), ...present(item.fields).sort(([a], [b]) => a.localeCompare(b)).map(([key, values]) => `${flat(key)}=${values.map(flat).join(' \\\\ ')}`)].join('\t')).join('\n');
}

/** Replace every target's tags with the copied ones: one source for all targets,
 * or one source per target in table order. Keeps each target's native spelling
 * of existing field names and omits fields that already match. */
export function pastePlan(clipboard: TagClipboard|undefined, targets: PasteTarget[]): PasteUpdate[] {
  if (!clipboard?.items.length) throw new Error('Copy tags first.');
  if (!targets.length) throw new Error('Select files to paste tags to.');
  const count = clipboard.items.length;
  if (count !== 1 && count !== targets.length) throw new Error(`The clipboard holds tags of ${count} files. Select 1 or ${count} files to paste them.`);
  return targets.map((target, index) => {
    const source = clipboard.items[count === 1 ? 0 : index].fields, updates: Fields = {};
    const native = new Map(present(target.fields).map(([key]) => [fieldNameIdentity(key), key]));
    const copied = new Set(Object.keys(source).map(fieldNameIdentity));
    for (const [identity, key] of native) if (!copied.has(identity)) updates[key] = [];
    for (const [key, values] of Object.entries(source)) {
      const name = native.get(fieldNameIdentity(key)) ?? key;
      if (JSON.stringify(target.fields[name] ?? []) !== JSON.stringify(values)) updates[name] = [...values];
    }
    return { path: target.path, updates };
  }).filter(update => Object.keys(update.updates).length);
}

/** Cut stages removal of every present field. */
export function cutPlan(targets: PasteTarget[]): PasteUpdate[] {
  return targets.map(target => ({ path: target.path, updates: Object.fromEntries(present(target.fields).map(([key]) => [key, [] as string[]])) })).filter(update => Object.keys(update.updates).length);
}
