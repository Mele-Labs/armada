// The Sessions a parent follows, read off its ledger and the window's Sessions: every `started_to` row, with the group's id where a plan group
// was handed; the child's state, its pull request and its title are its own. **Drawn the same wherever the parent keeps them**: under its Plan
// group, or in Now and then Done when it had none, and in the Artifacts layer.

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

/** Whether a child has finished: done, ended or out of reach. Until then it is work in hand on its parent's spine. */
export function finishedChild(child: Session): boolean {
  const { state } = stateOf(child);
  return state === "done" || state === "ended" || state === "quiet";
}

/** The title of the plan group a child was handed from. */
export const groupTitleOf = (parent: Session, group: string): string | undefined => parent.work?.plan?.groups.find((one) => one.id === group)?.title;

/** The children of a parent, in the order its ledger holds them, each with the title of its group when it was handed one. */
export function childrenOf(sessions: readonly Session[], parent: Session): { session: Session; group?: string; title: string | undefined }[] {
  return linkedOf(sessions, parent).map((one) => ({ ...one, title: one.group === undefined ? undefined : groupTitleOf(parent, one.group) }));
}

/**
 * Where each plan group handed to a child stands while the child has not finished, whatever the parent ticked when it handed it: being worked once
 * the child is under way, still to come while it waits to start. A finished child leaves its group to its tasks.
 */
export function inHandOf(sessions: readonly Session[], parent: Session): Map<string, "current" | "upcoming"> {
  return new Map(
    childrenOf(sessions, parent).flatMap((one): [string, "current" | "upcoming"][] => {
      if (one.group === undefined || finishedChild(one.session)) return [];
      const { state } = stateOf(one.session);
      return [[one.group, state === "after" || state === "draft" ? "upcoming" : "current"]];
    }),
  );
}

/** The children a parent started outside its plan, the ones no group's row draws. */
export const looseChildrenOf = (sessions: readonly Session[], parent: Session): Session[] =>
  childrenOf(sessions, parent).flatMap((one) => (one.group === undefined ? [one.session] : []));
