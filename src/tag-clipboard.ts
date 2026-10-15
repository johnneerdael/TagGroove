type Fields = Record<string, string[]>;
/** One copied cover. Its image bytes stay in the backend, keyed by `digest`. */
export type ClipCover = { digest: string; picture_type: string; description: string; mime: string; size: number; width: number|null; height: number|null };
/** `artwork` is undefined when the file's covers could not be copied; pasting then leaves covers alone. */
export type ClipboardItem = { name: string; fields: Fields; covers: number; artwork?: ClipCover[] };
export type TagClipboard = { items: ClipboardItem[] };
export type PasteTarget = { path: string; fields: Fields };
export type PasteUpdate = { path: string; updates: Fields };
/** The cover list a save writes to a file, replacing its embedded covers. */
export type PendingCover = { digest: string; picture_type: string; description: string };
export type CoverTarget = { path: string; covers: { digest?: string; picture_type: string; description: string }[] };
export type CoverUpdate = { path: string; covers: PendingCover[] };

// Same ASCII-insensitive identity as tag-edits (kept local so this module stands alone).
const fieldNameIdentity = (name: string) => name.replace(/[a-z]/g, letter => letter.toUpperCase());
const present = (fields: Fields) => Object.entries(fields).filter(([, values]) => values.length);

/** Copy each selected file's tags in table order. Removed (empty) fields are not copied. */
export function copyTags(sources: ClipboardItem[]): TagClipboard {
  if (!sources.length) throw new Error('Select files to copy their tags.');
  return { items: sources.map(source => ({ name: source.name, covers: source.covers, ...(source.artwork ? { artwork: source.artwork.map(cover => ({ ...cover })) } : {}), fields: Object.fromEntries(present(source.fields).map(([key, values]) => [key, [...values]])) })) };
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

const pending = (cover: { digest?: string; picture_type: string; description: string }): PendingCover => ({ digest: cover.digest ?? '', picture_type: cover.picture_type, description: cover.description });
const sameCovers = (a: PendingCover[], b: PendingCover[]) => JSON.stringify(a) === JSON.stringify(b);

/** Paste tags also replaces each target's covers with the copied artwork, mapped
 * like the fields (one source for all targets, or one per target). Sources
 * copied without artwork information, and targets that already have the same
 * covers, get no cover update. */
export function pasteCoverPlan(clipboard: TagClipboard|undefined, targets: CoverTarget[]): CoverUpdate[] {
  if (!clipboard?.items.length || !targets.length) return [];
  const count = clipboard.items.length;
  if (count !== 1 && count !== targets.length) return [];
  return targets.flatMap((target, index) => {
    const artwork = clipboard.items[count === 1 ? 0 : index].artwork;
    if (!artwork) return [];
    const covers = artwork.map(pending);
    return sameCovers(covers, target.covers.map(pending)) ? [] : [{ path: target.path, covers }];
  });
}

/** Cut tags removes the covers too. */
export function cutCoverPlan(targets: CoverTarget[]): CoverUpdate[] {
  return targets.filter(target => target.covers.length).map(target => ({ path: target.path, covers: [] }));
}

/** Digests of pasted, unsaved covers: their image data must outlive the next copy. */
export function pendingDigests(lists: Iterable<PendingCover[]>): string[] {
  return [...new Set([...lists].flatMap(list => list.map(cover => cover.digest)))];
}

/** Status text for Copy and Cut tags. */
export function copiedMessage(clipboard: TagClipboard, cut: boolean, failed: number): string {
  const files = clipboard.items.length, covers = clipboard.items.reduce((sum, item) => sum + (item.artwork?.length ?? 0), 0);
  const art = covers ? ` with ${covers} cover${covers === 1 ? '' : 's'}` : '';
  const missing = failed ? ` Artwork of ${failed} file${failed === 1 ? '' : 's'} could not be copied.` : '';
  return `${cut ? 'Cut' : 'Copied'} tags of ${files} file${files === 1 ? '' : 's'}${art}.${missing}`;
}
