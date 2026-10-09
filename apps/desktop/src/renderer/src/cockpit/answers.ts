// What a call can be answered with, from the keyboard: its answers as a numbered list, one picked,
// and Send. A Plan question is its decisions' options; a Session's call is the options of the item it
// waits on, or a reply in words; a pull request or main's red is Open and Poke where somebody owns it,
// and Send a Drone and Attach where nobody does. A call with no answer of its own opens what it is
// about. A Session's call is answered through Fleet (`answerWaiting`) and clears when Fleet takes it
// or says nothing holds it; a refusal stays on the card and goes to a toast. The rest do what their
// button did on the Dashboard, and clear the call, except where one opens something and leaves it
// standing. **From the press until Fleet answers, one act is out**: its control is `pending`, the rest
// of the card is held, and a second press is not a second send.

import { useEffect, useRef, useState } from "react";
import { actionOf, keyFor } from "@armada/components";
import type { AnswerWaiting } from "@armada/protocol";
import { isTerminal, titleOf } from "@armada/screens";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../../shared/bridge";
import { viewsOf } from "../merge-line";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { useSleepSource } from "../sleep";
import { attachPr } from "./claims";
import { answerWaiting } from "./answer-waiting";
import { dismissItem } from "./dismissed";
import { mainOwner, pullOwner, type Owner } from "./owner";
import { asksAnAgent } from "./standing";
import { sessionIdOf } from "./waiting";
import type { Hosts, Item } from "../Dashboard";

/** `key` is a standing answer's own key; the others are picked by their number. `says` is what it tells the agent. */
export type Answer = { id: string; label: string; run: () => void; key?: string; says?: string; description?: string; recommended?: boolean };

/** The picker an Attach opens: who a pull request can be attached to. */
export type Attaching = { candidates: readonly Owner[]; choose: (owner: Owner) => void; close: () => void };

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
  /** Whoever already owns what the call is about, where it is a pull request or main's red. */
  owner?: Owner;
  /** A reply in words, where the call takes one: a Session's question, or an item with no options. */
  reply?: (text: string, accepted?: () => void) => void;
  /** The picker, while an Attach has it open. */
  attaching?: Attaching;
  /** The control pressed and waiting on Fleet: an answer's id, `reply` or `dismiss`. Nothing else on the card acts meanwhile. */
  pending: string | undefined;
  /** Dismisses the call for good: on Fleet where a Session holds it, here where Fleet raised it. */
  dismiss: () => void;
};

/** What an act out to Fleet came to. */
type Told = { kind: "refused"; said: string } | { kind: "answered" | "gone" | "dismissed" | "attached" };

/** The pull request a call is about, as the card and the claim name it. */
function pullOf(item: Item, state: BridgeState): { number: number; branch: string; url: string; open: boolean } | undefined {
  const views = viewsOf(state);
  if (item.key.startsWith("pull:")) {
    const number = Number(item.key.slice("pull:".length));
    const pull = views.flatMap((view) => view.hub?.pulls ?? []).find((one) => one.number === number);
    return pull === undefined ? undefined : { number, branch: pull.branch, url: pull.url, open: true };
  }
  if (item.key.startsWith("main:")) {
    const main = views.find((view) => view.root === item.key.slice("main:".length))?.hub?.main;
    const merge = main?.state === "red" ? main.red.merge : undefined;
    return merge === undefined ? undefined : { number: merge.number, branch: merge.branch ?? "", url: merge.url ?? "", open: true };
  }
  return undefined;
}

/** Where `o` goes: the Job, the Session, or the pull request the call is about. */
function opener(item: Item, hosts: Hosts, state: BridgeState): (() => void) | undefined {
  if (item.job !== undefined) return () => hosts.onOpen(item.job!.id);
  const session = sessionIdOf(item.key);
  if (session !== undefined) return () => hosts.onOpenSession(session);
  const pull = pullOf(item, state);
  return pull === undefined || pull.url === "" ? undefined : () => hosts.onOpenLink(pull.url);
}

/**
 * Two answers that stand after the numbered ones on a question an agent asked: the best solution,
 * thought through, or the quickest reasonable path. **On a Session's permission they hand the
 * decision to run the command to the agent** (owner, 9 Oct 2026), so they say that. On a Session's
 * call each is an answer carrying `mode: "best" | "quick"`, which Fleet tells the agent as an
 * instruction; on a Plan question or a state Fleet raised, they only clear the card for now.
 */
