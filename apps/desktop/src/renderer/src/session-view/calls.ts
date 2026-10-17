// What a Session waits on him for, as one list of calls, for the takes that draw it away from the pinned region: in the composer, as the thread's
// last card, or in the right column. A call is a line of words and the answers it takes, numbered: a question's options, a permission's Allow and
// Deny, a plan's Approve and Changes, a judge's Send and Dismiss. Nothing here sends anything itself: each answer runs what the pinned cards ran.
// The take is the scenario's (`draft.calls`, set on the root as `data-calls`); with none set the thread pins its cards as it did.

import { useCallback, useMemo, useRef, useState } from "react";
import type { QuestionAnswer } from "@armada/components";
import type { Session, SessionAnswer } from "@armada/screens/src/draft/sessions";

export type CallTake = "composer" | "thread" | "progress";

/** The take this page draws, or none: the pinned region. */
export function callsTake(): CallTake | undefined {
  const one = document.documentElement.dataset.calls;
  return one === "composer" || one === "thread" || one === "progress" ? one : undefined;
}

/** One answer a call takes: pressed, or picked by its digit. */
export type CallOption = { key: string; label: string; hint?: string; run: () => void };

export type Call = {
  key: string;
  kind: "question" | "permission" | "plan" | "judge";
  /** What it is, in a word. */
  word: string;
  /** The one line: the question, the command, what the plan or the judge says. */
  text: string;
  /** What More reads under it. */
  detail: readonly string[];
  options: readonly CallOption[];
  /** Typing is Other: the words are the answer. A question only. */
  other?: (text: string) => void;
};

/** What answering does, as the pinned cards did it. */
export type CallActs = {
  answer: (answer?: string, answers?: QuestionAnswer[], folder?: string) => void;
  approve: () => void;
  review: () => void;
  sendConcern: (id: string) => void;
  dismissConcern: (id: string) => void;
};

const OFFER: Partial<Record<SessionAnswer, string>> = { allow_once: "Allow", refuse: "Deny", allow_and_remember: "Allow and remember" };

/** The calls, the one the agent is held on first. A question asked in several parts is a call for each part, and is sent once all are answered. */
export function useCalls(session: Session, acts: CallActs): readonly Call[] {
  const [parts, setParts] = useState<Readonly<Record<string, QuestionAnswer>>>({});
  const held = useRef(parts);
  held.current = parts;
  const asked = session.asked;
  const plan = session.work?.plan;
  const concerns = session.work?.concerns;
  const answerPart = useCallback(
    (key: string, of: readonly string[], answer: QuestionAnswer) => {
      const next = { ...held.current, [key]: answer };
      if (of.every((one) => next[one] !== undefined)) {
        setParts({});
        acts.answer(undefined, of.map((one) => next[one]!));
      } else setParts(next);
    },
    [acts],
  );
  return useMemo(() => {
    const list: Call[] = [];
    if (asked !== undefined) {
      const questions = asked.questions ?? [];
      if (questions.length > 0) {
        const keys = questions.map((_, at) => `ask:${asked.call ?? session.id}:${at}`);
        questions.forEach((one, at) => {
          const key = keys[at]!;
          if (parts[key] !== undefined) return;
          const pick = (chosen: string) => answerPart(key, keys, { question: one.question, chosen: [chosen] });
          list.push({
            key,
            kind: "question",
            word: "Question",
            text: one.question,
            detail: [],
            options: one.options.map((option) => ({ key: option.label, label: option.label, ...(option.description === "" ? {} : { hint: option.description }), run: () => pick(option.label) })),
            other: pick,
          });
        });
      } else {
        const offers = (asked.offers ?? ["allow_once", "refuse"]).filter((one) => OFFER[one] !== undefined);
        list.push({
          key: `perm:${asked.call ?? session.id}`,
          kind: "permission",
          word: "Permission",
          text: asked.command,
          detail: [],
          options: offers.map((one) => ({ key: one, label: OFFER[one]!, run: () => acts.answer(one) })),
        });
      }
    }
    if (plan !== undefined && !plan.approved) {
      list.push({
        key: "plan",
        kind: "plan",
        word: "Plan",
        text: "Plan to approve",
        detail: plan.groups.map((one) => one.title),
        options: [
          { key: "approve", label: "Approve", run: acts.approve },
          { key: "changes", label: "Changes", run: acts.review },
        ],
      });
    }
    for (const one of concerns ?? []) {
      list.push({
        key: `judge-${one.id}`,
        kind: "judge",
        word: "Judge",
        text: one.consequence ?? one.text.split("\n")[0] ?? one.text,
        detail: [one.expected, one.produced].flatMap((line) => (line === undefined ? [] : [line])),
        options: [
          { key: "send", label: "Send to session", hint: "Goes to the agent as your message", run: () => acts.sendConcern(one.id) },
          { key: "dismiss", label: "Dismiss", run: () => acts.dismissConcern(one.id) },
        ],
      });
    }
    return list;
  }, [asked, plan, concerns, parts, session.id, acts, answerPart]);
}
