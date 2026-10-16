// What a call can be answered with, from the keyboard: its answers as a numbered list, one picked,
// and Send. A Plan question is its decisions' options; a Session's call is the options of the item it
// waits on, else what its act says (run a command, open a pull request or a page), and a reply in
// words beside either; a pull request or main's red is Open and Poke where somebody owns it,
// and Send a Drone and Attach where nobody does. A call with no answer of its own opens what it is
// about. A Session's call is answered through Fleet (`answerWaiting`) and clears when Fleet takes it
// or says nothing holds it; a refusal stays on the card and goes to a toast. The rest do what their
// button did on the Dashboard, and clear the call, except where one opens something and leaves it
// standing. **From the press until Fleet answers, one act is out**: its control is `pending`, the rest
// of the card is held, and a second press is not a second send.

import { useEffect, useRef, useState } from "react";
import { actionOf, isPressed, keyFor, useCodeRun, type CodeRun, type Press } from "@armada/components";
import type { AnswerWaiting } from "@armada/protocol";
import { isTerminal, titleOf } from "@armada/screens";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../../shared/bridge";
import { restartOf } from "../build-change";
import { viewsOf } from "../merge-line";
import { runnerOf } from "../session-runner";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { useSleepSource } from "../sleep";
import { attachPr } from "./claims";
import { answerJobAsk } from "./answer-job";
import { answerWaiting } from "./answer-waiting";
import { dismissItem, dismissWaiting } from "./dismissed";
import { fixJob, type JobFix } from "./fix-job";
import { ownerOfCall, type Owner } from "./owner";
import { asksAnAgent } from "./standing";
import { sessionIdOf } from "./waiting";
import type { Hosts, Item } from "../Dashboard";

/**
 * `key` is a standing answer's own key; the others are picked by their number. `says` is what it tells the agent.
 * `once` marks an act, which runs on one press, its click or its number: an option is picked and then sent,
 * because a reply can go with it, which `run` takes as `words`.
 */
export type Answer = { id: string; label: string; run: (words?: string) => void; key?: string; act?: string; says?: string; description?: string; recommended?: boolean; once?: boolean };

/** How long a command that passed stays on its card, with its output and exit, before the card goes. */
const PASSED_FOR = 1400;
/** After the card begins to leave, how long before the item is settled and the agent told, so the card's exit is not cut short. */
const LEAVES = 300;

/** The picker an Attach opens: who a pull request can be attached to. */
export type Attaching = { candidates: readonly Owner[]; choose: (owner: Owner) => void; close: () => void };

