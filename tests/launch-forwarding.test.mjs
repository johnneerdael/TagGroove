import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const read = path => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const base = JSON.parse(await read('src-tauri/tauri.conf.json'));
const mac = JSON.parse(await read('src-tauri/tauri.macos.conf.json'));
const tags = await read('crates/tag-core/src/tags.rs');
const playlists = await read('src/playlist-ui.ts');
const audio = [...tags.match(/fn audio_extension[\s\S]*?matches!\(\s*extension\.as_str\(\),([\s\S]*?)\)\s*\}/)[1].matchAll(/"([^"]+)"/g)].map(m => m[1]);
const playlistExtensions = JSON.parse(playlists.match(/PLAYLIST_EXTENSIONS=(\[[^\]]*\])/)[1].replace(/'/g, '"'));
// Extensions Tauri's bundler turns into LSItemContentTypes. macOS ignores
// CFBundleTypeExtensions in an entry that has content types, so these may not
// share an entry with extension-only types (tauri-utils `extension_to_uti`).
const INFERRED = ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'tiff', 'tif', 'ico', 'heic', 'heif', 'webp', 'svg', 'mp4', 'mov', 'avi', 'mkv', 'mp3', 'wav', 'aac', 'm4a', 'pdf', 'txt', 'rtf', 'html', 'htm', 'json', 'xml'];
const associations = mac.bundle.fileAssociations;

test('Finder Open With covers every readable audio extension, playlists and folders', () => {
  assert.ok(audio.length > 30 && audio.includes('flac') && audio.includes('mp+'), 'tag-core extension list parsed');
  const byExtension = new Set(associations.flatMap(a => a.ext));
  const types = new Set(associations.flatMap(a => a.contentTypes || []));
  // System audio types (mp3, AAC, M4A, WAV) conform to public.audio; MP4/Matroska video use their own types.
  const typed = {mp3: 'public.audio', wav: 'public.audio', aac: 'public.audio', m4a: 'public.audio', mp4: 'public.mpeg-4', mkv: 'org.matroska.mkv'};
  for (const ext of audio) assert.ok(byExtension.has(ext) || types.has(typed[ext]), `.${ext} is not associated`);
  for (const ext of playlistExtensions) assert.ok(byExtension.has(ext), `.${ext} playlists are not associated`);
  assert.ok(associations.some(a => a.contentTypes?.includes('public.folder') && a.role === 'Viewer'), 'folders can be dropped on the Dock icon');
});

test('associations never claim the default handler and keep extensions effective', () => {
  for (const association of associations) {
    assert.equal(association.rank, 'Alternate', `${association.name} must not become the default app`);
    assert.ok(['Editor', 'Viewer'].includes(association.role));
    if (association.ext.length) {
      assert.equal(association.contentTypes, undefined, `${association.name} mixes content types with extensions`);
      for (const ext of association.ext) assert.ok(!INFERRED.includes(ext), `.${ext} in ${association.name} would add LSItemContentTypes and void its extensions`);
    }
  }
  assert.equal(associations.find(a => a.name === 'Audio file (by extension)').role, 'Editor');
  // The Windows installer would replace the default .mp3 handler and Android would gain intent filters.
  assert.equal(base.bundle.fileAssociations, undefined, 'file associations stay macOS-only');
});
