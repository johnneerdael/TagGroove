# Native end-user workflow test plan

Status: **native execution started: UAT-01 PARTIAL; UAT-02 PASS after a reproduced metadata fix; UAT-03 through UAT-14 NOT RUN**. See [execution evidence](user-workflows-execution.md) and [measured results](user-workflows-results.json). Do not treat fixture generation or unit-test results as a passed user acceptance test.

Run against the release worktree at `/Users/jneerdael/Scripts/opentag-worktrees/brand-releases` after the appearance implementation/checkpoint is finished and the root agent authorizes the isolated native session. Do not use the current app or computer-use session while the interface agent owns it. No app build, launch, UI operation, preference access, or user-library access was performed during this preparation.

These cases exercise the visible OpenTag interface and actual file results. They are not an original-Mp3tag differential qualification. Current scope and missing interfaces remain in [PARITY.md](../PARITY.md). Primary expectations for mask extraction, filter expressions, and MTE rendering are documented at [filename imports](https://docs.mp3tag.de/converters/import-tags-from-filenames/), [filters](https://docs.mp3tag.de/filter/), and [exports](https://docs.mp3tag.de/export/); workflow-specific details below describe the current OpenTag surface.

## Disposable fixtures and receipts

All generated media live under `/tmp/opentag-user-test`; never choose a user music directory in a file dialog. The dependency-free Python fixture tool uses Python 3.9+ and installed `ffmpeg`/`ffprobe`. It authors an ordinary FLAC Vorbis-comment block directly to create real repeated values, leaving the encoded audio bytes intact. It does not depend on mutagen, the app's tag reader, or the app's scripting implementation.

```sh
python3 docs/verification/user-workflows-fixtures.py prepare
```

Preparation deliberately refuses to overwrite an existing baseline. The current prepared baseline is inventoried in [user-workflows-fixture-baseline.json](user-workflows-fixture-baseline.json). Full generation commands, independent probes, per-packet hashes and input hashes are under `/tmp/opentag-user-test/receipts`. The checked-in ledger records one observed fixture set; regeneration with a different codec/tool version may produce different bytes, so collect a new baseline rather than assuming hashes are portable.

| Fixture under `fixtures/library` | Actual audio | Initial tags used by the tests |
| --- | --- | --- |
| `Disc 1/01 - Solar Arcade - First Light.mp3` | 3 s, 220 Hz tone, stereo 44.1 kHz, MP3 | TITLE `First Light`, ARTIST `Solar Arcade`, ALBUM `Morning Signals`, TRACK `01/06`, DISC `1/2`, COMMENT `Original comment` |
| `Disc 1/02 - Solar Arcade - Night Shift.mp3` | 3 s, 330 Hz tone, stereo 44.1 kHz, MP3 | TITLE `Night Shift`, same artist/album, TRACK `02/06`, COMMENT `Second original` |
| `Disc 1/03 - Mémoire - Café 猫.flac` | 3 s, 440 Hz tone, stereo 48 kHz, FLAC | ARTIST values `Mémoire`, `Guest 猫`; MOOD values `Calm`, `Focus`; TEST_A `left`; TEST_B `right`; UAT_REMOVE `remove-me`; DATE `2020-04-18` |
| `Disc 2/04 - Solar Arcade - Quoted Title.m4a` | 3 s, 550 Hz tone, stereo 44.1 kHz, AAC/M4A | TITLE `Title, with "quotes"`, ARTIST `Solar Arcade`, ALBUM `Morning Signals`, TRACK `4/6` |
| `Disc 2/05 - Zephyr - Paper Planes.opus` | 3 s, 660 Hz tone, mono 48 kHz, Opus | TITLE `Paper Paper Planes`, ARTIST `Zephyr`, ALBUM `Open Sky` |
| `Disc 2/06 - Fixture Lab - Sanity Pulse.wav` | 3 s, 770 Hz tone, mono 44.1 kHz, PCM WAV | TITLE `Sanity Pulse`, ARTIST `Fixture Lab`, ALBUM `Diagnostics` |

Other fixture directories contain:

- `art/front-600.png`: independently probed PNG, 600×600; `art/back-800x450.jpg`: JPEG, 800×450. These are synthetic colored images, not user artwork.
- `text/lyrics-utf8.txt`: multiline Unicode; positional UTF-8 and BOM UTF-16 import text; malformed and Windows-1252 text for optional negative/future-interface coverage.
- `exports/group-boundary.mte`: filename/comment rows; `exports/summary-csv.mte`: explicit quoted/escaped title export. Both are tiny independently authored templates.
- `actions/UAT-A-uppercase-comment.mta` and `UAT-B-prefix-comment.mta`: supported Format Value actions. `UAT-unresolved.mta` contains an intentionally unknown action ID.
- `identifier-only.json`: exactly `{"identifier":"org.opentag.editor.usertest"}`.

Start each case from an independent copy, then launch a fresh isolated app session so previous in-memory file rows cannot contaminate the count:

```sh
python3 docs/verification/user-workflows-fixtures.py reset common
```

Use the case slug specified below instead of `common`. Files appear under `/tmp/opentag-user-test/work/<case>/library`; write outputs only under that case's `outputs` directory. `reset` refuses an existing case unless explicitly passed `--force`; use that only after closing the case in the app. It never resets the immutable fixture baseline or user-library files.

Capture checkpoints before and after UI operations:

```sh
python3 docs/verification/user-workflows-fixtures.py receipt common pending
python3 docs/verification/user-workflows-fixtures.py receipt common saved-reloaded
```

The tool records file SHA256, ordered encoded-audio packet SHA256/count/bytes, decoded signed-16-bit PCM SHA256, ffprobe metadata/streams, picture-packet hashes, and actual repeated FLAC comments. It inventories output files separately. Comparisons identify renamed audio by its unique decoded-payload hash instead of assuming that the old filename still exists. Any missing file, changed PCM, or changed encoded-audio packet set requires investigation; tag edits should preserve audio. Whole-file hashes are expected to change for real metadata/cover edits, and remain unchanged for preview, search, theme changes, export, discard and simple rename.

Preparation preflight: a `preflight` working copy was created and independently re-probed. All six complete file hashes, encoded-audio packet hashes and decoded PCM hashes matched. See `/tmp/opentag-user-test/receipts/preflight/untouched-comparison.json`. This checks the fixture/receipt mechanism only, not OpenTag.

## Isolated native build

After the appearance owner finishes, root must approve and hand over the native app/CUA session. Record the source commit, dirty-source state, build command, app executable hash and config override in the eventual execution receipt. Build the same functional source with only the application identifier changed, using a separate target directory so this does not replace the currently running appearance-test build:

```sh
CARGO_TARGET_DIR=/tmp/opentag-user-test/target npm run tauri -- build --config /tmp/opentag-user-test/identifier-only.json
```

No version, product name, application logic, feature flag or functionality setting changes. The identifier-only difference must be explicit in the report; this is not byte-identical to a public release package. Before launch, verify the built `Info.plist` contains `org.opentag.editor.usertest`, then explicitly launch that bundle rather than locating an arbitrary app called OpenTag:

```sh
plutil -extract CFBundleIdentifier raw -o - /tmp/opentag-user-test/target/release/bundle/macos/OpenTag.app/Contents/Info.plist
open -n /tmp/opentag-user-test/target/release/bundle/macos/OpenTag.app
```

Select the test app by its bundle identifier in computer use. Confirm its action/config/WebKit storage is separate from `org.opentag.editor` before exercising settings. A fresh profile should have no previously configured action groups. On macOS the expected app config directory is `~/Library/Application Support/org.opentag.editor.usertest`; confirm the actual resolved path instead of assuming it. Record only hashes/metadata of any existing regular-profile configuration before/after; do not expose its contents, modify its files, clear its storage, or launch a user music library. If storage isolation cannot be established, stop the native test and report that specific precondition as unmet.

Do not change the system-wide appearance to test System mode. Record the system's existing appearance, verify the app resolves System accordingly, and qualify OS-change propagation separately if it is not exercised.

## Cases and expected user-visible/file results

Case status is recorded in the execution evidence linked above. Use visible native controls and dialogs. Do not replace user workflows with direct IPC calls, DOM state injection, generated screenshots, or implementation-mirroring assertions.

### UAT-01 — Add files, add folder, deduplicate and inspect (case `discovery`)

1. From the empty test app, choose **Add files…** and select the two MP3s in `Disc 1`. Expect two rows, readable common tags and approximately `0:03` durations.
2. Choose **Open folder…** and select this case's `library` directory. Expect six unique rows, including nested `Disc 2`, Unicode names, FLAC, M4A, Opus and WAV.
3. Add the same MP3s again. Expect six rows, not eight. Sort Filename both ways, select individual rows, then select all visible rows from the table.
4. Verify codec/sample-rate/channel information against the independent baseline probes, not inferred extensions alone. Cancel another file picker and confirm the list stays unchanged.

Evidence: empty/loaded screenshots, six-row inventory/selection count, before/after receipts with every complete file hash unchanged. A decoder/read error is a failure to investigate, not a reason to silently omit a fixture.

### UAT-02 — Common edits are drafts, then Save and Reload (case `common`)

1. Open the library, select only the two MP3s. Confirm their shared album is visible and differing comments are represented as mixed values.
2. Change ALBUM to `UAT Shared Album` and COMMENT to `UAT common — Café 猫`. Before Save, capture `pending`: both rows should be dirty, while every on-disk file hash still matches its before receipt. Operations requiring saved state should be disabled with an understandable reason.
3. Click **Save tags**, then **Reload tags**. Expect both values to persist, no remaining dirty count, no file errors, and no discard prompt on a clean reload.
4. Independently inspect both MP3s. Only these two files should have metadata changes; all six audio packet/PCM identities must be preserved.

Evidence: mixed panel, pending state, saved/reloaded state, `pending` and `saved-reloaded` receipts with tag values and unchanged unselected files.

### UAT-03 — Extended multivalues, rename swap, removal and mixed preservation (case `extended`)

1. Select the FLAC and open **Extended tags…**. Confirm ARTIST has two actual values and MOOD has two; these must not appear as one separator-joined value.
2. Add a third MOOD value `Late Night`; add multiline COMMENT `First line` / `Second line`; remove UAT_REMOVE. Simultaneously change TEST_A's name to TEST_B and TEST_B's name to TEST_A without replacing either value.
3. Choose **Keep changes**. Capture `pending`: no disk hash may change yet. Save and Reload. Verify three MOOD values, the multiline comment, absent UAT_REMOVE, TEST_A=`right`, TEST_B=`left`, and both original ARTIST values. Inspect raw repeated Vorbis comments independently.
4. Select the FLAC and first MP3 together. In Extended tags, rename MOOD to UAT_MOOD without editing its mixed/hidden values. Keep, Save and Reload. FLAC must retain all three values under UAT_MOOD; the MP3 must not acquire fabricated empty/mixed text or new values.

Evidence: value controls and field names before/after, `pending`, `saved-reloaded` and `mixed-rename` receipts. No value loss, unintended normalization of other fields, or writes to unrelated files.

### UAT-04 — Number selected files in visible order, with draft state (case `numbering`)

1. Sort Filename descending and select the two MP3s, with Night Shift preceding First Light in the table.
2. Open **Auto-numbering…**; choose start `7`, minimum digits `2`, and include total tracks. Apply.
3. Expect Night Shift TRACK=`07/2`, First Light TRACK=`08/2`, pending status and unchanged disk hashes. Save and Reload, then independently verify the compound track strings.
4. Unselected track/disc/date values must stay unchanged. Cancel another numbering dialog and confirm it creates no new draft.

This exercises the implemented basic numbering UI; disc-reset wizard and original offset/count semantics remain outside its current parity claim.

### UAT-05 — Filename → Tag preview and apply (case `filename-tags`)

1. Set both MP3 TITLE fields to `Needs import` through the panel and Save, making the conversion effect observable.
2. Choose **Filename → Tag…**, enter `%track% - %artist% - %title%`, and Preview. Expect track `01`/`02`, ARTIST `Solar Arcade`, and the original First Light/Night Shift titles, without `.mp3` inside TITLE.
3. Capture the preview and `preview` receipt; preview must not alter files. Apply, Reload and inspect independent tags. Album and unrelated fields stay unchanged.
4. Try a nonmatching mask on a fresh selection. Expect an explicit error and no partially applied conversion; capture the actual error/state rather than treating an empty preview as success.

### UAT-06 — Dynamic rename receipt matches the exact applied destination (case `rename-dynamic`)

1. Select the first MP3, open **Tag → Filename…**, and enter `UAT-$uuid()`. Preview and record the full actual destination shown, including extension, in the execution receipt.
2. Change the template to `UAT2-$uuid()`. Apply must be unavailable until another Preview. Run Preview again and record that final destination.
3. Apply once. The actual file must exist at exactly the final reviewed destination; it must not use another freshly evaluated UUID. The original source path must no longer exist, and the selected row/path must reconcile correctly.
4. Reload the renamed file. Tags, exact complete-file bytes, encoded packets and PCM must be unchanged. Cancel another rename preview and confirm no extra file appears.

Use a separate fresh `rename-counter` case for a two-file preview with `$num(%_counter%,2) - $upper(%title%)` in ascending Filename order. Expect `01 - FIRST LIGHT.mp3` and `02 - NIGHT SHIFT.mp3`; both applied destinations must match their preview rows.

### UAT-07 — Search, syntax errors and Boost regular expressions (case `filter`)

Open all six baseline files. Enter each query using the real Filter control and wait for filtering to settle:

| Query | Expected visible files |
| --- | --- |
| `猫` | FLAC only |
| `ARTIST IS "Solar Arcade"` | Two MP3s and M4A: three |
| `ARTIST IS "Solar Arcade" AND TITLE HAS Light` | First Light only |
| `NOT ARTIST IS "Solar Arcade"` | FLAC, Opus and WAV: three |
| `"$upper(%artist%)" IS "SOLAR ARCADE"` | Two MP3s and M4A: three |
| `TITLE MATCHES "(?<=First )Light"` | First Light only; lookbehind must execute |
| `TITLE MATCHES "\b(Paper)\s+\1\b"` | Opus only; pattern backreference must execute |
| `ARTIST IS "Solar Arcade" AND (` | Explicit invalid-filter feedback |
| `TITLE MATCHES "("` | Explicit regex error, not silent “no matches” |

Clear the filter and expect all six rows. Rapidly replace a valid query with an invalid one, then clear it; stale asynchronous results must not overwrite the final unfiltered list. Selection identities and file bytes must not change. Capture valid/invalid/cleared screenshots and an unchanged file receipt.

### UAT-08 — Quick action success and invalid-regex error (case `quick-action`)

1. Select First Light, choose **Actions (Quick)…**, and select **Replace with Regular Expression**. Field TITLE; pattern `(?<=First )Light`; replacement `Glow`.
2. Apply and Reload. Expect TITLE `First Glow`, selected file only, with its original filename and unchanged audio payload. Observe whether the operation writes immediately; do not claim a draft/save cycle if none occurs.
3. Repeat with pattern `(`. Expect an explicit error and no additional file modification. Cancel another Quick action and confirm no mutation.

### UAT-09 — Ordered groups, persistence and deferred MTE at group boundaries (case `groups`)

1. Import `UAT-A-uppercase-comment.mta` and `UAT-B-prefix-comment.mta` through **Actions… → Import .mta…**. These use a supported Format Value schema and should not import unresolved actions.
2. Edit group A. Add an **Export** action using `fixtures/exports/group-boundary.mte`, output `/tmp/opentag-user-test/work/groups/outputs/group-a.txt`, append off, partition off. Move Export **before** A's Format Value action. Save group changes.
3. Enable both groups in order A then B; select the two MP3s and apply. A's Export must run after A's field actions even though its row precedes them. The output must contain `First Light.mp3|FIRST LIGHT` and `Night Shift.mp3|NIGHT SHIFT`, using the full actual fixture basenames in the first column.
4. Final saved COMMENT fields must be `B:FIRST LIGHT` and `B:NIGHT SHIFT`. The output must not contain `Original comment` or the later group's `B:` prefix. This distinguishes deferred execution from immediate execution and from deferral until the end of every group.
5. Close and reopen the isolated app, then Actions. Confirm group contents, enabled state/order and saved selection preset survive. A preset should change which groups are selected without replacing their action contents.
6. If time permits, repeat with fresh media and reversed group order, updating the output to that fresh case. B then A should end with uppercase titles as comments, without `B:`. Record both actual order lists and results.

Optional negative: import `UAT-unresolved.mta`. Expect preserved unresolved settings and blocked execution, not guessing action 999 or changing files. Do not import/execute unreviewed user action libraries.

### UAT-10 — Add, inspect, change properties and remove covers (case `covers`)

1. Select First Light. Its initial cover state is empty. Choose **Add…** and select `art/front-600.png`. Reload; expect one PNG cover, 600×600, with actual picture-packet bytes matching the input PNG hash.
2. Through Quick action **Set Cover Properties**, set type Back Cover and description `UAT back — 猫` (disable keep-existing-description). Reload and verify both properties independently from the embedded picture metadata.
3. Add `back-800x450.jpg` as a second cover. Verify two selectable covers and distinguish JPEG 800×450 from the retained PNG. Select the JPEG and Remove; the PNG/back-cover description must remain intact.
4. Remove the last cover and Reload. Expect no cover. Rewritten file bytes need not equal their original padding/layout, but all audio packet/PCM hashes must still match and unrelated tags must survive.

Capture screenshots/probes after add, property edit, one removal and final removal. An empty thumbnail is not proof that embedded bytes were removed; use the independent stream/picture receipt.

### UAT-11 — Export CSV and M3U8 from the selected order (case `exports`)

1. Select First Light, the FLAC, and the quoted-title M4A in ascending Filename order. Choose **Export CSV…** and save `outputs/tags.csv`; choose **Create playlist…** and save `outputs/list.m3u8`.
2. Independently read the CSV as UTF-8 with delimiter `;`. Expect the fixed eight-column header TITLE/ARTIST/ALBUM/TRACK/YEAR/GENRE/COMMENT/PATH and exactly three records. The M4A title must decode to `Title, with "quotes"` as one cell; the Unicode values must survive, and FLAC artists must not disappear. Record native-field/format differences rather than inventing missing aliases.
3. Read M3U8 as UTF-8. Expect `#EXTM3U`, three `#EXTINF` records in selected table order, and exactly three media paths. Resolve each path relative to `outputs` and prove it identifies the selected actual file, including Unicode names.
4. Verify all six audio files are byte-for-byte unchanged and output files have recorded hashes. Cancel another export picker and confirm no new output appears.

Optional overwrite guard: only if the real save picker allows an exact existing audio destination, attempt it on a disposable fixture and expect rejection before audio mutation. If extension filtering prevents the attempt, record UI prevention rather than bypassing the picker through IPC.

### UAT-12 — Unsaved close, cancel and discard (case `unsaved-close`)

1. Draft a TITLE edit `Never write this` on First Light; do not Save. Attempt to close the isolated window. Expect an understandable pending-change warning.
2. Cancel close. Expect the same file selection and pending value, and unchanged on-disk bytes. Capture that state.
3. Close again and explicitly discard. Reopen the isolated app and reload the same fixture. Expect the original TITLE, no saved discarded text, and all complete file hashes unchanged.
4. Separately make an unsaved action-group edit and attempt window close. Confirm that unsaved action changes are also represented in the guard. Record any OS/modal limitation instead of claiming a branch that was not exercised.

### UAT-13 — System/Light/Dark preserve working state (case `appearance-state`)

1. Open all six files, sort Filename descending, select the two MP3s, enter a valid filter retaining both, and draft COMMENT `Theme must preserve this` without saving. Record selected filenames/order/filter/dirty count/value and the before receipt.
2. Select Dark, then Light using the app's visible appearance control. In each mode verify table/panel text, controls, disabled states, errors and artwork placeholders remain legible; inspect the saved draft and selection after each switch.
3. Open and cancel Extended tags and an invalid conversion/filter state in each mode to check actual dialogs/errors. Do not try to click a background theme control through a modal or inject app state.
4. Capture two native screenshots showing the same working content in Light and Dark. All six complete file hashes must remain unchanged. Discard the test draft explicitly.
5. Set one explicit appearance, close/reopen the isolated app, and confirm that preference persists in the test profile. Select System and verify it resolves to the already existing OS appearance. Do not claim live OS-change propagation unless that was separately exercised.

### UAT-14 — Optional multiline text import action (case `text-action`)

Through Quick **Import Text File**, import `fixtures/text/lyrics-utf8.txt` into LYRICS for one MP3. Reload and inspect exact Unicode and line breaks. Other fields/audio must survive. The public release UI does not yet expose the standalone Text File → Tag positional converter: do not label its UTF-8/UTF-16/malformed fixture files as a passed UI converter test. Record that interface as not present; the fixtures are available when it is wired.

## Execution receipt and completion decision

For each case, record case ID, source/build identity, identifier override, exact native actions, expected result, actual result, screenshot paths, before/after receipt paths, and a status: PASS, FAIL, NOT RUN or NOT PRESENT. Include exact error text and the first failing step. Keep generated screenshots/logs under this fixture root during execution; only deliberately selected proof artifacts belong in the repository.

Any lost multivalue, unselected-file mutation, changed audio payload, preview/apply destination mismatch, silent invalid expression, overwritten input audio, or unresolved test-profile isolation blocks workflow completion. Report concrete failures to root for a scoped fix, then rerun the affected case on a fresh copy. A successful build, screenshot, or test helper is not sufficient evidence for a native workflow pass.

This preparation is complete when the six files/images/text inputs and baseline receipts exist, the isolated-profile procedure is reviewable, and root can take over the actual native session. The user-workflow objective itself remains incomplete until the authorized cases are executed and their evidence is assessed.
