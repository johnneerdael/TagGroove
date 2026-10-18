# Guide reference-detail corrections — 2026-10-18

Scope: three concrete counterexamples found while auditing consolidated draft PR64's guide against current implementation. The 189 written-page count does not establish complete detail coverage, accurate teaching or original-runtime equivalence.

| Reference | Correction and practical consequence | Evidence |
| --- | --- | --- |
| [Format strings](../user-guide/reference/format-strings.md) | External tool parameters use their separate seven path placeholders; general scripting placeholders/functions do not apply there. Link to the tool-specific syntax instead of promising a universal evaluator. | Pure tool-parameter check passed; current tool formatter contract inspected. |
| [Regular expressions](../user-guide/reference/regular-expressions.md) | `_TAG` confines replacement to tag fields; `_ALL` also includes the filename including extension. Distinguish `_FILENAME` and `_DIRECTORY`; avoid a recipe that silently renames files when the intent is cleaning tags. | Seven action-guide tests passed; action implementation and action-specific guide inspected. |
| [Web-source JSON](../user-guide/tag-sources/web-source-reference.md) | Dotted array indices are zero-based; `json_select_array` uses one-based indices. First/second-object examples start with fresh contexts, explicitly return to root where needed and document literal dotted-key precedence. | One new actual-engine test executes the documented four comparisons, root return and literal-key precedence; 38 web-source guide tests passed. |

Three scripting-reference tests also passed. All 59 existing marked web-source command examples retain their original first three fenced blocks. Strict MkDocs, formatting and scoped whitespace checks passed; the root combined Rust suite includes the new engine test. No production parser behavior was changed by these corrections.

No browser/native/clipboard/provider/Windows operation was performed. These bounded fixes do not certify the remaining original documentation details or the requested improvement in user teaching; continue the detail audit with actual examples and explicit unsupported boundaries.
