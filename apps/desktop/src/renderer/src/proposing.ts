// The one proposal this window is on, from the press to what it became.
//
// **Its own file for the reason `dispatch.ts` is one**: it is a subject rather
// than a line of wiring, and `App.tsx` is over the length the gate warns at.
// The two decisions in it are pure functions, tested in node beside
// `where-open.ts`'s fold; the hook is the state around them.
//
// The screen it drives, and why it exists, is the owner's decision of
// 30 Sep 2026, *the wait is a destination*.

import { useRef, useState } from "react";
import type { ProposalAnswer, ProposedJob } from "@armada/components";
import type { JobSummary, ProposalInFlight } from "@armada/protocol";
import type { Answered } from "@armada/screens";

/** One proposal, as the screen reads it. */
export type ProposalRun = {
  /**
   * What was sent, as sent. `null` where this window did not send it — a window
   * reopened while its own proposer is still out.
   */
  sent: string | null;
  at: ProposalAnswer;
};

/** What the window needs to show a proposal, and the two ways it stops. */
export type Proposing = {
  /** The proposal on screen, or `null` where the window is showing none. */
  run: ProposalRun | null;
  /**
   * Send one. The run goes to `reading` at once, so the caller can leave the
   * composer in the same press, and `ask` answers into it.
   *
   * **A refusal the screen has no drawing for ends the run** rather than
   * standing on it: `answeredAs` hands those back as `unasked` with an
   * `Outcome`, which the app draws in its own failure pipeline.
   */
  start: (sent: string, ask: () => Promise<Answered>) => void;
  /**
   * Stop showing it. **Leaving is leaving**: the same call is not adopted a
   * second time, so the rail works while a proposer is out, and an answer that
   * lands afterwards is dropped — the Jobs are on the board either way.
   */
  leave: () => void;
};

/**
 * The proposal this window is showing.
 *
 * `proposing` is `BridgeState.proposing`, matched by main on this window's own
 * `client_ref`. It is what lets a window reopened mid-call land here rather than
 * on a board that says nothing about the money being spent.
 */
export function useProposing(proposing: ProposalInFlight | null): Proposing {
  const [run, setRun] = useState<ProposalRun | null>(null);
  // The published call this window has already shown and left. State, because
  // what is drawn depends on it.
  const [left, setLeft] = useState<string | null>(null);
  // Which press the run on screen belongs to. A ref: nothing renders from it,
  // and an answer arriving for an older press must be dropped rather than
  // pulling somebody back to a screen they left.
  const press = useRef(0);
  // What is published now, read through a ref because `leave` is called from
  // effects bound once — `onSummoned`'s. A closure from the first render would
  // mark no call left behind and the screen would adopt itself back.
  const latest = useRef(proposing);
  latest.current = proposing;

  function start(sent: string, ask: () => Promise<Answered>): void {
    press.current += 1;
    const mine = press.current;
    setRun({ sent, at: { at: "reading" } });
    void ask().then(
      (read) => {
        if (press.current !== mine) return;
        setRun(read.proposal.at === "unasked" ? null : { sent, at: read.proposal });
      },
      (thrown: unknown) => {
        // A screen left on `reading` is worse than the throw: nothing on it
        // says the call is dead. The throw carries on to `watchUncaught`.
        if (press.current === mine) setRun(null);
        throw thrown;
      },
    );
  }

  function leave(): void {
    press.current += 1;
    setLeft(latest.current?.proposal_id ?? null);
    setRun(null);
  }

  return { run: showing(run, proposing, left), start, leave };
}

/**
 * Which proposal is on screen: this window's own run, or a call it sent before
 * it was reopened.
 *
 * **Adopted once.** A window and the daemon have independent lifetimes, so a
 * reopened window has a proposer out and no run for it — and a person who then
 * presses the rail has left, which is why `left` exists rather than the
 * adoption simply repeating on every render.
 */
export function showing(
  run: ProposalRun | null,
  proposing: ProposalInFlight | null,
  left: string | null,
): ProposalRun | null {
  if (run !== null) return run;
  if (proposing === null || proposing.proposal_id === left) return null;
  return { sent: null, at: { at: "reading" } };
}

/**
 * The proposal's rows, against the board's own reading of each Job.
 *
 * **The answer is the press's and the status is Fleet's**, and approving one
 * changes the status without changing the answer. A row drawn from the answer
 * alone would say `needs approval` for as long as the screen stayed open, under
 * a Job already queued.
 */
export function asTheBoardSays(at: ProposalAnswer, jobs: readonly JobSummary[]): ProposalAnswer {
  if (at.at !== "proposed") return at;
  return { ...at, jobs: at.jobs.map((job) => onTheBoard(job, jobs)) };
}

function onTheBoard(job: ProposedJob, jobs: readonly JobSummary[]): ProposedJob {
  const row = jobs.find((one) => one.id === job.id);
  return row === undefined || row.status === job.status ? job : { ...job, status: row.status };
}
