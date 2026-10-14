type Fields = Record<string, string[]>;

type FilterEntry = {
  path: string;
  filename: string;
  duration: number;
  bitrate: number;
  sample_rate: number;
  size: number;
  technical?: Fields;
};

// Keep application, selection, clock and export context with their native owners.
// This is the same ownership boundary as evaluation::technical_fields.
const contextOwned = new Set([
  '_APP', '_COUNTER', '_FILETYPE', '_DATE', '_DATETIME', '_WORKINGDIR', '_WORKINGPATH',
  '_FILENAME_REL', '_FOLDERPATH_REL', '_CRC', '_MD5', '_MD5AUDIO', '_MAX_COUNTER',
  '_TOTAL_FILES', '_TOTAL_SIZE', '_TOTAL_SIZE_RAW', '_TOTAL_TIME', '_TOTAL_TIME2',
  '_TOTAL_TIME_RAW', '_PLAYLIST_FILENAME', '_PLAYLIST_FILENAME_EXT', '_PLAYLIST_FOLDERPATH',
]);

/** Preserve the legacy projection, then prefer actual file measurements. */
export function buildFilterFields(entry: FilterEntry, rawFields: Fields, source?: {uri?: string}): Fields {
  const separator = entry.path.includes('\\') ? '\\' : '/';
  const folder = entry.path.slice(0, Math.max(entry.path.lastIndexOf('/'), entry.path.lastIndexOf('\\')));
  const dot = entry.filename.lastIndexOf('.');
  const extension = dot > 0 ? entry.filename.slice(dot + 1) : '';
  const stem = dot > 0 ? entry.filename.slice(0, dot) : entry.filename;
  const fields: Fields = {
    ...rawFields,
    _FILENAME: [stem],
    _FILENAME_EXT: [entry.filename],
    _PATH: [entry.path],
    _FOLDERPATH: [folder + separator],
    _DIRECTORY: [folder.split(/[\\/]/).pop() || folder],
    _EXTENSION: [extension],
    _LENGTH_SECONDS: [String(entry.duration)],
    _BITRATE: [String(entry.bitrate)],
    _SAMPLERATE: [String(entry.sample_rate)],
    _FILE_SIZE: [String(entry.size)],
  };
  const provider = !!source || Object.entries(entry.technical ?? {}).some(([name, values]) => name.toUpperCase() === '_PATH' && values.some(value => value.startsWith('content://')));
  for (const [name, values] of Object.entries(entry.technical ?? {})) {
    const key = name.replace(/[a-z]/g, letter => letter.toUpperCase());
    if (key.startsWith('_') && !contextOwned.has(key)) fields[key] = [...values];
  }
  if (provider) {
    delete fields._FOLDERPATH; delete fields._DIRECTORY;
    if(source)fields._PATH=source.uri?[source.uri]:[];
  }
  return fields;
}
