# Protocol 23.41: the hub on the merge line

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`ipc::MergeLineHub`, additive on `MergeLine` as `hub`: main's CI as Fleet reads it off the forge and every open pull request with its `ci`.
A line from a Fleet before 23.41 has none and a Bridge before it ignores one.
`hub.main.state` and `hub.pull_requests[].ci` are strict sets Bridge picks a mark from, so a new value is a major.

**A repository with a hub and no queue is now in `get_merge_lines`**, with an empty line.
That is the one thing an older Bridge sees: an empty panel where it drew none, which is a rendering and not a parse.

**Main's failed job log is not a new route.** `observe_land_check` takes the branch `main` and reads the job's log off the forge, so the lifeboat and the route table are untouched and Bridge still never reaches the forge.

**Event stream: neither better nor worse.** It adds no queue and no event kind.
The two-second loop already published `merge_lines.changed` whole on a change; the hub makes that message larger (a hundred pull requests at most) and changes it when main or a pull request does, which is rarer than a queue turn.
