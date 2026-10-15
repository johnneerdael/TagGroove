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

/** Commands that work on whole files; chapter rows only edit tag fields. */
const WHOLE_FILE=['save-copy','add-cover','remove-cover','cover-manage','remove-tags','remove-tag-types','strip-mp4-chapters','tags-filename','filename-filename','playlist-selected'];
/** Commands that write every listed file (or its folder) and therefore every row. */
const ALL_FILES=['playlist-all','playlist-generate'];
export const CHAPTER_ROW_REASON='Chapter rows only support editing tag fields. Turn off Options → Tags → Advanced → List chapters as separate files and reload the directory to work with the whole file.';

/** Why `op` is unavailable for the selected (and listed) rows, or ''. */
export function chapterRowReason(op:string,selected:Iterable<string>,listed:()=>Iterable<string>=()=>[]):string{
  const rows=(paths:Iterable<string>,test=isChapterRow)=>{for(const path of paths)if(test(path))return true;return false;};
  const whole=WHOLE_FILE.includes(op)||op.startsWith('util-')||(op.startsWith('file-')&&op!=='file-play');
  if(whole&&rows(selected))return CHAPTER_ROW_REASON;
  if(op==='file-play'&&rows(selected,path=>!!subsongParts(path)?.file.toLowerCase().endsWith('.cue')))return CHAPTER_ROW_REASON;
  if(ALL_FILES.includes(op)&&rows(listed()))return CHAPTER_ROW_REASON;
  return '';
}
