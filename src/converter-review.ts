type Fields = Record<string, string[]>;
type Column = {name: string; template: string; values?: Record<string, string>};
export type FilenameConversionRequest = {
  paths: string[];
  old_pattern: string;
  new_pattern: string;
  columns?: Column[] | null;
};
export type TextImportRequest = {
  paths: string[];
  source: {kind: 'file'; path: string} | {kind: 'text'; text: string};
  mask: string;
  options: {ansi_codepage: string | null; case_sensitive_paths: boolean};
};
export type FilenameConversionItem = {path: string; target: string; captures: Fields; associated?: {from:string;to:string}[]};
export type TextImportItem = {path: string; selection_index: number; source_line: number; fields: Fields};
/**
 * `source_path` is the text file read, after evaluating a format-string source.
 * `unmatched` lists the selected files left unchanged: names that do not match
 * the old filename pattern, or files no keyed text record names.
 */
export type ConverterPreview<Item> = {plan_token: string; items: Item[]; source_path?: string | null; unmatched?: string[]};
type Attempt<Request> = {revision: number; request: Request; context: string};
type Receipt<Request, Item> = {request: Request; planToken: string; items: Item[]; unmatched: string[]; context: string; reviewedAt: number};
const snapshot = <T>(value: T): T => JSON.parse(JSON.stringify(value));

/** Native blur/change may repeat values already reported by input or capture. */
export function createConverterInputTracker(initial: readonly (string | boolean)[]) {
  let previous = JSON.stringify(initial);
  return (current: readonly (string | boolean)[]) => {
    const next = JSON.stringify(current);
    if (next === previous) return false;
    previous = next;
    return true;
  };
}

/** A late response can never authorize a different request or workspace state. */
export function createConverterReview<Request, Item>(
  validate: (request: Request, items: Item[], unmatched: string[]) => void,
  now: () => number = Date.now,
) {
  let revision = 0;
  let pending: Attempt<Request> | undefined;
  let receipt: Receipt<Request, Item> | undefined;
  const same = (request: Request, context: string, reviewed: {request: Request; context: string}) =>
    context === reviewed.context && JSON.stringify(request) === JSON.stringify(reviewed.request);
  const invalidate = () => { revision++; pending = undefined; receipt = undefined; };
  return {
    invalidate,
    get pending() { return !!pending; },
    /** Files the reviewed plan changes; undefined before a preview. */
    get planned() { return receipt?.items.length; },
    begin(request: Request, context: string) {
      invalidate();
      pending = {revision, request: snapshot(request), context};
      return pending;
    },
    complete(attempt: Attempt<Request>, preview: ConverterPreview<Item>, request: Request, context: string) {
      if (pending !== attempt || attempt.revision !== revision || !same(request, context, attempt)) return false;
      const unmatched = preview.unmatched ?? [];
      if (typeof preview.plan_token !== 'string' || !preview.plan_token.trim() || !Array.isArray(preview.items) ||
          !Array.isArray(unmatched) || unmatched.some(path => typeof path !== 'string')) {
        throw new Error('The converter preview could not be verified. Run Preview again.');
      }
      validate(attempt.request, preview.items, unmatched);
      receipt = {request: snapshot(attempt.request), planToken: preview.plan_token, items: snapshot(preview.items), unmatched: [...unmatched], context, reviewedAt: now()};
      pending = undefined;
      return true;
    },
    fail(attempt: Attempt<Request>) {
      if (pending !== attempt || attempt.revision !== revision) return false;
      invalidate();
      return true;
    },
    canApply(request: Request, context: string) {
      return !!receipt && !pending && receipt.items.length > 0 && now() - receipt.reviewedAt < 300_000 && same(request, context, receipt);
    },
    take(request: Request, context: string) {
      if (!receipt || pending || !receipt.items.length || now() - receipt.reviewedAt >= 300_000 || !same(request, context, receipt)) {
        invalidate();
        throw new Error('The selection, inputs or preview changed. Run Preview again before applying.');
      }
      const reviewed = snapshot(receipt);
      invalidate(); // Every Apply attempt consumes the receipt, including errors.
      return reviewed;
    },
  };
}

function validFields(value: unknown): value is Fields {
  return !!value && typeof value === 'object' && !Array.isArray(value) &&
    Object.values(value).every(values => Array.isArray(values) && values.every(item => typeof item === 'string'));
}
/** The preview validator of one converter dialog; it forwards the files the preview leaves unchanged. */
export function converterValidator(kind: 'filename' | 'text') {
  return (request: FilenameConversionRequest | TextImportRequest, items: (FilenameConversionItem | TextImportItem)[], unmatched: string[]) =>
    kind === 'filename'
      ? validateFilenameItems(request as FilenameConversionRequest, items as FilenameConversionItem[], unmatched)
      : validateTextItems(request as TextImportRequest, items as TextImportItem[], unmatched);
}
/** Every selected file is either renamed by one item or left unchanged, in selection order. */
export function validateFilenameItems(request: FilenameConversionRequest, items: FilenameConversionItem[], unmatched: string[] = []) {
  let item = 0, skipped = 0;
  const valid = items.length + unmatched.length === request.paths.length && request.paths.every(path => {
    const current = items[item];
    if (current?.path === path) {
      item++;
      return typeof current.target === 'string' && !!current.target && validFields(current.captures);
    }
    return unmatched[skipped++] === path;
  });
  if (!valid) throw new Error('The filename preview does not match the selected files. Run Preview again.');
  const sources=new Set<string>(),targets=new Set<string>();
  for(const row of items){
    if(row.associated===undefined)continue;
    if(!Array.isArray(row.associated)||row.associated.some(member=>{
      if(!member||typeof member.from!=='string'||!member.from||typeof member.to!=='string'||!member.to||sources.has(member.from)||targets.has(member.to))return true;
      sources.add(member.from);targets.add(member.to);return false;
    }))throw new Error('The companion preview could not be verified. Run Preview again.');
  }
}
/** Keyed records may follow source-line order; the selected files no record names are left unchanged. */
export function validateTextItems(request: TextImportRequest, items: TextImportItem[], unmatched: string[] = []) {
  const indices = new Set<number>();
  if (items.length + unmatched.length !== request.paths.length || items.some(item => {
    const index = item.selection_index;
    const invalid = !Number.isInteger(index) || index < 0 || index >= request.paths.length ||
      item.path !== request.paths[index] || indices.has(index) || !Number.isInteger(item.source_line) || item.source_line < 1 || !validFields(item.fields);
    indices.add(index);
    return invalid;
  }) || request.paths.filter((_, index) => !indices.has(index)).some((path, index) => unmatched[index] !== path)) {
    throw new Error('The text preview does not match the selected files. Run Preview again.');
  }
}
