// Cover window and Manage Covers state: selection aggregation, cover details
// and whole-list cover plans. No DOM access, so it is unit-tested directly.
export type CoverInfo={index:number;mime:string;description:string;picture_type:string;data_base64:string;size?:number;width?:number|null;height?:number|null;digest?:string};
export type CoverRef={digest:string;picture_type:string;description:string};
export type SetCover=CoverRef&{mime:string;size:number;width:number|null;height:number|null;files:number[];index:number;data_base64:string};
export type CoverSet={files:number;covers:SetCover[];errors:{path:string;error:string}[]};
export type StagedImage={token:string;mime:string;size:number;width:number|null;height:number|null;data_base64:string;label:string};
export type ImageSource={staged:string}|{path:string};
/** `occurrence` picks one of several identical covers in a file; without it every copy is meant. */
export type PlanItem={original:CoverRef|null;image:ImageSource|null;picture_type:string;description:string;all_files:boolean;occurrence?:number};

/** ID3v2 picture types in APIC order with Mp3tag's names. */
export const PICTURE_TYPES:[string,string][]=[['Other','Other'],['Icon','File icon'],['OtherIcon','Other file icon'],['CoverFront','Front Cover'],['CoverBack','Back Cover'],['Leaflet','Leaflet'],['Media','Media'],['LeadArtist','Lead artist'],['Artist','Artist'],['Conductor','Conductor'],['Band','Band or orchestra'],['Composer','Composer'],['Lyricist','Lyricist'],['RecordingLocation','Recording location'],['DuringRecording','During recording'],['DuringPerformance','During performance'],['ScreenCapture','Screen capture'],['BrightFish','A bright colored fish'],['Illustration','Illustration'],['BandLogo','Band logo'],['PublisherLogo','Publisher logo']];
export function pictureTypeLabel(type:string){return PICTURE_TYPES.find(([id])=>id===type)?.[1]||type||'Cover';}

