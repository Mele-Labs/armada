# Protocol 14.16: Helm writes a file in the checkout

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1373`. **Retired 13 Oct 2026 with Helm's checkout writes; see *Helm is a Job*.** `helm.changed_checkout`, a new event kind, additive: published as a Helm job's own
stream says it wrote a file, carrying `HelmChangedCheckout { manifest_id, tool, path, at }`.
Helm edits the repository's checkout directly on a person's ask, with no worktree and no Job
around the change, so this is what lets somebody who finds a file changed see that Helm changed
it — `docs/concepts/helm.md`, *Audit trail*, and the rule `studio.helm_acted` already follows.

**It names writes Armada can name, not every change Helm caused.** `tool` is one of the built-ins
that edits a file, so the path is known; a shell line may also have written something and nothing
in the stream says whether it did. Those calls stay on the conversation's own socket, where they
already were.
