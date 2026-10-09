// What a call can be answered with, from the keyboard: its answers as a numbered list, one picked,
// and Send. A Plan question is its decisions' options, one decision at a time; every other call has
// the acts its detail card offered, said as answers. A call with no answer of its own opens what it
// is about. Mock only: each answer does what its button did on the Dashboard, and clears the call.

import { useEffect, useState } from "react";

import type { BridgeState } from "../../../shared/bridge";
import { viewsOf } from "../merge-line";
import type { Hosts, Item } from "../Dashboard";

/** `key` is a standing answer's own key; the others are picked by their number. `says` is what it tells the agent. */
export type Answer = { id: string; label: string; run: () => void; key?: string; says?: string };

export type Answering = {
  /** The question asked now: a Plan decision's, or the call's own fact. */
  ask: string;
  answers: readonly Answer[];
  /** Which answer is picked, where one is. A lone answer starts picked. */
  picked: number | undefined;
  pick: (index: number) => void;
  /** Picks the standing answer a key names, where there is one. */
  pickKey: (key: string) => boolean;
  /** Picks the answer a digit names among the numbered ones. */
  pickNumber: (digit: number) => void;
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

/**
 * Two answers that stand after the numbered ones on a question an agent asked: the best solution,
 * thought through, or the quickest reasonable path. **On a Session's permission they hand the
 * decision to run the command to the agent** (owner, 9 Oct 2026), so they say that. Mock only: each
 * would be one Fleet command, an answer to the ask carrying `mode: "best" | "quick"` that the agent
 * is told as an instruction.
 */
function standingOf(finish: () => void, permission: boolean): Answer[] {
  return [
    {
      id: "best",
      key: "b",
      label: "Make the best decision",
      says: permission ? "Tells the agent to weigh the command carefully and decide for itself whether to run it" : "Tells the agent to weigh the options and choose the best solution itself",
      run: finish,
    },
    {
      id: "quick",
      key: "g",
      label: "Just get it done",
      says: permission ? "Tells the agent to run the command if it is reasonable and keep moving" : "Tells the agent to take the quickest reasonable path and keep moving",
      run: finish,
    },
  ];
}

export function useAnswering(item: Item, hosts: Hosts, state: BridgeState, finish: () => void): Answering {
  const [at, setAt] = useState(0);
  const [picked, setPicked] = useState<number>();
  // A new call starts from its first question with nothing picked.
  useEffect(() => (setAt(0), setPicked(undefined)), [item.key]);

  const decisions = item.decisions;
  const decision = decisions?.[at];
  const open = opener(item, hosts, state);

  const numbered: Answer[] =
    decision !== undefined
      ? decision.options.map((option) => ({
          id: option.id,
          label: option.label,
          run: () => (at + 1 < decisions!.length ? (setAt(at + 1), setPicked(undefined)) : finish()),
        }))
      : actsOf(item, hosts, finish);
  // The standing two answer a question an agent asked and could have answered itself: a Plan question,
  // a Drone's or a Judge's, a Session's ask. A call Fleet raises about a state (a failing pull request,
  // main red, a failed Check, a stuck Drone) keeps only its own acts (owner, 9 Oct 2026).
  const askedByAgent = decisions !== undefined || item.kind === "Drone question" || item.kind === "Judge question" || item.key.startsWith("session:");
  const answers = askedByAgent ? [...numbered, ...standingOf(finish, item.key.startsWith("session:"))] : numbered;

  // One answer is the answer: Enter sends it without a number first.
  const chosen = picked ?? (answers.length === 1 ? 0 : undefined);
  const pick = (index: number) => answers[index] !== undefined && setPicked(index);
  const pickNumber = (digit: number) => digit <= numbered.length && pick(digit - 1);
  const pickKey = (key: string) => {
    const index = answers.findIndex((one) => one.key === key);
    if (index < 0) return false;
    setPicked(index);
    return true;
  };
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
    pickKey,
    pickNumber,
    step,
    canSend: answers.length === 0 ? open !== undefined : chosen !== undefined,
    send,
    ...(decisions === undefined || decisions.length < 2 ? {} : { decision: { at, of: decisions.length } }),
    open,
  };
}
