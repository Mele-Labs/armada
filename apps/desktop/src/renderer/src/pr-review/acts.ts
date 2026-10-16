// What a press on the review surface does beyond the store: a finding sent to the Session that holds the
// pull request is a message to that Session, sent the way its own composer sends one.

import { useCallback } from "react";

import { useSessionsDraft } from "../sessions-draft";
import type { Finding, ReviewPr } from "./model";
import { useReviewStore } from "./store";

const textOf = (pr: ReviewPr, findings: readonly Finding[]): string =>
  findings.length === 1
    ? `Review finding on #${pr.number}, ${findings[0]!.path}: ${findings[0]!.title}\n\n${findings[0]!.body}`
    : [`Review findings on #${pr.number}`, ...findings.map((one) => `${one.path}: ${one.title}\n${one.body}`)].join("\n\n");

/** Send findings to the Session that holds `pr`. Absent where none of his Sessions does. */
export function useSendFindings(pr: ReviewPr | undefined): ((findings: readonly Finding[]) => void) | undefined {
  const draft = useSessionsDraft();
  const store = useReviewStore();
  const holder = pr?.holder;
  const send = useCallback(
    (findings: readonly Finding[]) => {
      if (pr === undefined || holder === undefined || draft === undefined || store === undefined || findings.length === 0) return;
      void Promise.resolve(draft.send(holder.id, { text: textOf(pr, findings), files: [], sketches: [], tags: [] })).then((went) => {
        if (went !== false) store.sent(pr.key, findings.map((one) => one.id));
      });
    },
    [pr, holder, draft, store],
  );
  return holder === undefined || draft === undefined ? undefined : send;
}
