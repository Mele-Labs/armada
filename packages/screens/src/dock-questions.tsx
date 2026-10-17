// Every Helm call waiting on a person, as the dock's cards (#935). Main gathers them from every
// repository Fleet serves; this decides what each card says.

import type { DockQuestion } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { manifestLabel } from "@armada/shell/src/repository-label";
import { helmOfferedOf } from "./copy";
import { span } from "./duration";
import { outstandingId, type Outstanding } from "./outstanding";

/** What a person may do from a card. Absent draws the control off. */
export type DockActs = {
  /** An answer by its wire name, a `HelmCallAnswer`. #936. */
  onAnswer?: (question: Outstanding, answer: string) => void;
  /** True while this card's own answer is in flight. */
  answering?: (question: Outstanding) => boolean;
  /** Why this card's last answer did not take, in Fleet's own words. Cleared by a fresh attempt. */
  refusalFor?: (question: Outstanding) => string | undefined;
};

/** When it was asked. */
export function askedAt(question: Outstanding): string {
  return question.call.asked_at;
}

/** How long it has waited: a figure at `now`, or where the window has no time to give, a card that reads the clock where it is drawn. */
const waitingOf = (question: Outstanding, now: number | null): DockQuestion["waiting"] =>
  now === null ? (at: number) => span(askedAt(question), at) ?? undefined : (span(askedAt(question), now) ?? undefined);

/**
 * The cards, oldest first. **A call is drawn without any board behind it**: it names its repository
 * off the ask itself, and never falls out of the list for a board that does not hold it. #1389.
 */
export function dockQuestionsOf(
  questions: readonly Outstanding[],
  repositories: readonly RepositorySummary[],
  now: number | null,
  acts: DockActs = {},
): DockQuestion[] {
  return [...questions]
    .sort((a, b) => Date.parse(askedAt(a)) - Date.parse(askedAt(b)))
    .map((question) => {
      const { onAnswer, answering, refusalFor } = acts;
      const { tool, detail, rule, offers } = question.call;
      return {
        id: outstandingId(question),
        repository: manifestLabel(question.call.manifest_id, repositories),
        waiting: waitingOf(question, now),
        label: "Helm needs your permission",
        asked: detail === "" ? `Helm wants to use ${tool}.` : <span className="mono">{detail}</span>,
        detail: `Your settings would have to allow ${rule}.`,
        answers: helmOfferedOf(offers).map(({ offer, label, means }) => ({ id: offer, label, consequence: means })),
        ...(onAnswer === undefined
          ? { note: "Open Armada on this machine to answer." }
          : { onAnswer: (answer: string) => onAnswer(question, answer), answering: answering?.(question) }),
        refusal: refusalFor?.(question),
      };
    });
}
