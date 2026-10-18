# Protocol 23.42: acting on main's red, and what merged

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Additive.
`fix_main` (`POST /merge_lines/fix`, body `FixMain {root, job?, brief?}`) dispatches a Job for a red main or sends the work back to one, and answers with that Job's `JobSummary`.
`JobSummary.fixes_main` (`FixesMain {state, check, test?, merge?, fixed_in?}`) is a Job's part in a red; `hub.fixing`, which 23.41 declared and never set, now names the Job on it; `hub.merged` lists the newest five pull requests merged into the base, read from the forge on the visit that already lists the open ones.

**Older peers:** a Bridge before 23.42 ignores every field and draws no buttons, so nothing breaks; it draws the line's own `landed` list.
A Bridge at 23.42 against an older Fleet is refused as any minor ahead is, so the buttons never reach a Fleet without the route.
`FixesMain.state` is read as working for a value it does not know, so a new one is not a major.

**One store table, additive** (`main_fix.main_ci_fixes`): which Job took each red. No other record changed.
