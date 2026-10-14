# Native workflow execution checkpoint

On 2026-10-14, native CUA operated the macOS arm64 debug app against six
independently generated three-second audio files under `/tmp/opentag-user-test`.
No user music files were selected or modified. The application identifier alone
was overridden to `org.opentag.editor.usertest`; the executable/source identities
are recorded in `user-workflows-results.json`. This is functional source testing,
not a byte-identical public release package or an original-Mp3tag differential.

## Discovery: partially verified

The actual Open folder picker recursively loaded six files: two MP3, FLAC, M4A,
Opus and WAV, including Unicode filenames and actual FLAC multivalues. Adding
both already-loaded MP3s through Add files kept six unique rows. Cancelling a
subsequent picker preserved the list. Independent complete-file SHA-256,
encoded-packet hashes and decoded PCM hashes remained unchanged for all six.

The plan's initial two-file load, both sort directions, all-row selection and
individual sample-rate/channel inspection remain unexecuted. UAT-01 is PARTIAL.

The native Go to Folder sheet did not accept automated Return/Escape input
reliably. The test used the picker's visible sidebar, location menu, folder list
and Open buttons instead. This automation limitation is not recorded as an app
failure or as a passed keyboard-navigation test.

## Common editing: passed after a reproduced fix

In a fresh common-case copy, the two MP3s showed their shared album and mixed
comments. Setting ALBUM to `UAT Shared Album` and COMMENT to
`UAT common — Café 猫` created two drafts. Before Save, all six complete-file
hashes and audio identities remained unchanged. Conversion/numbering/cover
operations showed pending-state restrictions.

The first actual Save reported two failures:
`The format could not preserve all values for field COMMENT`.
Both originals, all unselected files, and all audio remained unchanged. Raw ID3
inspection independently identified FFmpeg's `TXXX:comment` frame. The reader
exposes it as the same case-insensitive logical field as `COMM`; the writer
previously retained the old alias while inserting the standard frame.

The focused fix removes a matching logical-name UserText alias when replacing
its standard mapped field. Other fields remain untouched. A real ID3-tagged WAV
regression first failed with the native error, then passed replacement, Unicode,
unedited title-alias preservation, subsequent title replacement, comment removal,
unmanaged-field preservation and exact audio-byte checks. This does not establish
universal support for every alternate alias or MOVEMENT representation.

After rebuilding and reopening the unchanged common fixtures, the same two
drafts saved successfully: `Saved tags: 2 files`. Reload returned the exact album
and Unicode comment, with no pending edits or operation errors. Independent
ffprobe inspection confirmed both tag dictionaries equal their originals with
only the requested album/comment changes. Exactly the two selected MP3 file
hashes changed; the four unselected files stayed byte-for-byte identical. All six
encoded-packet hashes and decoded PCM hashes matched the baseline. UAT-02 PASS.

![Native save failure before the fix](user-workflow-comment-before.png)

![Successful save and reload after the fix](user-workflow-comment-after.png)

The regular app retained System (Dark) when the isolated app selected Light.
The regular profile's action configuration did not exist before testing; action
configuration persistence/isolation will be exercised with the group workflow.
The isolated app retained Light across actual native Quit/relaunch cycles.

Validation: 15 metadata integration tests and 13 mapping integration tests passed,
including the generated encoded-format/audio-packet matrix. Core all-target
Clippy passed with warnings denied. The isolated native debug bundle built and
signed ad-hoc successfully. These results do not certify mobile file access.

## Native filters: passed with recorded dataset variation

The visible Filter control exercised Boolean AND/NOT, scripted `$upper` values,
Boost lookbehind and pattern backreferences, malformed Boolean syntax, an invalid
regular expression, and a quick valid → invalid → clear sequence. Final clearing
restored all six rows and retained the two selected files. Every expected result
was observed; see `user-workflow-filter-results.json`. The invalid regex reported
its actual unmatched-parenthesis error instead of silently returning no matches.

This case reused the successfully saved common dataset. Unicode search `猫`
therefore correctly matched the two MP3 comments as well as the FLAC (three
files); the other plan queries were unaffected by the earlier album/comment
edits. All six complete-file hashes stayed unchanged throughout filtering.
Native AX typing substituted a curly quote in the short invalid-regex input;
pasting the literal expression through the visible control verified the actual
Boost error. General typing substitutions remain a separate UX investigation.
UAT-07 PASS with this explicit input variation.

![Explicit native regular-expression error](user-workflow-filter-error.png)

## Extended tags: passed after a reproduced mixed-format fix

UAT-03 used an independent six-file copy. Native Extended tags displayed two
ARTIST and two MOOD values as separate controls. Added a third MOOD value,
created a two-line COMMENT, removed UAT_REMOVE, and swapped TEST_A/TEST_B names
without replacing their values. Keep changes left every file byte-identical.
Save/Reload then retained the exact three MOOD values, two ARTIST values, line
break, swapped values and deletion. Only the FLAC file changed; all six encoded
and decoded audio identities remained identical.

