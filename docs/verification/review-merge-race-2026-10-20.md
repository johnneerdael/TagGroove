# PR73 postmerge proof race

PR73 merged as `b3097651a7bbb00b8ae4e3ed8f77bf272cf6e3ca` after clean
exact-head Codex review and successful Linux validation run 39485594147.
The final live controller run 39488075602 passed before the normal protected
merge. No admin bypass was used.

Metadata controller 39488077315 overlapped that merge. Its snapshot changed
from open to closed, so it wrote the pending status `PR changed while reading
review state` at 20:00:01 UTC. Main controller 39488207818 then refused release
authorization because that status hid the preceding successful proof.
No packaging or native integration jobs had started.

The workflow repair skips only that exact pending marker from the trusted
Actions Bot during merged-source proof selection. It still requires a fresh
clean merged review, valid notes, unique open head, trusted completed successful
validation run, exact base/head parents and identical tested/merged trees.
An intervening failure, any other invalidation, missing proof or untrusted
marker still refuses. No commit status or review approval is manufactured.

The positive race regression failed before the repair. The full GitHub tooling
suite passes 135 tests after it, including refusal cases. Release-script tests
and actionlint are checked separately before the direct-main workflow push.
Actual release dispatch and publication require subsequent hosted evidence.