export function formatSize(bytes:number){
  if(bytes<1024)return `${bytes} B`;
  if(bytes<1024*1024)return `${Math.round(bytes/1024)} KB`;
  return `${(bytes/1024/1024).toFixed(1)} MB`;
}
export function imageFormat(mime:string){
  const subtype=mime.replace(/^image\//i,'').toLowerCase();
  return ({jpeg:'JPEG',jpg:'JPEG',png:'PNG',gif:'GIF',bmp:'BMP',tiff:'TIFF',webp:'WEBP'} as Record<string,string>)[subtype]||subtype.toUpperCase()||'Unknown';
}
/** "1200×1200, 245 KB, JPEG" like Mp3tag's cover window. */
export function coverDetails(cover:{mime:string;size?:number;width?:number|null;height?:number|null}){
  return [cover.width&&cover.height?`${cover.width}×${cover.height}`:'',cover.size?formatSize(cover.size):'',imageFormat(cover.mime)].filter(Boolean).join(', ');
}
/** Identity used to compare covers between files. Without a digest (browser preview) size stands in for content. */
export function coverKey(cover:{digest?:string;picture_type:string;description:string;size?:number;mime?:string}){
  return [cover.digest||`${cover.mime}:${cover.size}`,cover.picture_type,cover.description].join('\u0000');
}
export function coverRef(cover:{digest?:string;picture_type:string;description:string}):CoverRef{
  return {digest:cover.digest||'',picture_type:cover.picture_type,description:cover.description};
}

export type SelectionCovers={state:'none'|'same'|'varying';files:number;withCovers:number;distinct:CoverRef[]};
/** Mp3tag shows the covers only when every selected file has the same ones. */
export function selectionCovers(lists:CoverInfo[][]):SelectionCovers{
  const signatures=lists.map(list=>list.map(coverKey).join('\u0001'));
  const withCovers=lists.filter(list=>list.length).length;
  const seen=new Map<string,CoverRef>();
  for(const list of lists)for(const cover of list){const key=coverKey(cover);if(!seen.has(key))seen.set(key,coverRef(cover));}
  const state=!lists.length||!withCovers?'none':signatures.every(signature=>signature===signatures[0])?'same':'varying';
  return {state,files:lists.length,withCovers,distinct:[...seen.values()]};
}

/** Plan that keeps every cover of a uniform selection except edits made by `change`. */
export function keepPlan(covers:CoverInfo[],change:(item:PlanItem,index:number)=>PlanItem|null=item=>item):PlanItem[]{
  return covers.flatMap((cover,index)=>{
    const occurrence=covers.slice(0,index).filter(other=>coverKey(other)===coverKey(cover)).length;
    const item=change({original:coverRef(cover),image:null,picture_type:cover.picture_type,description:cover.description,all_files:false,occurrence},index);return item?[item]:[];
  });
}
/** Plan for varying selections: every distinct cover, each kept only where present. */
export function distinctPlan(distinct:CoverRef[],change:(item:PlanItem)=>PlanItem|null=item=>item):PlanItem[]{
  return distinct.flatMap(cover=>{const item=change({original:{...cover},image:null,picture_type:cover.picture_type,description:cover.description,all_files:false});return item?[item]:[];});
}

export type ManagedCover={id:string;original:CoverRef|null;image:ImageSource|null;picture_type:string;description:string;all_files:boolean;files:number;total:number;mime:string;size:number;width:number|null;height:number|null;data_base64:string;label:string};
export function managedFromSet(set:CoverSet):ManagedCover[]{
  return set.covers.map((cover,index)=>({id:`c${index}`,original:coverRef(cover),image:null,picture_type:cover.picture_type,description:cover.description,all_files:false,files:cover.files.length,total:set.files,mime:cover.mime,size:cover.size,width:cover.width,height:cover.height,data_base64:cover.data_base64,label:''}));
}
/** New images are front covers; a repeated type gets the file name as description, as ID3 needs distinct descriptions. */
export function newManaged(list:ManagedCover[],image:StagedImage,id:string,total:number):ManagedCover{
  const type='CoverFront';
  const clash=list.some(cover=>cover.picture_type===type&&!cover.description);
  let description=clash?image.label:'';
  for(let n=2;description&&list.some(cover=>cover.picture_type===type&&cover.description===description);n++)description=`${image.label} (${n})`;
  return {id,original:null,image:{staged:image.token},picture_type:type,description,all_files:true,files:total,total,mime:image.mime,size:image.size,width:image.width,height:image.height,data_base64:image.data_base64,label:image.label};
}
export function replaceManaged(cover:ManagedCover,image:StagedImage):ManagedCover{
  return {...cover,image:{staged:image.token},mime:image.mime,size:image.size,width:image.width,height:image.height,data_base64:image.data_base64,label:image.label};
}
/** Move the selected covers one step, keeping their relative order; blocked moves leave the list unchanged. */
export function moveManaged(list:ManagedCover[],ids:Set<string>,delta:-1|1):ManagedCover[]{
  const result=[...list],order=delta<0?result.map((_,i)=>i):result.map((_,i)=>result.length-1-i);
  for(const index of order){
    if(!ids.has(result[index].id))continue;
    const target=index+delta;if(target<0||target>=result.length||ids.has(result[target].id))continue;
    [result[index],result[target]]=[result[target],result[index]];
  }
  return result;
}
/** Move the dragged covers before `before` (or to the end). */
export function dropManaged(list:ManagedCover[],ids:Set<string>,before:string|null):ManagedCover[]{
  const moving=list.filter(cover=>ids.has(cover.id)),rest=list.filter(cover=>!ids.has(cover.id));
  const at=before===null?rest.length:rest.findIndex(cover=>cover.id===before);
  if(at<0)return list;
  return [...rest.slice(0,at),...moving,...rest.slice(at)];
}
export function planItems(list:ManagedCover[]):PlanItem[]{
  return list.map(cover=>({original:cover.original,image:cover.image,picture_type:cover.picture_type,description:cover.description,all_files:cover.all_files||!cover.original}));
}
export function planChanged(initial:ManagedCover[],list:ManagedCover[]){
  return JSON.stringify(planItems(initial))!==JSON.stringify(planItems(list));
}
export function presence(cover:ManagedCover){
  const files=cover.all_files||!cover.original?cover.total:cover.files;
  return {full:files>=cover.total,text:files>=cover.total?`In all ${cover.total} file${cover.total===1?'':'s'}`:`In ${files} of ${cover.total} files`};
}

export type CoverCommand='add'|'replace'|'remove'|'extract'|'adjust'|'manage'|'type'|'description'|'cut'|'copy'|'paste'|'previous'|'next'|'view'|'aspect';
export type CoverMenuItem={command:CoverCommand;label:string;enabled:boolean;shortcut?:string;checked?:boolean;value?:string;children?:CoverMenuItem[]}|'separator';
export type CoverMenuState={selected:number;state:SelectionCovers['state'];count:number;index:number;displayedType?:string;canEdit:boolean;canClipboard:boolean;aspect:boolean;mac:boolean};
/** The documented cover window context menu plus navigation and the larger viewer. */
export function coverMenu(s:CoverMenuState):CoverMenuItem[]{
  const mod=s.mac?'⌘':'Ctrl+',shown=s.state==='same'&&s.count>0,edit=s.canEdit&&s.selected>0,any=s.state!=='none';
  return [
    {command:'add',label:'Add cover…',enabled:edit},
    {command:'remove',label:'Remove cover',enabled:edit&&any,shortcut:'Del'},
    {command:'replace',label:'Replace cover…',enabled:edit&&shown},
    {command:'extract',label:'Extract cover…',enabled:shown&&s.selected>0},
    {command:'adjust',label:'Adjust cover…',enabled:edit&&any},
    {command:'manage',label:'Manage covers…',enabled:edit,shortcut:s.mac?'⌘⌥M':'Ctrl+Alt+M'},
    'separator',
    {command:'type',label:'Set cover type',enabled:edit&&any,children:PICTURE_TYPES.map(([id,label])=>({command:'type' as const,label,value:id,enabled:edit&&any,checked:shown&&s.displayedType===id}))},
    {command:'description',label:'Set cover description…',enabled:edit&&any},
    'separator',
    {command:'cut',label:'Cut cover',enabled:edit&&shown&&s.canClipboard,shortcut:`${mod}X`},
    {command:'copy',label:'Copy cover',enabled:shown&&s.canClipboard,shortcut:`${mod}C`},
    {command:'paste',label:'Paste cover',enabled:edit&&s.canClipboard,shortcut:`${mod}V`},
    'separator',
    {command:'previous',label:'Previous cover',enabled:shown&&s.index>0,shortcut:'←'},
    {command:'next',label:'Next cover',enabled:shown&&s.index<s.count-1,shortcut:'→'},
    {command:'view',label:'View larger…',enabled:shown,shortcut:'Enter'},
    {command:'aspect',label:'Correct aspect ratio',enabled:true,checked:s.aspect},
  ];
}

/** File extension for extracted covers (mirrors the backend). */
export function coverExtension(mime:string){
  return ({'image/png':'png','image/gif':'gif','image/bmp':'bmp','image/tiff':'tif','image/webp':'webp'} as Record<string,string>)[mime.toLowerCase()]||'jpg';
}
const IMAGE_EXTENSIONS=['jpg','jpeg','png','gif','bmp','webp'];
export function isImagePath(path:string){return IMAGE_EXTENSIONS.includes((/\.([^.\\/]+)$/.exec(path)?.[1]||'').toLowerCase());}
export const imageExtensions=()=>[...IMAGE_EXTENSIONS];
/** A file name from an evaluated format string: path separators and reserved characters are replaced. */
export function coverFileName(value:string,fallback='folder'){
  const name=value.replace(/[\\/:*?"<>|\u0000-\u001f]/g,'_').replace(/^[\s.]+|[\s.]+$/g,'');
  return name||fallback;
}
/** Physical drop coordinates (Tauri) against a CSS-pixel rectangle. */
export function insideRect(position:{x:number;y:number},rect:{left:number;top:number;right:number;bottom:number},scale:number){
  const x=position.x/scale,y=position.y/scale;
  return x>=rect.left&&x<=rect.right&&y>=rect.top&&y<=rect.bottom;
}