Selecting that FLAC with an MP3 and renaming MOOD initially failed with
`Duplicate field: ALBUM`. The dialog had separate rows for native `ALBUM` and
`album`, then uppercased both during validation. A separate test also exposed
that case-only differences in source keys could lose values during a rename.

The fix builds one row per ASCII-insensitive logical name and indexes each
file's actual spelling and original values. Two-pass resolution retains swaps,
per-file multivalues and untouched raw names. DATE/YEAR remain distinct, as do
custom names such as maß/MASS. Three focused regressions passed with the rest
of the 35 JavaScript tests. The native debug bundle rebuilt successfully.

The rebuilt dialog displayed one ALBUM row and opened at the first field;
previously initial Add field focus had scrolled it to the bottom. Renaming MOOD
to UAT_MOOD in the mixed selection created and saved exactly one FLAC draft.
The MP3 acquired no field or physical write. Reload retained all three values
under UAT_MOOD. Independent raw Vorbis inspection confirmed ARTIST, COMMENT,
TEST_A/TEST_B and every unrelated key/value, including native casing, stayed
intact. All five other complete files stayed unchanged; all six encoded-packet
and PCM hashes matched the original fixture baseline. UAT-03 PASS.

![Mixed-format duplicate-field error before the fix](user-workflow-extended-before.png)

![One logical field row and correct initial scroll after the fix](user-workflow-extended-after.png)

![Persisted renamed multivalues](user-workflow-extended-multivalues.png)

The metadata reader and writer do not yet use identical Unicode field-name
identity rules in every path; this checkpoint does not claim universal Unicode
custom-field equivalence. The focused reviewer found no introduced loss or swap
defect in these changes.

## Numbering: passed

UAT-04 used a fresh case. Clicking the visible Filename header switched the
table to descending order. The selected Night Shift/First Light files received
07/2 and 08/2 from start 7, two digits and total tracks enabled. Applying numbers left
all complete-file hashes unchanged until Save. Save and Reload persisted the
exact strings in visible order; independent ffprobe dictionaries differed only
in TRACK. Four unselected files and all six audio payloads were unchanged.
Cancelling another numbering dialog created no draft. UAT-04 PASS. This covers
the implemented basic numbering interface, not the missing disc-reset wizard.

![Saved numbering in descending order](user-workflow-numbering.png)

## Filename conversion: passed

UAT-05 saved Needs import as the two MP3 titles, then previewed
`%track% - %artist% - %title%`. The visible preview extracted artist Solar Arcade,
original titles and tracks 01/02 without extensions. Independent before/after
preview hashes proved all six files unchanged. Apply and Reload restored the
titles and wrote the extracted tracks. Independent probes confirmed every other
tag, all unselected complete files and all audio remained intact. The pattern
`NO MATCH %title%` produced an explicit error with Apply disabled. UAT-05 PASS.

![Reviewed filename extraction](user-workflow-import-preview.png)

![Explicit nonmatching-pattern error](user-workflow-import-error.png)

## Reviewed dynamic and counter renaming: passed

UAT-06 used separate fresh dynamic and counter copies. Previewing UAT-$uuid(),
then changing to UAT2-$uuid(), disabled Apply until a new preview. Apply created
exactly the final visible UUID destination, retained the extension, removed the
old source, and reconciled the selected row. Reload succeeded. The first preview
path was never created. A later preview was cancelled without another file.

The two-file counter pattern `$num(%_counter%,2) - $upper(%title%)` in ascending
Filename order previewed and applied exactly 01 - FIRST LIGHT.mp3 and
02 - NIGHT SHIFT.mp3. Independent receipts confirmed previews changed no bytes,
and all six complete-file contents and encoded/decoded audio remained identical
after each rename case. Exact reviewed destinations are in the results JSON.
UAT-06 PASS.

![Final reviewed UUID destination](user-workflow-rename-preview.png)

![The applied filename matches the review](user-workflow-rename-after.png)

## Quick regular-expression action: passed

UAT-08 selected First Light and applied Replace with Regular Expression to TITLE,
using lookbehind `(?<=First )Light` and replacement Glow. The dialog explicitly
stated immediate saving; First Glow persisted after Reload with no separate Save.
Option+Shift+6 opened Quick Actions on macOS. Applying invalid pattern `(` showed
the actual unmatched-parenthesis error. Cancelling it and another quick dialog
left all six complete-file hashes equal to their before-error checkpoint.
Independent ffprobe tags differed from the original only in the selected title;
the other five files and all audio remained unchanged. UAT-08 PASS.

![Explicit Quick Action regex error](user-workflow-quick-error.png)

UAT-09 through UAT-14 remain NOT RUN in this workflow suite. Earlier appearance
and menu checks are separately recorded in `branding-appearance.md` and
`menu-switch.md`; they are not substitutes for the remaining working-data cases.
Original format/preferences/converter/action parity remains open in the project
ledger and documentation inventory.
