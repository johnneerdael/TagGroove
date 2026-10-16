# Diagnostic library foundation

This dependency layer introduces the tag-core diagnostic APIs used by the
language-switching integration. The desktop application continues to call its
existing English APIs. Language settings, native IPC payloads, translation
rendering and the two language-guide chapters belong to the later integration.

## Compatibility boundary

- Keep existing `String` errors and result structures available to callers.
- Add structured causes and indexed diagnostic companions alongside legacy
  results. Preserve the original error order and raw external values.
- Render source-owned English messages from the same structured cause used by
  the detailed API. Substitute placeholders once so braces in filenames,
  field values and external diagnostics remain data.
- Retain the existing scripting and export entry points while adding detailed
  failure forms for the later native integration.

The layer contains all 29 tag-core files from the integration snapshot
`ed9beee4091268d12cecc961d6faadc9746637cb`, together with its import,
scripting and batch-error diagnostic fixtures. The batch-error fixture is
required by the new core unit tests and therefore travels with this library.
These files are preserved without selectively removing tests or changing
their implementation for the intermediate application.

## Qualification

Qualify this actual combination of new core and existing application bindings
with the full locked workspace tests and all-target Clippy. The completed
integration's native results do not establish that this intermediate
combination compiles or preserves the legacy calls. Also build the existing
frontend, run its UI tests and require the strict guide build and publication
checks. Record measured results in the PR's validation section.

This layer activates no language feature or new user setting. Its public
release notes describe the diagnostic library and compatibility validation;
the later integration supplies the user-facing language instructions.