export type Answering = {
  /** The question asked now: a Plan decision's, or the call's own fact. */
  ask: string;
  /** What an errand's act names, a command or a page: drawn whole on the card, not on the key. */
  target?: string;
  answers: readonly Answer[];
  /** Which answer is picked, where one is. A lone answer starts picked. */
  picked: number | undefined;
  pick: (index: number) => void;
  /** Picks the standing answer a press names, where there is one — by its act, so a rebound key picks it. */
  pickKey: (press: Press) => boolean;
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
  /** Opens the walk a Session waits on approval of, where the call is one. Opening it answers nothing. */
  openWalk?: () => void;
  /** Whoever already owns what the call is about, where it is a pull request or main's red. */
  owner?: Owner;
  /** A reply in words, where the call takes one: a Session's question, or an item with no options. */
  reply?: (text: string, accepted?: () => void) => void;
  /** The picker, while an Attach has it open. */
  attaching?: Attaching;
  /** The run of a command the card runs from its own key, where it can: its output and how it ended, drawn under the command. */
  run?: CodeRun;
  /** The control pressed and waiting on Fleet: an answer's id, `reply` or `dismiss`. Nothing else on the card acts meanwhile. */
  pending: string | undefined;
  /** Dismisses the call for good: on Fleet where a Session holds it, here where Fleet raised it. */
  dismiss: () => void;
  /** Removes an item the agent stated, with the person's reason (empty is fine); Fleet tells the agent. Absent on any other call. */
  remove?: (reason: string) => void;
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

/**
 * What a poke about main's red says: main is red, after which merge, and what failed. The merge has landed, so its owner is asked for a fix against main, not a push to it.
 */
function mainRedSaid(views: ReturnType<typeof viewsOf>, root: string): string {
  const main = views.find((view) => view.root === root)?.hub?.main;
  const red = main?.state === "red" ? main.red : undefined;
  const merged = red?.merge === undefined ? "" : ` after pull request #${red.merge.number} merged`;
  const failed =
    red === undefined
      ? []
      : [{ check: red.check, test: red.test }, ...(red.also ?? [])].map((one) => `- ${one.check}${one.test === undefined ? "" : `: ${one.test}`}`);
  return [`main is red${merged}. Find why and open a pull request that fixes main.`, ...failed].join("\n");
}

/** Where `o` goes: the Job, the Session, or the pull request the call is about. */
function opener(item: Item, hosts: Hosts, state: BridgeState): (() => void) | undefined {
  if (item.job !== undefined) return () => hosts.onOpen(item.job!.id);
  const slept = item.sleep?.sessionId;
  if (slept !== undefined) return () => hosts.onOpenSession(slept);
  const sleptJob = item.sleep?.jobId;
  if (sleptJob !== undefined) return () => hosts.onOpen(sleptJob);
  const session = sessionIdOf(item.key);
  if (session !== undefined) return () => hosts.onOpenSession(session);
  const pull = pullOf(item, state);
  return pull === undefined || pull.url === "" ? undefined : () => hosts.onOpenLink(pull.url);
}

/**
 * Two answers that stand after the numbered ones on a question an agent asked (`asksAnAgent`, so
 * never on a Session's errand line): the best solution, thought through, or the work finished and landed. **On a Session's permission they hand the
 * decision to run the command to the agent** (owner, 9 Oct 2026), so they say that. On a Session's
 * call each is an answer carrying `mode: "best" | "quick"`, which Fleet tells the agent as an
 * instruction; on a Plan question or a state Fleet raised, they only clear the card for now.
 */
function standingOf(send: (mode: "best" | "quick") => void, permission: boolean): Answer[] {
  return [
    {
      id: "best",
      key: keyFor("call_best"),
      act: "call_best",
      label: actionOf("call_best").verb,
      says: permission ? "Tells the agent to weigh the command carefully and decide for itself whether to run it" : "Tells the agent to weigh the options and choose the best solution itself",
      run: () => send("best"),
    },
    {
      id: "quick",
      key: keyFor("call_quick"),
      act: "call_quick",
      label: actionOf("call_quick").verb,
      says: permission ? "Tells the agent to run the command if it is reasonable and keep moving" : "Tells the agent to finish the work and land it, skipping the formalities",
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

  // A command a Session waits on is run here, where the Session works, by the path its ledger runs it by.
  // A clean exit stays on the card for a beat, then the card goes and the item is settled and the agent told.
  const home = sessions.find((one) => one.id === item.waiting?.sessionId);
  const runner =
    draft === undefined || home === undefined
      ? undefined
      : runnerOf(draft, home, (settle) => {
          window.setTimeout(() => {
            finish();
            window.setTimeout(settle, LEAVES);
          }, PASSED_FOR);
        });
  const ran = useCodeRun(runner);
  // Nothing starts it again while it runs or has passed and the card is about to go, from the block's play control or the key.
  const guarded: CodeRun = { ...ran, start: (command) => (ran.phase.state === "running" || (ran.phase.state === "ended" && ran.phase.code === 0) ? undefined : ran.start(command)) };
  // From the press until it ends badly the command holds the card; one that passed holds it until the card has gone.
  const running = ran.phase.state === "running" || (ran.phase.state === "ended" && ran.phase.code === 0);
  const pull = pullOf(item, state);
  const owner = ownerOfCall(views, sessions, item.key);

  // A Session's call answers through Fleet.
  const waiting = item.waiting;
  const restart = restartOf(waiting?.item.act);
  const sendWaiting = (id: string, body: Omit<AnswerWaiting, "session_id" | "item_id">, accepted?: () => void) => {
    if (waiting === undefined) return;
    wait(id, () => answerWaiting({ session_id: waiting.sessionId, item_id: waiting.item.id, ...body }), () => (accepted?.(), finish()));
  };

  /** Best or quick on a Job's ask. An ask Fleet holds no question for, such as the mock's, only clears. */
  const sendToJob = (mode: "best" | "quick") => {
    const call = item.job === undefined ? undefined : answerJobAsk(item.job.id, item.asked, mode);
    if (call === undefined) return finish();
    wait(mode, call);
  };

  /** A short nudge to whoever owns the pull request or main's red, saying which, through the path that reaches them. */
  const poke = (who: Owner) => {
    const said = item.key.startsWith("main:") ? mainRedSaid(views, item.key.slice("main:".length)) : `Pull request #${pull?.number ?? ""}'s checks failed. Address them and push to the pull request.`;
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

  // A Sleep decision is agreed with, or changed in words; either way it is reviewed and the call leaves.
  const slept = useSleepSource();
  const sleep = item.sleep;
  const reviewed = (id: string) => slept?.review(id);

  /** Where a pull request the Session names is, from its own attachments or the repository's open ones. */
  const pullAddress = (number: number): string | undefined => {
    const attached = sessions.find((one) => one.id === waiting?.sessionId)?.attachments.find((one) => one.kind === "pull_request" && one.number === number);
    if (attached?.kind === "pull_request") return attached.address;
    return views.flatMap((view) => view.hub?.pulls ?? []).find((one) => one.number === number)?.url;
  };
  const copy = (command: string) => {
    try {
      void navigator.clipboard.writeText(command);
      hosts.onTell?.("Copied");
    } catch {
      // Not copied: the command stays on the card to be read.
    }
  };

  // What went wrong on a Job is fixed from its card: each fix is one press, out to Fleet until it answers.
  const issue = item.issue;
  const jobId = item.job?.id;
  const fix = (request: JobFix): Promise<Told> =>
    fixJob(request).then((done): Told => (done.ok ? { kind: "answered" } : { kind: "refused", said: done.said }));
  /** The words go to the Drone that is live, or start the step again with them where none is. */
  const redirect = (words: string): JobFix | undefined =>
    jobId === undefined
      ? undefined
      : issue?.of === "drone"
        ? { kind: "redirect_one_drone", jobId, words, ...(issue.target === undefined ? {} : { droneId: issue.target }) }
        : { kind: "restart_step", jobId, words };

  const answer = (label: string, run: (words?: string) => void, extra: Partial<Answer> = {}): Answer => ({ id: label, label, run, ...extra });
  const oneTap = (label: string, run: () => void): Answer => answer(label, run, { once: true });
  const fixesOf = (one: NonNullable<typeof issue>, id: string): Answer[] => {
    const tap = (label: string, request: JobFix, says?: string): Answer =>
      answer(label, () => wait(label, () => fix(request), finish), { once: true, ...(says === undefined ? {} : { says }) });
    // A call Fleet serves draws only the acts its issue names. Retry step is one; the other is the box.
    if (item.served === true) return (one.acts ?? []).filter((act) => act.glyph === "retry_step").map(() => tap("Retry step", { kind: "restart_step", jobId: id }));
    if (one.of === "check") {
      const check = one.target ?? one.name;
      return [
        tap("Retry", { kind: "retry_check", jobId: id, check }),
        tap("Skip", { kind: "skip_check", jobId: id, check }, "Ends it as skipped, not passed"),
        ...((one.unfinished ?? 0) > 1 ? [tap("Skip all", { kind: "skip_checks", jobId: id }, "Ends every unfinished Check as skipped, not passed")] : []),
      ];
    }
    if (one.of === "clash") {
      const other = one.clash?.other;
      return other === undefined ? [] : [oneTap("Open the other Job", () => hosts.onOpen(other.jobId))];
    }
    return [tap("Retry step", { kind: "restart_step", jobId: id })];
  };
  const actsOf = (): Answer[] => {
    // Agreeing is the whole answer; where it was decided opens from the card's own Open.
    if (sleep !== undefined) return [oneTap("Agree", () => (reviewed(sleep.id), finish()))];
    if (waiting !== undefined) {
      const given = waiting.item;
      // A walk needs no choice: approving is what Fleet does with an answer that names none.
      if (given.source === "walk") return [oneTap("Approve", () => sendWaiting("Approve", {}))];
      // A plan is approved from its own call: the version is the one in the item's id. Sending changes needs the plan, so that is in the Session.
      if (given.source === "plan") return draft?.work === undefined ? [] : [oneTap("Approve", () => draft.work?.approve(waiting.sessionId))];
      // A judge's concern reaches the agent only on his press; dismissing it tells the agent nothing.
      if (given.source === "judge") {
        const work = draft?.work;
        return work === undefined ? [] : [oneTap("Send to session", () => work.sendConcern(waiting.sessionId, given.id)), oneTap("Dismiss", () => work.dismissConcern(waiting.sessionId, given.id))];
      }
      // A Session that said it is done is ended, or kept: Keep going drops the call and leaves the Session as it is.
      if (given.source === "done") {
        return [
          oneTap("End session", () => (draft?.close?.(waiting.sessionId), finish())),
          oneTap("Keep going", () =>
            wait("Keep going", async (): Promise<Told> => {
              const kept = await dismissWaiting(waiting.sessionId, given.id);
              return kept.kind === "dismissed" ? { kind: "dismissed" } : kept;
            }, dismissed),
          ),
        ];
      }
      if (given.options !== undefined && given.options.length > 0) {
        return given.options.map((option, at) =>
          answer(plainLabel(option.label), (words) => sendWaiting(plainLabel(option.label), { choice: at, ...(words === undefined || words === "" ? {} : { text: words }) }), {
            ...("description" in option && typeof option.description === "string" ? { description: option.description } : {}),
            ...(RECOMMENDED.test(option.label) ? { recommended: true } : {}),
          }),
        );
      }
      // An errand for him follows its act. A line with none is a word to the agent, in the text box.
      const act = given.act;
      if (act?.kind === "run") {
        // A slash command is the session's own: it is told, as a reply. Anything else he runs himself.
        // The key says the verb; the command itself is on the card, whole and in its own case.
        if (act.target.startsWith("/")) return [oneTap("Run", () => sendWaiting("Run", { text: act.target }))];
        // A restart is the Fleet panel's build switch: one key, nothing to copy or say. The runner switches it.
        if (restart !== undefined) return runner === undefined ? [] : [answer(restart.label, () => guarded.start(act.target), { once: true })];
        const ended = ran.phase.state === "ended" && ran.phase.code !== 0;
        return [
          ...(runner === undefined ? [] : [answer("Run", () => guarded.start(act.target), { once: true, ...(ended ? { label: "Run again" } : {}) })]),
          oneTap("Copy", () => copy(act.target)),
        ];
      }
      if (act?.kind === "approve_pr") {
        const number = act.target.replace(/^#/, "");
        const address = pullAddress(Number(number));
        return [oneTap(`Open #${number}`, () => (address === undefined ? hosts.onOpenSession(waiting.sessionId) : hosts.onOpenLink(address)))];
      }
      if (act?.kind === "walk") return [oneTap("Open the page", () => (draft?.openWindow === undefined ? hosts.onOpenLink(act.target) : draft.openWindow(waiting.sessionId, act.target)))];
      return [];
    }
    // Somebody is already on it: go and see that they are addressing it, or nudge them. No new Drone.
    if ((item.key.startsWith("main:") || item.key.startsWith("pull:")) && pull !== undefined) {
      if (owner !== undefined) {
        const noun = owner.kind === "session" ? "Session" : "Job";
        return [
          oneTap(`Open the ${noun}`, () => (owner.kind === "session" ? hosts.onOpenSession(owner.id) : hosts.onOpen(owner.id))),
          oneTap("Poke", () => poke(owner)),
        ];
      }
      const drone = item.key.startsWith("main:")
        ? oneTap("Hand to a Job", () => (hosts.onFix?.({ root: item.key.slice("main:".length) }), finish()))
        : oneTap("Send a Drone", () => (hosts.onPropose?.(`The checks on pull request #${pull.number} failed. Read the failure, say what you would change, and push only once I agree.`), finish()));
      return [drone, oneTap("Attach", () => setAttaching(true))];
    }
    if (issue !== undefined && jobId !== undefined) return fixesOf(issue, jobId);
    return [];
  };

  const numbered: Answer[] =
    decision !== undefined
      ? decision.options.map((option) => ({
          id: option.id,
          label: option.label,
          run: () =>
            at + 1 < decisions!.length
              ? (setAt(at + 1), setPicked(undefined))
              : item.answerCard === undefined
                ? finish()
                : wait(option.id, () => item.answerCard!(option.id)),
        }))
      : actsOf();
  const permission = waiting?.item.source === "permission";
  const standing = asksAnAgent(item) ? standingOf(waiting === undefined ? sendToJob : (mode) => sendWaiting(mode, { mode }), permission) : [];
  const answers = [...numbered, ...standing];

  // One answer is the answer: Enter sends it without a number first. Where every answer is an act, Enter runs the first.
  const chosen = picked ?? (answers.length === 1 || (answers.length > 0 && answers.every((one) => one.once === true)) ? 0 : undefined);
  const shown = pending ?? (running ? "Run" : undefined);
  const held = shown !== undefined;
  const pick = (index: number) => !held && answers[index] !== undefined && setPicked(index);
  // An act runs on its number; an option is picked, and sent after.
  const pickNumber = (digit: number) => {
    if (digit > numbered.length || held) return;
    const one = answers[digit - 1];
    if (one?.once === true) one.run();
    else pick(digit - 1);
  };
  const pickKey = (press: Press) => {
    const index = answers.findIndex((one) => one.act !== undefined && isPressed(one.act, press));
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
  // A first message is sent in the Session's own box, so the card has no reply and its one act opens the Session.
  const replies = waiting !== undefined && restart === undefined && waiting.item.source !== "walk" && waiting.item.source !== "permission" && waiting.item.source !== "first_message";

  return {
    ask: decision?.question ?? item.fact,
    ...(waiting?.item.act?.kind === "run" || waiting?.item.act?.kind === "walk" ? { target: waiting.item.act.target } : {}),
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
    ...(waiting?.item.act?.kind === "walk" ? { openWalk: () => draft?.openWindow?.(waiting.sessionId, waiting.item.act!.target) } : {}),
    ...(owner === undefined ? {} : { owner }),
    ...(sleep !== undefined
      ? { reply: (text: string, accepted?: () => void) => (slept?.override(sleep.id, text), reviewed(sleep.id), accepted?.(), finish()) }
      : {}),
    ...(issue !== undefined && jobId !== undefined && (item.served !== true || (issue.acts?.length ?? 0) > 0)
      ? {
          reply: (text: string, accepted?: () => void) => {
            const request = redirect(text);
            if (request !== undefined) wait("reply", () => fix(request), () => (accepted?.(), finish()));
          },
        }
      : {}),
    ...(replies ? { reply: (text: string, accepted?: () => void) => sendWaiting("reply", { text }, accepted) } : {}),
    ...(attaching ? { attaching: { candidates, choose: attach, close: () => setAttaching(false) } } : {}),
    ...(waiting?.item.act?.kind === "run" && runner !== undefined && restart === undefined && !waiting.item.act.target.startsWith("/") ? { run: guarded } : {}),
    pending: shown,
    // A Sleep decision is dismissed by reviewing it on Fleet, so it never comes back.
    dismiss: () => (sleep === undefined ? wait("dismiss", () => dismissItem(item), dismissed) : (reviewed(sleep.id), dismissed())),
    ...(waiting?.item.source === "agent" ? { remove: (reason: string) => wait("dismiss", () => dismissItem(item, reason), dismissed) } : {}),
  };
}
