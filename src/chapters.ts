// Chapter and cue sheet rows (Options → Tags → Advanced → List chapters as
// separate files, https://docs.mp3tag.de/options/tags/advanced/). A row's
// path is its file path, U+0001 and the 1-based subsong number; the backend
// reads and writes rows by that path.
export const SUBSONG_SEPARATOR='\u0001';

/** The file and subsong number of a chapter or cue sheet row. */
export function subsongParts(path:string):{file:string;number:number}|undefined{
  const at=path.lastIndexOf(SUBSONG_SEPARATOR);if(at<=0)return undefined;
  const number=Number(path.slice(at+1));
  return Number.isInteger(number)&&number>0?{file:path.slice(0,at),number}:undefined;
}
export const isChapterRow=(path:string)=>subsongParts(path)!==undefined;
const isCueRow=(path:string)=>!!subsongParts(path)?.file.toLowerCase().endsWith('.cue');
/** The file a row belongs to (a chapter file or a cue sheet), or the path itself. */
export const rowFile=(path:string)=>subsongParts(path)?.file??path;

/** Commands that work on whole files. The backend runs them once on the file of chapter rows; rows of a loaded cue sheet edit the sheet, so they refuse them. */
const PARENT_FILE=['add-cover','remove-cover','cover-manage','remove-tags','remove-tag-types','strip-mp4-chapters'];
/** Commands that rename, move or delete files, or list them in playlists; a chapter row is not a file of its own. */
const WHOLE_FILE=['save-copy','tags-filename','filename-filename','playlist-selected'];
/** Commands that write every listed file (or its folder) and therefore every row. */
const ALL_FILES=['playlist-all','playlist-generate'];
export const CHAPTER_ROW_REASON='Chapter rows are not files of their own. Turn off Options → Tags → Advanced → List chapters as separate files and reload the directory to rename, move or list the file.';
export const CUE_ROW_REASON='Cue sheet tracks only support editing tag fields, which are written into the cue sheet.';

/** Why `op` is unavailable for the selected (and listed) rows, or ''. */
export function chapterRowReason(op:string,selected:Iterable<string>,listed:()=>Iterable<string>=()=>[]):string{
  const rows=(paths:Iterable<string>,test=isChapterRow)=>{for(const path of paths)if(test(path))return true;return false;};
  // Show in file manager and Play use the audio file of the row.
  if(op==='file-reveal'||op==='file-play')return '';
  if((PARENT_FILE.includes(op)||op.startsWith('util-'))&&rows(selected,isCueRow))return CUE_ROW_REASON;
  if((WHOLE_FILE.includes(op)||op.startsWith('file-'))&&rows(selected))return CHAPTER_ROW_REASON;
  if(ALL_FILES.includes(op)&&rows(listed()))return CHAPTER_ROW_REASON;
  return '';
}

/**
 * The list order after `returned` paths came back from the backend: the
 * listed rows (or file) of a file are replaced, at the place of the first
 * one, by what came back for it — its rows, or the file itself once it has
 * no chapters — when something new came back for it (Undo, a whole-file
 * command) or it is one of the `whole` files a whole-file command ran on.
 * A save that returns some of a file's rows changes nothing.
 */
export function replaceFileRows(order:string[],returned:string[],whole:Iterable<string>=[]):string[]{
  const listed=new Set(order),replace=new Set(whole),groups=new Map<string,string[]>();
  for(const path of returned){const file=rowFile(path);if(!groups.has(file))groups.set(file,[]);groups.get(file)!.push(path);}
  for(const [file,paths] of groups){
    const before=order.filter(path=>rowFile(path)===file);
    const same=before.length===paths.length&&before.every((p,i)=>p===paths[i]);
    if(!before.length||same||!(replace.has(file)||paths.some(path=>!listed.has(path))))groups.delete(file);
  }
  if(!groups.size)return order;
  const out:string[]=[],done=new Set<string>();
  for(const path of order){
    const file=rowFile(path),group=groups.get(file);
    if(!group){out.push(path);continue;}
    if(!done.has(file)){done.add(file);out.push(...group);}
  }
  return out;
}
