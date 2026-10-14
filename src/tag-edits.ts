export type Fields = Record<string, string[]>;

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
    if (names.has(row.name)) throw new Error(`Duplicate field: ${row.name}`);
    names.add(row.name);
  }

  const updates: Fields = {};
  for (const row of rows) {
    if (row.changed && row.original && (row.removed || row.original !== row.name)) {
      updates[row.original] = [];
    }
  }
  for (const row of rows) {
    if (!row.changed || row.removed) continue;
    updates[row.name] = [...(row.valueChanged || !row.original ? row.values : source[row.original] || [])];
  }
  return updates;
}
