# Protocol 23.24: how the work leaves the worktree, as approved

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: the canvas offers stop at the branch, and a pull request that merges itself.

**Two optional booleans on two shapes, additive.** `LandingChoice` and `LandingRule` gain `local`
and `auto_merge`, each absent at false.

| Setting | What Fleet does |
|---|---|
| `local` | **Stop at the branch.** On entering the delivering step the work is committed on the Job's own branch, with no push, no pull request and no merge. The step then runs as any other, and the branch outlives the Job: reclaiming keeps unmerged work. |
| `auto_merge` | When the pull request opens, Fleet runs `gh pr merge --auto --merge` on it, so the forge merges once its required checks pass. A forge that will not, usually a repository that does not allow auto-merge, is said in the Job's log as a warning carrying the forge's own sentence, and the pull request stays open for a person. |

**One source of truth with `pr_mode`.** `local` wins: `pr_mode` is stored `ready` while it holds, so
`LandingRule` never reads `draft` beside `local`. `local` with `auto_merge` is 422
`fleet.unacceptable_proposal`, since a branch with no pull request has nothing to merge.
`policy_overrides.auto_merge` is untouched: it answers a `manifest_rule:auto_merge` gate, which is
the repository deciding, and this is one Job's forge setting.

**Store V104** adds `local` and `auto_merge` to `job_landing`, false on every row kept before. `Delivery`
gains `enable_auto_merge`. The refusal is a log line and not a wire error, because the approval
cannot know the forge's answer until the pull request exists.
