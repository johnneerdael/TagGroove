# Refresh preserves surviving manual order — 2026-10-18

Consolidated draft PR64 fixes a reproduced mismatch with the existing file-list
contract. Folder and playlist rows ABC could be manually moved to CAB; unchanged
source Refresh then rebuilt ABC while leaving manual mode selected. Saved
column criteria stayed intact, so the displayed mode did not explain the reset.

The [Refresh block](../../src/main.ts) snapshots surviving exact IDs before fresh
acceptance, retains the normal fresh values/selection/chapter replacement, then
restores surviving rank and appends new source IDs plus any remaining actual
accepted rows. It does not resurrect removed values. Fresh source rank remains
independent for explicit Playlist order, and column criteria/current mode stay
unchanged. Cache invalidation remains before later ordered selection reads.

[Production-function controls](../../tests/playlist-refresh.test.mjs) cover folder
and playlist CAB retention, all three display modes, new/removed rows, changed
exact chapter IDs, fresh source rank and explicit Playlist order. Existing
cancellation/stale/context and pending-edit controls remain. The busy-move
control accurately checks that the existing guard refuses manual moves during
scanning; it does not claim a successful concurrent reorder.

Five order controls failed against the prior code. The final 26 focused
playlist-refresh/order tests, production build and whitespace checks passed.
Independent source review found no remaining confident finding in the two-file
slice. Backend scanning is controlled at the test seam; this is not native,
original or provider qualification.

Cross-session ordinary order persistence remains L47b. This fix adds no startup
choice, automatic reopening or persistent state; explicit portable backup is a
separate implemented workflow.
