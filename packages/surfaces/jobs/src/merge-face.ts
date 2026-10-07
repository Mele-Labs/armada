// What the Merge control says and does at the review gate, from the forge's own checks on the pull
// request. **Passed or unread: Merge. Still running: Enable auto-merge. Failed: off, with the names.**
// The forge's CI, never Armada's Checks (`ci.ts`). `nothing_ran` and `unreadable` are Merge, since
// there is no run to wait on, and the forge's own refusal covers a branch rule this cannot see.

import type { PullRequestChecks, PullRequestState } from "@armada/protocol";

export type MergeFace = {
  /** Which press the control sends. */
  how: "merge" | "auto_merge";
  /** The face's label, where it is not the dialog's default. */
  label?: string;
  /** Present when the control is drawn and off. */
  blocked?: string;
};

export function mergeFaceOf(
  checks: PullRequestChecks | undefined,
  reading: PullRequestState | undefined,
): MergeFace {
  if (reading?.auto_merge === true && reading.state !== "merged") {
    return { how: "auto_merge", label: "Auto-merge on", blocked: "Merges when every check passes" };
  }
  const kind = checks?.kind;
  if (kind === "still_waiting") return { how: "auto_merge", label: "Enable auto-merge" };
  if (kind === "some_failed") {
    const failed = checks?.failed ?? [];
    return {
      how: "merge",
      blocked: failed.length === 0 ? "Checks failed" : `${failed.join(", ")} failed`,
    };
  }
  return { how: "merge" };
}

/** The checks as the card says them, in the forge's terms. `undefined` draws nothing. */
export function checksFactOf(checks: PullRequestChecks | undefined): string | undefined {
  switch (checks?.kind) {
    case "all_passed":
      return "Checks passed";
    case "still_waiting":
      return "Checks running";
    case "some_failed":
      return `Checks failed: ${(checks.failed ?? []).join(", ")}`.replace(/: $/, "");
    default:
      return undefined;
  }
}

export function confirmAutoMerge(host: string): { title: string; body: string } {
  return {
    title: "Merge when the checks pass?",
    body:
      `The checks on ${host} are still running. ${host} merges the pull request when they pass, and ` +
      "the job stays here until it has. Armada then takes the work and runs the repository's " +
      "after-merge checks against what landed.",
  };
}
