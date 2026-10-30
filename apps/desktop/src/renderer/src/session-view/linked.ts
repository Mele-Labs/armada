// The Sessions a parent follows, read off its ledger and the window's Sessions. A plan group handed to a new Session is a `started_to` row
// with the group's id; the child's state, its pull request and its title are its own. **Drawn the same wherever the parent keeps them**:
// under a Plan group and in the Artifacts layer.

import type { LinkedSessionProps } from "@armada/components";
import { attachmentsOf, linkedOf, type Session } from "@armada/screens/src/draft/sessions";

import { stateOf } from "../session-state";

type Checks = ReturnType<typeof attachmentsOf<"pull_request">>[number]["checks"];

const checksSaid = (checks: Checks) =>
  checks.state === "failed" ? `Checks failed: ${checks.failing}` : checks.state === "pending" ? "Checks running" : "Checks passed";

/** The pull request that stands for a child: the open one, else the last it merged. */
export function pullOf(child: Session) {
  const all = attachmentsOf(child, "pull_request");
  return all.find((pr) => pr.state !== "merged") ?? all.at(-1);
}

function prOf(child: Session): LinkedSessionProps["pr"] {
  const one = pullOf(child);
  if (one === undefined) return undefined;
  return {
    number: one.number,
    merged: one.state === "merged",
    checks: one.checks.state === "failed" ? "failed" : one.checks.state === "pending" ? "running" : "passed",
    checksSaid: checksSaid(one.checks),
  };
}

/** A child as the shared row draws it. `group` is left off where the row already stands under the group's own. */
export function linkedPropsOf(child: Session, open: (id: string) => void, group?: string): LinkedSessionProps {
  const { state, said } = stateOf(child);
  const pr = prOf(child);
  return {
    title: child.title ?? child.id,
    ...(group === undefined ? {} : { group }),
    state,
    said,
    ...(pr === undefined ? {} : { pr }),
    onOpen: () => open(child.id),
  };
}

/** The title of the plan group a child was handed from. */
export const groupTitleOf = (parent: Session, group: string): string | undefined => parent.work?.plan?.groups.find((one) => one.id === group)?.title;

/** The children of a parent, in the order its ledger holds them, each with the title of its group. */
export function childrenOf(sessions: readonly Session[], parent: Session): { session: Session; group: string; title: string | undefined }[] {
  return linkedOf(sessions, parent).map((one) => ({ ...one, title: groupTitleOf(parent, one.group) }));
}
