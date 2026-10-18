# Protocol 23.87: one file of a pull request, read when it is opened

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Acts on a pull request*.
**Additive only**: one operation and the DTOs `PullRequestFileOf` and `PullRequestFilePatch`.
`PullRequestPage` is unchanged, so no patch rides it.

| Operation | Carries | Notes |
| --- | --- | --- |
| `get_pull_request_file` (`GET /pull_request_files/:repository/:number?path=`) | `PullRequestFilePatch`: `path`, and `patch` or `said` | `patch` is the forge's hunks for the file with no `diff --git` header. A file the forge sent none for (binary, or too large) is a 200 with `said` in Fleet's words. A path the pull request does not list is a 422 `fleet.pull_request_file_unknown`; a forge that would not answer is a 500 `fleet.pull_request_unreadable`. `agent_access` `Bridge only` |

**Read from the forge with `gh api repos/{owner}/{repo}/pulls/N/files`**, the one read that carries a patch per file and leaves it out for a file it will not show.
Fleet reads the pages and keeps the one file, so a person opening a file pays for the pull request's files and not for a patch on every page read.

**Skew.** A Fleet before 23.87 has no route and answers 404, and the IDs differ, so the two are refused regardless.
Bridge's half is `packages/protocol/src/pull-requests.ts`, written by hand like the rest.
There is no number to take: `armada need --status` shows nothing standing, and the sections above carry no ordering to follow.
