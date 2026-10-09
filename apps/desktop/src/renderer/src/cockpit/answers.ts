// What a call can be answered with, from the keyboard: its answers as a numbered list, one picked,
// and Send. A Plan question is its decisions' options, one decision at a time; every other call has
// the acts its detail card offered, said as answers. A call with no answer of its own opens what it
// is about. Mock only: each answer does what its button did on the Dashboard, and clears the call.

import { useEffect, useState } from "react";

import type { BridgeState } from "../../../shared/bridge";
import { viewsOf } from "../merge-line";
import type { Hosts, Item } from "../Dashboard";

export type Answer = { id: string; label: string; run: () => void };

export type Answering = {
  /** The question asked now: a Plan decision's, or the call's own fact. */
  ask: string;
  answers: readonly Answer[];
  /** Which answer is picked, where one is. A lone answer starts picked. */
  picked: number | undefined;
  pick: (index: number) => void;
  /** Moves the pick down or up the list, from nothing to the first or last. */
  step: (by: 1 | -1) => void;
  /** Whether Enter sends: an answer is picked, or the call has none and Enter opens it. */
  canSend: boolean;
  send: () => void;
  /** Which decision of a Plan question this is, and how many there are. */
  decision?: { at: number; of: number };
  /** Opens what the call is about, where it has somewhere to open. */
  open: (() => void) | undefined;
};

/** Where `o` goes: the Job, the Session, or the pull request the call is about. */
function opener(item: Item, hosts: Hosts, state: BridgeState): (() => void) | undefined {
  if (item.job !== undefined) return () => hosts.onOpen(item.job!.id);
  if (item.key.startsWith("session:")) return () => hosts.onOpenSession(item.key.slice("session:".length));
  if (item.key.startsWith("pull:")) {
    const number = Number(item.key.slice("pull:".length));
    const pull = viewsOf(state).flatMap((view) => view.hub?.pulls ?? []).find((one) => one.number === number);
    return pull === undefined ? undefined : () => hosts.onOpenLink(pull.url);
  }
  return undefined;
}

/** The answers a call that is not a Plan question offers. */
function actsOf(item: Item, hosts: Hosts, finish: () => void): Answer[] {
  const answer = (label: string, run: () => void = finish): Answer => ({ id: label, label, run });
  if (item.key.startsWith("session:")) return [answer("Allow"), answer("Deny")];
  if (item.key.startsWith("main:")) {
    return [answer("Hand to a Job", () => (hosts.onFix?.({ root: item.key.slice("main:".length) }), finish()))];
  }
  if (item.key.startsWith("pull:")) {
    const number = Number(item.key.slice("pull:".length));
    return [answer("Send a Drone", () => (hosts.onPropose?.(`The checks on pull request #${number} failed. Read the failure, say what you would change, and push only once I agree.`), finish()))];
  }
  if (item.kind === "Check failed") return [answer("Retry")];
  if (item.kind === "Drone stuck") return [answer("Restart the step"), answer("Fresh Drone")];
  return [];
}

export function useAnswering(item: Item, hosts: Hosts, state: BridgeState, finish: () => void): Answering {
  const [at, setAt] = useState(0);
  const [picked, setPicked] = useState<number>();
  // A new call starts from its first question with nothing picked.
  useEffect(() => (setAt(0), setPicked(undefined)), [item.key]);

  const decisions = item.decisions;
  const decision = decisions?.[at];
  const open = opener(item, hosts, state);

  const answers: Answer[] =
    decision !== undefined
      ? decision.options.map((option) => ({
          id: option.id,
          label: option.label,
          run: () => (at + 1 < decisions!.length ? (setAt(at + 1), setPicked(undefined)) : finish()),
        }))
      : actsOf(item, hosts, finish);

  // One answer is the answer: Enter sends it without a number first.
  const chosen = picked ?? (answers.length === 1 ? 0 : undefined);
  const pick = (index: number) => answers[index] !== undefined && setPicked(index);
  const step = (by: 1 | -1) =>
    setPicked(chosen === undefined ? (by === 1 ? 0 : answers.length - 1) : Math.min(answers.length - 1, Math.max(0, chosen + by)));
  const send = () => {
    if (answers.length === 0) open?.();
    else if (chosen !== undefined) answers[chosen]?.run();
  };

  return {
    ask: decision?.question ?? item.fact,
    answers,
    picked: chosen,
    pick,
    step,
    canSend: answers.length === 0 ? open !== undefined : chosen !== undefined,
    send,
    ...(decisions === undefined || decisions.length < 2 ? {} : { decision: { at, of: decisions.length } }),
    open,
  };
}
