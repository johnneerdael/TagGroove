# Automation compatibility: first R05/R06 batch

> Current compatibility: [C57](native-mta-implementation-2026-10-19.md) adds
> canonical native cover/playlist action records and representation-aware warnings.
> Keep the dated app-only ID/warning observations below as the earlier baseline.


This records a bounded implementation against source baseline
`4a1a95ff4eddb9da7492aaace68197c4f81347d0`. It does not close R05 or R06.
Local native-core tests, simulated DOM interaction, original static evidence,
and original-runtime comparison are separate forms of evidence.

## Delivered behavior

- Find's Regular expression strategy calls the existing Boost bridge, compiling
  once for the complete filename/tag-value request. Literal strategies remain local.
- Native work runs on a blocking worker with two concurrent requests permitted.
  A pattern is limited to 64 KiB; one request to 100,000 values, 1 MiB per value
  and 64 MiB total. Validation/matching failure returns no partial results.
- Changing criteria/options, issuing a newer search or closing the dialog
  invalidates older responses. The visible table is cleared for changed criteria.
- Filter errors identify invalid selector format strings and MATCHES operands.
  Unfinished quotations identify their opening character and explain the fix.
- Action groups containing Adjust Cover, Set Cover Properties or Generate
  Playlists identify their exported `.mta` mappings as app-only extensions in
  the group view and export status. IDs 1001–1003 are unchanged.

## Copied-expression corpus

`src-tauri/src/find.rs` exercises these raw patterns against `Paper Boats`,
`paper boats`, `Paper Boats Live` and `Different`, through Find, filter MATCHES,
`$regexp` and the RegexReplace action:

| Pattern | Purpose |
| --- | --- |
| `(?i)\APaper\s+Boats\z` | Inline case override and whole-value anchors |
| `(?i)\APaper\h+Boats\z` | Boost horizontal-space escape |
| `(?<=Paper )Boats` | Lookbehind |
| `Paper\K Boats` | Reset the reported match start |

This is an independently authored TagGroove consistency corpus. It is not a
captured original-app result. Case defaults remain context-specific; Find's
checkbox, filter's ignored-case default and scripting/action flags are tested
separately. Web-source parser matching has a case-sensitive default and its
own quoting/escaping layer. The source engine's owner lane adds the independent
`boost_whole_subject_patterns_match_across_source_commands` fixture for
SayRegexp/RegexpReplace using the first pattern and invalid `(`; that separate
change is not included in this PR.

The native Find test also runs the six written examples from the regex guide,
including invalid syntax, through Boost. The JS tests assert the native batch
contract and invalid/stale-response handling; they do not imitate Boost with
JavaScript RegExp. The selected inline-option/anchor pattern is deliberately
invalid under JavaScript Unicode RegExp.

## Original static evidence for action files

Reference: Mp3tag 3.36.1 Windows x64, executable SHA-256
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The reusable private extraction is `TMPDIR/re/mp3tag-3.36.1-roadmap`.
No original execution or original-generated special-action fixture was available.

Seen in its extracted help map: `3000050` links to `actions/adjust-cover` and
`3000800` links to `actions/set-cover-properties`. Seen in offset-bearing
UTF-16 strings: `Adjust cover` at file offset `0xb6b85e`, `Set cover properties`
at `0xb6baae`, and `Playlist Generator` at `0xb6c66e`. These identify help/UI
surfaces, **not `.mta` type identifiers or serialized option semantics**.

The supplied setup's four shipped `data/actions/*.mta` files contain only
T=1 or T=2 records. They do not settle the three special mappings:

| File | SHA-256 | Observed T values |
| --- | --- | --- |
| CD-R.mta | `ccf92feca6bf8ea7606ba905ba36ee38627f21a101aa3af10e911e2467d79597` | 2 |
| Case conversion.mta | `9dcf4db6d968ef54c1906e22d28f3a0cfb75352526ef741546199af614684aac` | 1 |
| Schreibweise aendern.mta | `9dcf4db6d968ef54c1906e22d28f3a0cfb75352526ef741546199af614684aac` | 1 |
| Standard.mta | `e6df2a5c8782fa3c433191ef9185529476402a45f4276f71c7979b1edad6b4d7` | 2 |

No proprietary fixture or executable is added to the repository. Correct
original serialization remains unknown; no identifier is inferred from a help
ID or label. Existing unresolved imported records remain visible and blocked.

## Remaining qualification

- Capture the same corpus in the original app using disposable metadata files;
  include Find, filter, action, export and column evaluation contexts.
- Qualify `$caps` punctuation/explicit separators, field-presence versus literal
  truth, optional sections/nested quotes, zero division/unknown functions,
  Split Field empty pieces and multivalue output before changing semantics.
- Qualify already implemented `$columnNames`, `$columnValues` and the fourth
  `$list` argument in real column/export/action contexts. Do not reimplement them.
- Qualify mixed quoted/bare filter fragments against the original. The existing
  rejection and complete-operand quoting workaround are deliberately preserved.
- Generate original Adjust Cover, Set Cover Properties and Generate Playlists
  `.mta` fixtures covering every option, import them, export them and reopen in
  the original. Retain independently recorded expected properties and hashes.
- Confirm opaque-frame retention for Remove Fields Except before changing it.
- Validate Find with native input on each desktop platform, large real lists and
  multiline/Unicode metadata; a DOM test is not native OS-input evidence.

The existing `tests/e2e/core-workflows.js` suite now checks Boost-only Find
syntax, two fixture filenames, clearing old results after an edit and an
invalid-pattern error through the real native command. This native-window
suite is reserved for CI; it was not launched on the working desktop.

No release, installation, original-runtime equivalence, native OS-input result
or completed area-wide parity is claimed by this batch.
