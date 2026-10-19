# Filesystem kinds for playlist-shaped input roots — 2026-10-19

PR #64 review [4319096414](https://github.com/johnneerdael/TagGroove/pull/64#discussion_r4319096414) identified a real directory named `archive.m3u` being mistaken for a playlist. The finding applies to input expansion as well as tool-relative paths and Keep moved files. This is a repair of represented loading/workspace behavior, not a new L47a inline-path or external-rename Refresh feature.

## Cause and repair

The earlier code guessed root kind from a playlist suffix and membership in scanned audio rows. A directory is not itself an audio row, and a suffix does not establish whether a path is a file. The production load could therefore try to read the directory as a playlist; the tool context could also become its parent instead of the folder itself.

The backend now classifies original input paths from filesystem metadata as file, directory or unknown. Expansion and the retained path-to-kind map use that result. Real directories keep their own root context even when named with a supported playlist suffix. Actual playlist files still expand in source order and use their containing folder for relative tool placeholders. Missing or unreadable roots are not silently declared directories.

The map follows append/replace, deduplication, Refresh and directory/file rebasing. Workspace version 2 may capture root kinds as aligned metadata; old snapshots remain readable and old journal serialization omits the absent optional field. Restore rechecks actual roots, including a captured active playlist outside the root list, because the filesystem or a portable root mapping can change their kinds.

Independent review found and corrected an interaction in the first repair draft: Refresh must preserve the chosen active playlist rather than switching to the last appended playlist. A captured null active playlist must remain null; metadata classification does not select another playlist. The normal single-root location label and its data/literal flag are also preserved.

## Focused evidence

- The sanitized baseline replay against the earlier production source fails the new suffixed-directory loading case with `false !== true`; it is an actual load failure, not a missing test symbol. Retained run: `taggroove-root-kinds-red-bbcxch2x`.
- At repair checkpoint `493ecf1b5c1e3e06058c7808a362de657daeef85`, the nine directly affected frontend files pass **89 tests**, with no failures or skips. Cases cover all seven playlist suffixes on directories, actual playlist files, tool-relative roots, Keep moved files, append/replace/rebase, active-playlist preservation, mapped/legacy workspace restoration and stale/cancelled publication. Retained run: `taggroove-root-kinds-final-ui-qyph681l`; all 14 changed frontend source/test hashes match before and after execution.
- Final TypeScript checking passes in `taggroove-root-kinds-final-types-pfdulwjl`. Formatting and whitespace checks pass. The strict guide passes in `taggroove-playlist-root-kind-guide-o3u3k5w8`.
- Independent read-only review found the active-playlist interaction described above, then verified its correction and the legacy outside-root context. Final re-review has no remaining concrete findings.
- The final Rust checks pass **28 tests**: one filesystem-metadata test covering actual files/directories for every supported playlist suffix; 26 profile-migration/legacy-journal compatibility tests; and one portable-root mapping/row-rank/audio-byte preservation test. The profile module's one intentionally ignored controlled restart child is invoked by its parent sandbox test. Retained runs: `taggroove-root-kinds-native-metadata-final-0upwuxz7`, `taggroove-root-kinds-profile-compat-final-2qvas22p` and `taggroove-root-kinds-native-remap-d67b8kkb`; final before/after source bindings match.
- All 16 local links across the three changed Markdown documents resolve. The initial Rust invocation compiled but selected zero tests because its exact filter was unqualified; it is retained as compilation-only evidence, not counted in these results. An incomplete new JSON test fixture was corrected before the final compatibility run; production behavior was unchanged by that correction.

## Empty active playlist append follow-up

The next exact-head Codex review identified a separate row-count edge: an active
playlist with zero readable rows was treated as an unchosen source when another
playlist was appended. Source choice must survive that empty result. Explicit
replacement still chooses the replacement playlist; an empty workspace with no
active playlist can still adopt its first playlist. The corrected condition also retains the source's displayed location/data flag
and chosen playlist/column/manual order. The sanitized regression against the
preceding source has 32 passes and one actual A-to-B context failure
(`taggroove-empty-active-red-final-qf3y84yz`); the corrected file passes all
**33 focused loading tests** (`taggroove-empty-active-green-gh5d5kch`). Empty
and all-missing playlists, empty/nonempty folder controls and explicit replacement
are covered. TypeScript passes in `taggroove-empty-active-types-xueu17ss`; both
changed source/test hashes match before and after. Independent read-only review
has no concrete findings. No Rust build or native UI run was repeated.

## CI native-bridge and catalog integration follow-up — 2026-10-20

[Linux validation 39333168455](https://github.com/johnneerdael/TagGroove/actions/runs/39333168455/job/118336307261) passes the Rust workspace tests and strict workspace/all-target Clippy at `91accdb9fc52b37d9b90a12d2359caa324392d63`. Its frontend suite then reports 853 passes, 11 failures and one skip. The failed job log is retained as `/tmp/taggroove-pr64-91accdb-linux-118336307261-plain.log`.

One failure was the new root-kind error missing from the backend-key snapshot. Regeneration adds exactly that key, with English/Dutch/German entries; all three catalogs now have 3,494 canonical keys. One playlist-diagnostic test assumed every native call was `read_playlist`; nine bundled startup/recovery cases lacked the registered `classify_source_paths` command. The native test doubles now provide explicit filesystem kinds and reject missing fixture paths. Additional assertions verify metadata initialization and scan order. Existing diagnostic, pending-receipt, recovery, acknowledgement, launch and timeout assertions remain intact. Production TypeScript and Rust logic are unchanged by this follow-up.

Focused baseline startup: six passes/nine failures; named playlist diagnostic: one failure; catalog-registration case: one failure. Corrected checks: **15 startup tests**, **one named playlist-diagnostic test** and **46 localization/extraction tests** pass. Retained runs: `taggroove-kind-mocks-startup-final-ie5dnu0a`, `taggroove-kind-mocks-dom-final-0uwhj6m1`, and `taggroove-ci-frontend-catalog-green-nwz6ff5_`. The changed test-file bindings match before and after execution. Catalog extraction consistency and whitespace checks pass; independent read-only review has no concrete findings. No full frontend suite, Rust workspace, native app build or native UI sweep was repeated locally.

## Limits

These checks concern local filesystem input classification and represented workspace state. They do not establish original Windows behavior, native GUI acceptance on other operating systems, physical provider behavior, arbitrary concurrent filesystem-writer exclusion, or a new rename/Refresh membership policy. No local application build or broad native UI sweep is needed for this repair; post-merge native integration remains a separate CI evidence layer.