function standingOf(send: (mode: "best" | "quick") => void, permission: boolean): Answer[] {
  return [
    {
      id: "best",
      key: keyFor("call_best"),
      label: actionOf("call_best").verb,
      says: permission ? "Tells the agent to weigh the command carefully and decide for itself whether to run it" : "Tells the agent to weigh the options and choose the best solution itself",
      run: () => send("best"),
    },
    {
      id: "quick",
      key: keyFor("call_quick"),
      label: actionOf("call_quick").verb,
      says: permission ? "Tells the agent to run the command if it is reasonable and keep moving" : "Tells the agent to take the quickest reasonable path and keep moving",
      run: () => send("quick"),
    },
  ];
}

/** An option's label without the agent's "(Recommended)", and whether it carried it. */
const RECOMMENDED = /\s*\(Recommended\)\s*$/i;
export const plainLabel = (label: string): string => label.replace(RECOMMENDED, "");

/** `finish` clears the call once Fleet has answered; `dismissed` does the same for a dismissal. */
export function useAnswering(item: Item, hosts: Hosts, state: BridgeState, finish: () => void, dismissed: () => void = finish): Answering {
  const [at, setAt] = useState(0);
  const [picked, setPicked] = useState<number>();
  const [attaching, setAttaching] = useState(false);
  const [pending, setPending] = useState<string>();
  // One act is out at a time, so a repeated key or a second press is not a second send.
  const out = useRef(false);
  // A new call starts from its first question with nothing picked.
  useEffect(() => (setAt(0), setPicked(undefined), setAttaching(false), setPending(undefined), void (out.current = false)), [item.key]);

  /**
   * Sends one act to Fleet, `id` the control pressed. The loop runs until Fleet answers: an acceptance
   * hands on to `done`, which leaves the control as it is while the card goes, and a refusal stops the
   * loop and says Fleet's words, with the card as it was.
   */
  const wait = (id: string, call: () => Promise<Told>, done: () => void = finish) => {
    if (out.current) return;
    out.current = true;
    setPending(id);
    const stop = () => {
      out.current = false;
      setPending(undefined);
    };
    void call().then(
      (told) => {
        if (told.kind === "refused") {
          stop();
          hosts.onTell?.(told.said);
        } else done();
      },
      stop,
    );
  };

  const decisions = item.decisions;
  const decision = decisions?.[at];
  const open = opener(item, hosts, state);
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const views = viewsOf(state);
  const pull = pullOf(item, state);
  const owner = item.key.startsWith("pull:")
    ? pullOwner(views, sessions, pull?.number ?? 0)
    : item.key.startsWith("main:")
      ? mainOwner(views, sessions, item.key.slice("main:".length))
      : undefined;

  // A Session's call answers through Fleet.
  const waiting = item.waiting;
  const sendWaiting = (id: string, body: Omit<AnswerWaiting, "session_id" | "item_id">, accepted?: () => void) => {
    if (waiting === undefined) return;
    wait(id, () => answerWaiting({ session_id: waiting.sessionId, item_id: waiting.item.id, ...body }), () => (accepted?.(), finish()));
  };

  /** A short nudge to whoever owns the pull request, through the path that reaches them. */
  const poke = (who: Owner) => {
    const said = `Pull request #${pull?.number ?? ""}'s checks failed. Address them and push to the pull request.`;
    if (who.kind === "session") {
      draft?.send(who.id, { text: said, files: [], sketches: [], tags: [] });
      finish();
    } else {
      wait("Poke", async (): Promise<Told> => {
        const done = await window.armada.redirectDrone(who.id, said);
        return done.ok ? { kind: "answered" } : { kind: "refused", said: refusalWords(done) };
      });
    }
  };

  const attach = (to: Owner) => {
    setAttaching(false);
    if (pull === undefined) return;
    // The card stays: what it said is now somebody's, and its answers change with the owner.
    wait(
      "Attach",
      async (): Promise<Told> => {
        const done = await attachPr(pull.number, to);
        return done.attached ? { kind: "attached" } : { kind: "refused", said: done.said };
      },
      () => {
        out.current = false;
        setPending(undefined);
      },
    );
  };
  const candidates: Owner[] = [
    ...state.jobs.filter((job) => !isTerminal(job)).map((job): Owner => ({ kind: "job", id: job.id, title: titleOf(job) })),
    ...sessions.filter((one) => one.dead === undefined).map((one): Owner => ({ kind: "session", id: one.id, title: one.title ?? one.id })),
  ];

  // A Sleep decision is kept, or changed in words; either way it is reviewed and the call leaves.
  const slept = useSleepSource();
  const sleep = item.sleep;
  const reviewed = (id: string) => slept?.review(id);

  const answer = (label: string, run: () => void, extra: Partial<Answer> = {}): Answer => ({ id: label, label, run, ...extra });
  const actsOf = (): Answer[] => {
    if (sleep !== undefined) return [answer("Keep", () => (reviewed(sleep.id), finish()))];
    if (waiting !== undefined) {
      const given = waiting.item;
      // A walk needs no choice: approving is what Fleet does with an answer that names none.
      if (given.source === "walk") return [answer("Approve", () => sendWaiting("Approve", {}))];
      return (given.options ?? []).map((option, at) =>
        answer(plainLabel(option.label), () => sendWaiting(plainLabel(option.label), { choice: at }), {
          ...("description" in option && typeof option.description === "string" ? { description: option.description } : {}),
          ...(RECOMMENDED.test(option.label) ? { recommended: true } : {}),
        }),
      );
    }
    // Somebody is already on it: go and see that they are addressing it, or nudge them. No new Drone.
    if ((item.key.startsWith("main:") || item.key.startsWith("pull:")) && pull !== undefined) {
      if (owner !== undefined) {
        const noun = owner.kind === "session" ? "Session" : "Job";
        return [
          answer(`Open the ${noun}`, () => (owner.kind === "session" ? hosts.onOpenSession(owner.id) : hosts.onOpen(owner.id))),
          answer("Poke", () => poke(owner)),
        ];
      }
      const drone = item.key.startsWith("main:")
        ? answer("Hand to a Job", () => (hosts.onFix?.({ root: item.key.slice("main:".length) }), finish()))
        : answer("Send a Drone", () => (hosts.onPropose?.(`The checks on pull request #${pull.number} failed. Read the failure, say what you would change, and push only once I agree.`), finish()));
      return [drone, answer("Attach", () => setAttaching(true))];
    }
    if (item.kind === "Check failed") return [answer("Retry", finish)];
    if (item.kind === "Drone stuck") return [answer("Restart the step", finish), answer("Fresh Drone", finish)];
    return [];
  };

  const numbered: Answer[] =
    decision !== undefined
      ? decision.options.map((option) => ({
          id: option.id,
          label: option.label,
          run: () => (at + 1 < decisions!.length ? (setAt(at + 1), setPicked(undefined)) : finish()),
        }))
      : actsOf();
  const permission = waiting?.item.source === "permission";
  const standing = asksAnAgent(item) ? standingOf(waiting === undefined ? () => finish() : (mode) => sendWaiting(mode, { mode }), permission) : [];
  const answers = [...numbered, ...standing];

  // One answer is the answer: Enter sends it without a number first.
  const chosen = picked ?? (answers.length === 1 ? 0 : undefined);
  const held = pending !== undefined;
  const pick = (index: number) => !held && answers[index] !== undefined && setPicked(index);
  const pickNumber = (digit: number) => digit <= numbered.length && pick(digit - 1);
  const pickKey = (key: string) => {
    const index = answers.findIndex((one) => one.key === key);
    if (index < 0) return false;
    if (!held) setPicked(index);
    return true;
  };
  const step = (by: 1 | -1) =>
    !held &&
    setPicked(chosen === undefined ? (by === 1 ? 0 : answers.length - 1) : Math.min(answers.length - 1, Math.max(0, chosen + by)));
  const send = () => {
    if (held) return;
    if (answers.length === 0) open?.();
    else if (chosen !== undefined) answers[chosen]?.run();
  };
  const replies = waiting !== undefined && waiting.item.source !== "walk" && waiting.item.source !== "permission";

  return {
    ask: decision?.question ?? item.fact,
    answers,
    picked: chosen,
    pick,
    pickKey,
    pickNumber,
    step,
    canSend: !held && (answers.length === 0 ? open !== undefined : chosen !== undefined),
    send,
    ...(decisions === undefined || decisions.length < 2 ? {} : { decision: { at, of: decisions.length } }),
    open,
    ...(owner === undefined ? {} : { owner }),
    ...(sleep !== undefined
      ? { reply: (text: string, accepted?: () => void) => (slept?.override(sleep.id, text), reviewed(sleep.id), accepted?.(), finish()) }
      : {}),
    ...(replies ? { reply: (text: string, accepted?: () => void) => sendWaiting("reply", { text }, accepted) } : {}),
    ...(attaching ? { attaching: { candidates, choose: attach, close: () => setAttaching(false) } } : {}),
    pending,
    dismiss: () => wait("dismiss", () => dismissItem(item), dismissed),
  };
}
