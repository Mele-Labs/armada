// The pull request behind a Job at its gate, as the mock's forge shows it: its checks are the
// fixture's `delivery.pull_request_detail.checks`, and Merge or Enable auto-merge move it as
// Fleet's answers do. Pure, so the mock and a test read the same rules.

import type { JobDetail, PullRequestState } from "@armada/protocol";

const numberOf = (address: string): number => Number(address.split("/").pop()) || 1;

export function pullRequestStateOf(whole: JobDetail, asked: boolean): PullRequestState | undefined {
  const address = whole.delivery?.pull_request;
  if (address === undefined) return undefined;
  const checks = whole.delivery?.pull_request_detail?.checks;
  return {
    manifest_id: whole.job.owner_manifest_id,
    number: numberOf(address),
    state: whole.delivery?.landed === "merged" ? "merged" : "open",
    auto_merge: asked,
    checks:
      checks?.kind === "all_passed"
        ? { state: "passed" }
        : checks?.kind === "some_failed"
          ? { state: "failed", failing: checks.failed ?? [] }
          : { state: "pending" },
    title: whole.delivery?.pull_request_title ?? "",
    branch: whole.job.branch ?? "",
    address,
  };
}

/** Fleet's refusal of Enable auto-merge, for the two checks readings it cannot take. */
export function autoMergeRefusal(state: PullRequestState): string | undefined {
  if (state.checks.state === "passed") return `the checks on pull request ${state.number} have passed already: merge it`;
  if (state.checks.state === "failed") return `${state.checks.failing.join(", ")} failed on pull request ${state.number}`;
  return undefined;
}
