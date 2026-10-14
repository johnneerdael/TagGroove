export type NumberingOptions = {
  track_start: number; track_digits: number; track_total: number | null;
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
