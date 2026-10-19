# Filter quoting and precedence: native static and public-port comparison

The five demonstrated quoting differences below are preserved baseline results. C55 now implements the correction and all 12 frozen vectors match; see [current local evidence](local-parity-closeout-2026-10-19.md#filter-quoting-c55-l11).

## Preserved baseline

This comparison uses the pinned Windows x64 Mp3tag 3.36.1 executable, SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`, and the current TagGroove public `compile_filter`/`matches` API. **Mp3tag was not executed.** Its results below are predictions from the native parser and predicate control flow, not original GUI observations.

## Evidence and execution

Native raw views, instructions, the decoded operator table and the coherent `37-evidence.md` note are retained in `/tmp/taggroove-native-filter-zmem7_pc`. Exactly 12 literal vectors were frozen before the port run; `native-vectors.json` SHA-256 is `d53bea8a3ae4cdcef666f4663b434bfbbd42af30d2f93d7b017756538a92b5ac`. The final `41-manifest.json` binds 44 retained artifacts and has SHA-256 `e1bebe97ffb4224ef6ba3c5f1b0e1dbd840543b405c8a2a11ff08a44a3dfc3c7`; root independently verified every bound size and hash.

The finite public-port probe is retained in `/var/folders/cm/m92k6_qs115046xzbll7hfkh0000gn/T/taggroove-columns-filter-audit-cl_cmsft/filter-public-probe-0cd3u7bq`. Receipt SHA-256: `e068ae834636df9a0eda88c5ffd26789d708df9fb18d18806bba4ca2452c9358`. The source-bound core library SHA-256 is `178906aa466a16b75678b44bf944fa87ebf84990421a3c68f415dc9541dbfcde`; all 49 producer-source bindings remained unchanged. Compiler and the single probe invocation both exited 0, with exact current-port assertions passing. `probe.log` SHA-256 is `f73b97f6f5d933dd1049909bdd271dfdadb72f7f46f41e080de5971e5da6eabc`.

There was no Cargo rebuild, audio write, provider request, native application launch or repository code change. Probe exit 0 means the diagnostic ran successfully; it does not mean all native predictions match.

## Closed native route

The visible-filter Apply route `0e131c` reaches the worker launcher, its jump stub `0df8c4`, worker `0dfb0c`, compile wrapper `0df920`, parser `27b130` and dispatcher `27a7cc`. This joins the inspected parser to application filtering rather than relying on similarly named strings.

The operator table at `140ac6280` selects OR before AND before NOT as outer splits; recursive child parsing yields **NOT > AND > OR**. Constructor/vtable paths join those nodes to actual short-circuit boolean evaluators. Parentheses cause recursive grouped parsing. The current port implements the same grouping.

For plain queries, operand reader `2795d4` treats a leading quote differently from a quote embedded in bare text. A leading quoted fragment becomes one phrase needle and leaves a trailing bare suffix unchecked. Bare text retains embedded quote characters and splits into word needles. Plain matching joins field values and filename-with-extension using vertical-tab separators, so a quoted space phrase does not match two words spread across fields. HAS uses the same operand reader. Unterminated leading quotes and missing HAS operands throw parser exceptions; their UI presentation remains unobserved.

## Twelve bounded cases

All contexts use filename `song.mp3`, with no hidden extra tag values.

| Case | Context and query | Native static prediction | Actual TagGroove result |
| --- | --- | --- | --- |
| 1 | TITLE=`blue`, ARTIST=`sky`; `blue sky` | Matches | Matches |
| 2 | Same fields; `"blue sky"` | Does not match: one phrase | Matches: words may span fields |
| 3 | TITLE=`blue`; `"blue" sky` | Matches: suffix ignored | Compile refusal |
| 4 | TITLE=`blue sky`; `blue "sky"` | Does not match: quote bytes literal | Compile refusal |
| 5 | TITLE=`blue`; `TITLE HAS "blue" sky` | Matches: suffix ignored | Compile refusal |
| 6 | TITLE=`blue sky`; `TITLE HAS blue "sky"` | Does not match: quote bytes literal | Compile refusal |
| 7 | A=1/B=0/C=0; `A IS 1 OR B IS 1 AND C IS 1` | Matches | Matches |
| 8 | Same fields; `(A IS 1 OR B IS 1) AND C IS 1` | Does not match | Does not match |
| 9 | Same fields; `A IS 1 OR (B IS 1 AND C IS 1)` | Matches | Matches |
| 10 | Same fields; `NOT A IS 1 AND B IS 1` | Does not match | Does not match |
| 11 | `"blue` | Parser failure | Compile refusal |
| 12 | `TITLE HAS` | Parser failure | Compile refusal |

L11 now retains five concrete quoting differences. Simply concatenating fragments or accepting every failed parse as literal text would not reproduce the traced rules and could remove useful syntax failures; no repair is implemented here.

L12 leaves implementation intake: its four precedence/grouping controls match existing code, and this audit establishes no missing locale implementation. C49 records that implemented scope. Original runtime, locale/collation, broader grammar and UI/error-presentation acceptance remain in the qualification register. This is not full filter parity.
