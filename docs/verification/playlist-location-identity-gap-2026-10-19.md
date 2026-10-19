# Playlist location identity: current writer/readback gap

TagGroove's existing playlist writer and reader can lose or substitute local
members. This is a demonstrated implementation defect, separate from the
implemented Refresh/source/manual-order behavior recorded under C25. No repair
or original-Mp3tag comparison is claimed here.

## Executed diagnosis

An external probe called the current `render_playlist` and `load_playlist`
implementations using the retained tag-core library. It created 12 owned WAV
recordings with deliberately distinct PCM, including pairs whose names differ
only by leading or trailing spaces. The expected member order and output
bytes were prepared independently of the returned list.

Across 42 writer/readback cases:

- Eight relative M3U/M3U8 cases omitted a member whose location began with `#`,
  including `#Intro.wav` and `#Album/Track.wav`, with extended output on and off.
  The writer emitted those names verbatim and the reader treated them as
  comments. The returned missing-file list was empty.
- Twelve M3U/M3U8/PLS cases substituted an existing trimmed-name sibling for
  the intended space-bearing filename. Independent device/inode observations
  and walked RIFF data-chunk hashes distinguish the recordings; this was not
  merely a different spelling of the same file. The returned missing-file
  list was again empty.
- Sixteen ordinary, absolute-path and PLS hash-name controls retained their
  expected members. Six additional URI-like and literal-percent observations
  also retained the expected local identities.

Six separate newline-name refusal controls kept their pre-existing output
sentinels unchanged. All source WAV bytes and empty tag views remained
unchanged. The probe bound 49 source/runtime inputs and retained the compiler
output, executable, raw log, source and result JSON outside Git.

The root review rechecked all 76 artifact hashes in the receipt and all 42
result records. Twenty result records have an incorrect member list, matching
the eight omissions and twelve substitutions above.

## Expanded unmodified-product baseline

A fresh diagnostic linked the unchanged retained production library and ran the
frozen 68-case playlist probe: the original prepared 22 cases plus 21 portable
and 25 Unix physical edge cases. It recorded **52 proposed-policy mismatch flags
and 16 matching controls**, not 52 distinct native bugs. No playlist correction
was applied or approved. The earlier original 42+6 proof was preserved and was
not rerun by this cohort.

Compilation and execution exited zero. Fixture creation, distinct device/inode
identity, source-byte preservation and empty-metadata assertions passed; all 66
owned media files and raw logs were retained. URI grammar/refusal controls,
including checks that remote URI refusal precedes local existence callbacks,
remained matching controls. The physical-name result qualifies the current
Unix/macOS filesystem; it does not qualify Windows trailing-space aliases.
Eighteen source bindings and the retained library were verified before and after
execution under an environment allowlist and OS deny-network sandbox. No Cargo,
product edit, original application execution or provider request occurred.

Final receipt SHA-256:
`42ebfbdd203dc2407239266d56594dd7f4da2bf29cc24f40ebe6f8f19e99b42d`.
Final manifest SHA-256:
`595e926c09d4bf3a2f88860832cbc6eff06081c74cd6285366603a36851ffeb5`.

## Cause and bounded repair proposal

`parse_entries` trims location lines and PLS values; `entry_candidates_diagnostic`
also trims before trying candidates. That can prioritize a different local
file. The writer does not disambiguate relative comment-looking M3U members.

The proposed repair preserves location data, resolves the exact spelling before
legacy compatibility alternatives, and uses a relative `./` prefix where
playlist syntax would otherwise consume a literal filename. URI refusal,
legacy whitespace fallbacks and newline refusal must retain independent
controls. The expanded baseline above covers BOM grammar, whitespace-only names and
quoted CUE resolution. It measures the unchanged product against the proposed
policy; it supplies no repair or passing-conformance evidence. XML uses a separate reader pipeline and is outside
this proposed repair.

## Evidence and limits

| Artifact | SHA-256 |
| --- | --- |
| Final diagnosis receipt | `7f03fbe20a84b7e31c82ea1e1e4f65a93762ad366a8d5953c7df89f85fb0cd62` |
| Results | `98b9dd0b5c4c3667db58fe5754a883874ba69b2be9794e5ac9d99e8f360af1e9` |
| Raw probe log | `342413cde89d0cf7773f45c68a6c6e6f59efabb64f5e30ca6f0da4f392e115ec` |
| Executed probe | `12f67686af0db6005b6ff235f4af13d87b003628d71a392bdfe01d137d317f4d` |
| Linked tag-core library | `178906aa466a16b75678b44bf944fa87ebf84990421a3c68f415dc9541dbfcde` |

The execution used local owned media with network denied and credentials
omitted. It exercised pure playlist rendering and reading, not an application
writer/journal transaction; no Undo proof follows from it. Native pickers,
original Mp3tag, Windows filesystem aliases, device behavior and player
compatibility remain unqualified. The unchanged production journal reserve
remained 2 GiB. See the [active remainder](../PARITY.md#uncovered-work) and
[C25's implemented order scope](../research/remaining-gaps.md#c25).
