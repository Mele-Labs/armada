// A Job's Commander, as the boards draw it. **Mock only**: Fleet serves no Commander, and the
// decision that makes one is `.claude/decisions/2026-10-11-a-job-has-a-commander.md`. The draft
// carries what the Overview's panel reads and the acts on it, as `SessionsDraft` does for Sessions.
//
// A Commander claims nothing: it steers Drones, the plan and the caps, and cannot pass a gate,
// attest, merge, push or edit a Check. What it does is rows of its thread, and each act that changes
// the plan leaves a mark on the task it touched, so the canvas and the Plan tab say whose hand it was.

import type { TaskBy } from "./task";

/** What the Commander did on its own, inside its reach. */
export type CommanderAct =
  | { kind: "raise_cap"; cap: "turns" | "cost"; from: string; to: string }
  | { kind: "split"; task: string; into: readonly string[] }
  | { kind: "restart"; target: string };

/** One row of the Commander's thread. Fleet's wake, the owner's words, what it read, said and did. */
export type CommanderRow =
  /** Fleet woke it, and the fact that did. */
  | { id: string; at: string; kind: "woke"; why: string }
  | { id: string; at: string; kind: "you"; text: string }
  | { id: string; at: string; kind: "said"; text: string }
  /** A read, one call a row; calls that follow one another fold into one. */
  | { id: string; at: string; kind: "read"; text: string }
  | { id: string; at: string; kind: "act"; act: CommanderAct };

export type CommanderView = {
  /** `waiting` is a card out to the owner. */
  state: "idle" | "working" | "waiting";
  rows: readonly CommanderRow[];
  /** What it changed in the plan, by task id: the mark the task wears. */
  marks: Readonly<Record<string, TaskBy>>;
};

export type CommanderDraft = {
  /** By Job id. A Job with no entry has no Commander panel. */
  views: Readonly<Record<string, CommanderView>>;
  /** The owner's words, as the Commander's next input. */
  send: (jobId: string, text: string) => void;
  /** The owner's answer to the card it put to him: the card's key, then the option. */
  answer: (jobId: string, key: string, option: string) => void;
};
