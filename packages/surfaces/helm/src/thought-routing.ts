// A thought typed into Helm's dock, read for where it belongs before it is asked. Fleet does the
// reading (`read_thought`) and Bridge the delivering, so this is the seam between the dock, which
// draws the card, and the window, which holds the Sessions, the Jobs and the Dispatch panel.

import type { ThoughtTarget as ThoughtOnTheWire } from "@armada/protocol";
import type { RepositorySummary } from "@armada/protocol";
import type { ComposerFile, ComposerTag, ThoughtTarget } from "@armada/components";

/**
 * The Session or Job a thought may go to. `id` is what the acts below take, and `manifestId` the
 * repository Fleet placed it in, which the wire's target is built back from.
 */
export type RouteTarget = ThoughtTarget & { id: string; manifestId?: string };

/** What Helm made of the words: a question for itself, or a thought for other work, with the best match if any. */
export type ThoughtReading = { kind: "question" } | { kind: "thought"; match?: RouteTarget };

/** What went with the words: pictures and files, sketches, and the issues, pull requests and Sessions tagged in them. */
export type ThoughtCarry = {
  files: readonly ComposerFile[];
  tags: readonly ComposerTag[];
  sketches: readonly { id: string; title: string }[];
};

export type ThoughtRouting = {
  /** Fleet's reading of the words. **A read that failed is a question**, so the words are never lost. */
  read: (text: string) => Promise<ThoughtReading>;
  /**
   * Delivers the words: a Session gets a message, a Job a redirect to its Drone. Resolves to whether
   * they arrived. A refusal is already said, in Fleet's words, and no row is recorded for it; one
   * that arrived is recorded, and the thread then carries it as a row of its own. **A Session gets
   * what went with the words; a Job's Drone is redirected with the words alone.**
   */
  send: (target: RouteTarget, text: string, carry?: ThoughtCarry) => Promise<boolean>;
  /** Opens the Session or Job. */
  open: (target: RouteTarget) => void;
  /** Opens the Dispatch panel holding the words, with this kind picked. */
  startNew: (kind: "session" | "job", text: string) => void;
};

/** A repository's name as the picker draws it, or its id where the repository is not one Fleet serves. */
export function repositoryName(repositories: readonly RepositorySummary[], manifestId: string | undefined): string {
  return repositories.find((one) => one.manifest?.id === manifestId)?.manifest?.repository ?? manifestId ?? "";
}

/** The wire's target, drawn: the repository named rather than its id. */
export function routeTargetOf(target: ThoughtOnTheWire, repositories: readonly RepositorySummary[]): RouteTarget {
  return {
    kind: target.kind,
    id: target.id,
    title: target.title,
    repository: repositoryName(repositories, target.manifest_id),
    ...(target.manifest_id === undefined ? {} : { manifestId: target.manifest_id }),
    ...(target.slot === undefined ? {} : { slot: target.slot }),
  };
}

/** A drawn target as the wire has it, for the row Fleet keeps. */
export function wireTargetOf(target: RouteTarget): ThoughtOnTheWire {
  return {
    kind: target.kind,
    id: target.id,
    title: target.title,
    ...(target.manifestId === undefined ? {} : { manifest_id: target.manifestId }),
    ...(target.slot === undefined ? {} : { slot: target.slot }),
  };
}
