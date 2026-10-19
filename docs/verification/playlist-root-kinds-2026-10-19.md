# Filesystem kinds for playlist-shaped input roots — 2026-10-19

PR #64 review [4319096414](https://github.com/johnneerdael/TagGroove/pull/64#discussion_r4319096414) identified a real directory named `archive.m3u` being mistaken for a playlist. The finding applies to input expansion as well as tool-relative paths and Keep moved files. This is a repair of represented loading/workspace behavior, not a new L47a inline-path or external-rename Refresh feature.

## Cause and repair

The earlier code guessed root kind from a playlist suffix and membership in scanned audio rows. A directory is not itself an audio row, and a suffix does not establish whether a path is a file. The production load could therefore try to read the directory as a playlist; the tool context could also become its parent instead of the folder itself.

The backend now classifies original input paths from filesystem metadata as file, directory or unknown. Expansion and the retained path-to-kind map use that result. Real directories keep their own root context even when named with a supported playlist suffix. Actual playlist files still expand in source order and use their containing folder for relative tool placeholders. Missing or unreadable roots are not silently declared directories.

The map follows append/replace, deduplication, Refresh and directory/file rebasing. Workspace version 2 may capture root kinds as aligned metadata; old snapshots remain readable and old journal serialization omits the absent optional field. Restore rechecks actual roots, including a captured active playlist outside the root list, because the filesystem or a portable root mapping can change their kinds.

Independent review found and corrected an interaction in the first repair draft: Refresh must preserve the chosen active playlist rather than switching to the last appended playlist. A captured null active playlist must remain null; metadata classification does not select another playlist. The normal single-root location label and its data/literal flag are also preserved.

## Focused evidence

- The sanitized baseline replay against the earlier production source fails the new suffixed-directory loading case with `false !== true`; it is an actual load failure, not a missing test symbol. Retained run: `taggroove-root-kinds-red-bbcxch2x`.
- The final nine directly affected frontend files pass **89 tests**, with no failures or skips. Cases cover all seven playlist suffixes on directories, actual playlist files, tool-relative roots, Keep moved files, append/replace/rebase, active-playlist preservation, mapped/legacy workspace restoration and stale/cancelled publication. Retained run: `taggroove-root-kinds-final-ui-qyph681l`; all 14 changed frontend source/test hashes match before and after execution.
- Final TypeScript checking passes in `taggroove-root-kinds-final-types-pfdulwjl`. Formatting and whitespace checks pass. The strict guide passes in `taggroove-playlist-root-kind-guide-o3u3k5w8`.
- Independent read-only review found the active-playlist interaction described above, then verified its correction and the legacy outside-root context. Final re-review has no remaining concrete findings.
- The final Rust checks pass **28 tests**: one filesystem-metadata test covering actual files/directories for every supported playlist suffix; 26 profile-migration/legacy-journal compatibility tests; and one portable-root mapping/row-rank/audio-byte preservation test. The profile module's one intentionally ignored controlled restart child is invoked by its parent sandbox test. Retained runs: `taggroove-root-kinds-native-metadata-final-0upwuxz7`, `taggroove-root-kinds-profile-compat-final-2qvas22p` and `taggroove-root-kinds-native-remap-d67b8kkb`; final before/after source bindings match.
- All 16 local links across the three changed Markdown documents resolve. The initial Rust invocation compiled but selected zero tests because its exact filter was unqualified; it is retained as compilation-only evidence, not counted in these results. An incomplete new JSON test fixture was corrected before the final compatibility run; production behavior was unchanged by that correction.

## Limits

These checks concern local filesystem input classification and represented workspace state. They do not establish original Windows behavior, native GUI acceptance on other operating systems, physical provider behavior, arbitrary concurrent filesystem-writer exclusion, or a new rename/Refresh membership policy. No local application build or broad native UI sweep is needed for this repair; post-merge native integration remains a separate CI evidence layer.
