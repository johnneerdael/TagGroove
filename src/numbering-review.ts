export type NumberingOptions = {
  track_start: number; track_digits: number; track_total: number | null;
  /** Totals are the last track number before the counter restarts (Ctrl+Shift+K: per folder). */
  count_track_total?: boolean;
  disc_enabled: boolean; disc_start: number; disc_digits: number; disc_total: number | null;
  next_disc_on_directory: boolean; next_disc_on_field: string | null;
  reset_track_on_next_disc: boolean; reset_disc_on_field: string | null;
};
export type NumberingUpdate = {path: string; fields: Record<string, string[]>};
export type NumberingRequest = {items: NumberingUpdate[]; options: NumberingOptions};

export function numberingCounter(value: string, label: string, max = 4_294_967_295): number {
  const number = Number(value);
  if (!value.trim() || !Number.isInteger(number) || number < 1 || number > max) {
    throw new Error(`${label}: enter a whole number from 1 to ${max}.`);
  }
  return number;
}

/** Keep a pure numbering preview tied to the selected rows and options. */
export function createNumberingReview() {
  type Attempt = {request: NumberingRequest};
  let pending: Attempt | undefined;
  let reviewed: {request: NumberingRequest; updates: NumberingUpdate[]} | undefined;
  const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value));
  const same = (a: NumberingRequest, b: NumberingRequest) => JSON.stringify(a) === JSON.stringify(b);
  const invalidate = () => { pending = undefined; reviewed = undefined; };
  return {
    invalidate,
    begin(request: NumberingRequest) {
      invalidate();
      pending = {request: copy(request)};
      return pending;
    },
    complete(attempt: Attempt, updates: NumberingUpdate[], current: NumberingRequest) {
      if (pending !== attempt || !same(attempt.request, current)) return false;
      invalidate();
      const names = attempt.request.options.disc_enabled ? ['TRACK', 'DISCNUMBER'] : ['TRACK'];
      if (!Array.isArray(updates) || updates.length !== current.items.length || updates.some((row, index) =>
        row?.path !== current.items[index].path || !row.fields || Object.keys(row.fields).length !== names.length ||
        names.some(name => !Array.isArray(row.fields[name]) || row.fields[name].length !== 1 ||
          typeof row.fields[name][0] !== 'string' || !/^\d+(?:\/\d+)?$/.test(row.fields[name][0])))) {
        throw new Error('The numbering preview does not match the selection. Run Preview again.');
      }
      reviewed = {request: copy(attempt.request), updates: copy(updates)};
      return true;
    },
    canApply(current: NumberingRequest) { return !!reviewed && same(reviewed.request, current); },
    take(current: NumberingRequest): NumberingUpdate[] {
      if (!reviewed || !same(reviewed.request, current)) {
        invalidate();
        throw new Error('The numbering inputs or selection changed. Run Preview again.');
      }
      const updates = copy(reviewed.updates);
      invalidate();
      return updates;
    },
  };
}

/** Wizard options remembered between runs. Total tracks is not: it starts at the number of selected files. */
export type NumberingMemory = {
  track_start: number; track_digits: number; track_total: boolean;
  disc_enabled: boolean; disc_start: number; disc_digits: number; disc_total: boolean; disc_total_value: number;
  next_disc_on_directory: boolean; next_disc_on_field: string; reset_track_on_next_disc: boolean; reset_disc_on_field: string;
};
type MemoryStorage = {getItem: (key: string) => string | null; setItem: (key: string, value: string) => void};
const NUMBERING_KEY = 'opentag.numbering';
/** The options Ctrl+Shift+K sets for one run; they are not remembered as Ctrl+K's options. */
const PER_FOLDER: Partial<NumberingMemory> = {next_disc_on_directory: true, reset_track_on_next_disc: true, disc_enabled: false};

export function numberingDefaults(): NumberingMemory {
  return {
    track_start: 1, track_digits: 2, track_total: false,
    disc_enabled: false, disc_start: 1, disc_digits: 1, disc_total: false, disc_total_value: 1,
    next_disc_on_directory: false, next_disc_on_field: '', reset_track_on_next_disc: false, reset_disc_on_field: '',
  };
}

function stored(storage: MemoryStorage | undefined): Partial<NumberingMemory> {
  try {
    const value = JSON.parse(storage?.getItem(NUMBERING_KEY) || 'null');
    return value && typeof value === 'object' ? value : {};
  } catch { return {}; }
}

/** Remembered options, each falling back to its default when missing or invalid; `perFolder` applies Ctrl+Shift+K's options on top. */
export function recallNumbering(storage: MemoryStorage | undefined, perFolder = false): NumberingMemory {
  const defaults = numberingDefaults(), saved = stored(storage) as Record<string, unknown>;
  const result = {...defaults} as Record<string, unknown>;
  for (const [key, fallback] of Object.entries(defaults)) {
    const value = saved[key];
    if (typeof fallback === 'number' ? Number.isInteger(value) && (value as number) >= 1 && (value as number) <= (key.endsWith('digits') ? 10 : 4_294_967_295) : typeof value === typeof fallback) result[key] = value;
  }
  return {...result as NumberingMemory, ...(perFolder ? PER_FOLDER : {})};
}

/** Remember the options of a staged run. A Ctrl+Shift+K run keeps the previously remembered values of the options it presets. */
export function rememberNumbering(storage: MemoryStorage | undefined, options: NumberingMemory, perFolder = false) {
  if (!storage) return;
  const previous = recallNumbering(storage), next = {...options};
  if (perFolder) for (const key of Object.keys(PER_FOLDER) as (keyof NumberingMemory)[]) (next as Record<string, unknown>)[key] = previous[key];
  try { storage.setItem(NUMBERING_KEY, JSON.stringify(next)); } catch { /* Preferences are optional. */ }
}
