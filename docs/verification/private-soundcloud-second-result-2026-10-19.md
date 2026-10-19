# Private SoundCloud second-result selection — 2026-10-19

The unchanged supplied SoundCloud source now has an executed second-search-hit
qualification through the existing
[private public-acquisition harness](private-source-public-acquisition-2026-10-19.md).
Independently invented external HTML runs through its actual INDEX JSON loop,
production result selection, scalar ALBUM parser, public apply and physical
WAV/PNG history. No repository code or supplied script was changed for this case.
The earlier [Traxsource TRACK Search case](private-track-search-2026-10-19.md)
remains separate evidence.

## Independent inputs and selection

The prewritten index oracle contains two columns, `TITLE` and `UPLOADER`, and
two complete rows:

| Index | TITLE | UPLOADER | Invented result URL |
| --- | --- | --- | --- |
| 0 | Offline Decoy | decoy-maker | `https://soundcloud.com/decoy-maker/offline-decoy` |
| 1 | Evening Beacon | beacon-maker | `https://soundcloud.com/beacon-maker/evening-beacon` |

Search values are `Beacon Maker` and `Evening Beacon`. The invented index keeps
the wrapper, root-relative links and whitespace expected by the supplied
parser; it is not a recorded provider response. The actual parser returns both
rows. Production `TagSourcesUI` checks every rendered cell, clicks the uniquely
URL-bound second row, verifies its selected state and activates Next. The Rust
bridge independently requires that second release URL before looking up any
release body. The reported `selected_result_index` is `1`.

The selected page independently expects these scalar fields:

| Field | Expected value |
| --- | --- |
| TITLE | Evening Beacon |
| ARTIST | Beacon Maker |
| YEAR | 2025 |
| RELEASETIME | 2025-02-03 |

Its release date is distinct from an invented 1999 creation-date decoy. Its
expected artwork address is also distinct from an avatar-image decoy. The
release oracle checks the complete field set and artwork URL before review.
The separate album title is empty and release artist is `Beacon Maker`.

The ALBUM route still produces **zero `SourceTrack` records**. A duration
property present in the invented page is not consumed by this supplied script.
Two search results are separate selectable releases; two destination WAVs do
not turn this scalar result into a private track-list or duration proof.

## Physical save and refusal controls

Two WAVs are selected in reversed order, with a third unselected WAV sentinel.
Only TITLE is checked; unchecked fields remain unchanged. The normal public
apply handler embeds a controlled generated PNG, replaces the sidecar and uses
the existing journal. Physical reload, all three distinct PCM ranges, the
unselected file, whole-file Undo/Redo and sidecar restoration pass. An in-memory
PCM mutation is detected without changing disk or history. Current-operation
Undo admission succeeds with label `Tag source import` and two audio files;
the production 2 GiB reserve is unchanged.

The positive case consumes four exact logical fixture responses: index,
selected release, image preview and apply image. It reports zero network
requests. Existing acquisition refusals also pass, including authorization,
unexpected URL/method, status/size, cancellation and invalid-image controls.

Four independently wrong expectations or payloads reach their intended guards:

| Control | Exact guard | Observed boundary |
| --- | --- | --- |
| Wrong expected second-row title | `independent index oracle` | One body consumed; no release body consumed. |
| Select the first result URL | `release oracle` | One body consumed; release lookup does not change transport counts. |
| Wrong expected scalar TITLE | `independent album oracle` | Two bodies consumed. |
| Substitute unreturned artwork URL in apply | `pre-write field/artwork oracle` | Writer is not called; all four protected files remain unchanged. |

Each deliberate control exits with the intended failure and no success
receipt. All retain exact WAV/sidecar/history state, which also preserves PCM.
The three source-oracle refusal receipts explicitly report reaped children.
The artwork receipt reports four protected files, not a consumed-body count;
it uses the existing pre-write bridge cleanup path. No arbitrary timeout or
unrelated failure is accepted as proof.

## Frozen evidence and limits

The current backend compile-only check passes. Its immutable APFS clone has
SHA-256
`18fa1f853264d5dc77c174cbe4d6e75d8988e9d28348d2b39a9bb1672f2ef224`,
bound to 257 unchanged compile/runtime inputs, including the current MPEG
policy. The first compile left free space below the reserve, so execution was
held before any parser/journal test. After the parent audited and reclaimed
inactive codegen objects, the five cases ran once using that same clone, with
no rebuild. Compile and hold receipts remain unchanged.

The positive manifest binds 31 inputs, including the earlier 27 original
inputs. All original/prepared inputs, source bindings and the clone hash are
verified unchanged before and after every case. Pages, private sources,
settings, manifests and literal expected oracles remain outside Git.
Final receipt SHA-256:
`c4cb099af3e81f3893125ebb111f91f1de1346c50d6f5a445db81cfc04b9b072`.

| Retained raw log | SHA-256 |
| --- | --- |
| `second-positive.log` | `ed451f496728db9a17888b4f6668219d5cfb3733661cb6fa47c91922cc9bc8d1` |
| `wrong-index.log` | `b3286458c50ded186ccf871ba6bc8284964dc7b03e127a5f195395ea84c4325d` |
| `wrong-selected-url.log` | `3e0e1e34b46f3d9bd22cf57a90b7585f2ca4d1fda61b8fad7a2dde5c1a992956` |
| `wrong-release-field.log` | `13615370cda19908d77dceea2b3450c30edb223c0d01427af2672286d212b354` |
| `wrong-artwork.log` | `8676b369033eb992cf14cb533c7c14c409447ef7d5e9e03e32f16fef22837548` |

All execution uses a sanitized credential-free environment and OS network
denial. The fixture transport follows normal request construction,
authorization and dispatch, but does not exercise HTTP/TLS, redirects or live
provider acquisition. Native Tauri/configuration lookup, original Mp3tag,
Windows/devices and genuine private track-list acceptance remain unqualified.
The session-wide URL registry is not a newly claimed source-specific grant.
