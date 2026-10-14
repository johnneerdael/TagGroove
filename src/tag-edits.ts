export type Fields = Record<string, string[]>;

// Match the Rust backend's ASCII-insensitive field identity without folding ß into SS.
export const fieldNameIdentity = (name: string) => name.replace(/[a-z]/g, letter => letter.toUpperCase());
function indexFields(source: Fields) {
  const index = new Map<string, { name: string; values: string[] }>();
  for (const [name, values] of Object.entries(source)) {
    const previous = index.get(fieldNameIdentity(name));
    if (previous) previous.values = [...previous.values, ...values];
    else index.set(fieldNameIdentity(name), { name, values });
  }
  return index;
}

/** One dialog row per logical field; native spelling stays in each file. */
export function selectionTagRows(sources: Fields[]) {
  const files = sources.map(indexFields);
  const names = [...new Set(files.flatMap(fields => [...fields.keys()]))].sort();
  return names.map(name => {
    const values = files.map(fields => fields.get(name)?.values ?? []);
    const mixed = values.some(value => JSON.stringify(value) !== JSON.stringify(values[0]));
    return { name, mixed, values: mixed ? [] : [...values[0]] };
  });
}

export type TagRowEdit = {
  original: string;
  name: string;
  values: string[];
  changed: boolean;
  valueChanged: boolean;
  removed: boolean;
};

/** Resolve every row from one file's original snapshot before applying edits.
 * Name-only edits retain that file's values, including mixed selections.
 */
export function resolveTagEdits(source: Fields, rows: TagRowEdit[]): Fields {
  const names = new Set<string>();
  for (const row of rows) {
    if (row.removed) continue;
    if (!row.name) throw new Error('Enter a field name for each row.');
    if (names.has(fieldNameIdentity(row.name))) throw new Error(`Duplicate field: ${row.name}`);
    names.add(fieldNameIdentity(row.name));
  }

  const updates: Fields = {};
  const fields = indexFields(source);
  const nativeKey = (name: string) => fields.get(fieldNameIdentity(name))?.name ?? name;
  for (const row of rows) {
    if (row.changed && row.original && (row.removed || fieldNameIdentity(row.original) !== fieldNameIdentity(row.name))) {
      updates[nativeKey(row.original)] = [];
    }
  }
  for (const row of rows) {
    if (!row.changed || row.removed) continue;
    updates[nativeKey(row.name)] = [...(row.valueChanged || !row.original ? row.values : fields.get(fieldNameIdentity(row.original))?.values ?? [])];
  }
  return updates;
}
