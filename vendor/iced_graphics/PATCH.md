# Patched iced_graphics 0.14.0

This is the crates.io release of `iced_graphics` 0.14.0 (MIT, iced
contributors; see LICENSE) with one change, in `src/damage.rs`, function
`group`. The app uses it through `[patch.crates-io]` in Cargo.toml.

**Why:** iced's CPU renderer repaints only the damaged parts of the window.
`group` merges the damaged rectangles in one greedy pass sorted by distance
from the window corner. With a scrolled two-column list (avatars and dots on
the left, times and badges on the right) that pass can leave dozens of
overlapping groups, and each group redraws everything it covers. Measured
with the fast-rebuild tools on a 1280x800 window at 125%: one wheel notch took
115-125 ms to reach the screen (94 ms drawing 38 groups) instead of 25 ms.

**The change:** after the greedy pass, keep merging any two groups whose union
wastes no more area than the original threshold, until none do. Overlapping
groups always merge; distant small changes (a caret blink and a hover) stay
separate. With it the same scroll draws one region in 5.6 ms.

**When to drop it:** when an iced release changes `damage::group` upstream,
delete this folder and the `[patch.crates-io]` entry, then rerun
`tools/feel.py` on the project to confirm scrolling stays within budget.
FiraSans-Regular.ttf is under the SIL Open Font License (fonts/OFL.txt).
