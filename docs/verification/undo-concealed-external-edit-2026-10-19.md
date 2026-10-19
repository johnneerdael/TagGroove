# Undo and Redo: concealed external edits

Historical pre-repair diagnosis. The [L61 content-baseline repair](history-content-baselines-2026-10-19.md) supersedes the unsafe behavior below; these original observations and retained artifacts remain unchanged.

The current journal can overwrite an external audio edit when it preserves the
file's identity and size and restores the recorded modification time. This
confirms the existing L61 baseline gap; it does not supply a safeguard repair
or original-Mp3tag comparison.

## Actual current-port diagnosis

Eight cases called the public core writer and `Journal` using independently
created mono PCM WAV files. A recorded Save changed TITLE and preserved all
PCM. Undo and Redo were each tested against four conditions:

| Condition before replay | Undo result | Redo result |
| --- | --- | --- |
| No external edit | Exact original restored; opposite replay restores saved file | Exact saved file restored; opposite replay restores original |
| In-place PCM edit with a changed modification time | Refused; external bytes preserved | Refused; external bytes preserved |
| Replacement inode with equal size and restored modification time | Refused; external bytes preserved | Refused; external bytes preserved |
| In-place PCM edit with equal size and restored exact modification time | Accepted, one path and no error; original replaces external audio | Accepted, one path and no error; saved file replaces external audio |

In both concealed cases, the device, inode, length and exact nanosecond
modification time matched the recorded baseline. Independent whole-file and
bounded RIFF/data-chunk SHA-256 observations nevertheless differed: exactly one
PCM byte was changed in place, at data offset +30. Original, saved, baseline,
external, replayed and opposite-replay vectors were retained separately.

The concealed Undo case's next Redo recovered the external bytes exactly:
Undo captures its after-side snapshot lazily. After concealed Redo, the next
Undo restored the original file instead of the external edit. This proves
silent overwrite and that particular replay asymmetry. It does not prove
irreversible loss across every possible recovery route.

All six ordinary controls passed. Journal admission succeeded with the
unchanged defaults: 50 operations, 2 GiB retained data and the 2 GiB free-space
reserve. Compiler and runtime exited zero; that means the diagnostic completed,
not that the two concealed-edit cases met the safety expectation.

## Source cause and reachability

[`Fingerprint`](../../crates/tag-core/src/history.rs) records length,
modification time and platform file identity. Replay compares that metadata;
it has no recorded content digest for rejecting these equal-metadata edits.
Its staged restore uses the same fingerprint checks. The backend's
[history adapter](../../src-tauri/src/history.rs) delegates to the core
Undo/Redo methods before reloading descriptions. That is a source-bound route,
not an executed desktop UI journey.

Unix change time advanced in both concealed cases while birth time remained
equal. Those are observations, not a proven replacement safeguard. The
journal's own hard-link, unlink and restore activity can also change metadata;
any stronger policy must account for those effects, publication races and
large-file costs. No content-check or timestamp-policy change is approved or
implemented by this evidence.

## Retained evidence and limits

| Artifact | SHA-256 |
| --- | --- |
| Final diagnosis receipt | `8e39afa0ae4e1bc813ab189059900e4d4bf911bacba8d9bf676d6c83dfe74784` |
| 128-file manifest | `585c2344e98ccc257b7c669e56a9f18b42213c059b85cab129ee56d7892b8c26` |
| Raw runtime log | `f8a3120bf80f821d18bce7bbfd4d5fdafea81021ed6ae6c43a9af6013c0fa89f` |
| Executed probe | `295b6dfdba0f9c4f86ab38ae12d53e70eac1efa7d7f2079b78af8b5ca040c1fc` |
| Independent audit receipt | `f7e983c350e325f73409f47bcfd3cc048d8680db1e2054f89d4b668123e12676` |

The actual Cargo producer and retained library digest were verified, with all
49 source/build/lock bindings unchanged before and after execution. The
independent reviewer verified all 128 artifacts and independently walked and
hashed the WAV stages. Library SHA-256:
`178906aa466a16b75678b44bf944fa87ebf84990421a3c68f415dc9541dbfcde`.

The probe used an environment allowlist and OS network denial on the current
macOS local filesystem. It performed no Cargo rebuild, repository change,
original/native/Windows execution, provider operation or app GUI action.
Cross-session recovery and a universal external-change guarantee remain
unqualified. See the [current history guide](../user-guide/guides/undo-history.md)
and [L61](../research/remaining-gaps.md#l61).
