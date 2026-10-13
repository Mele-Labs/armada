// What the Dashboard's calls carry beyond the Board, by Job id: a Plan decision's options, the words
// leading up to a question, the request, and where the Job lives. **The mock's draft carries all of
// it**; a real Fleet's is the Now it serves (`now-calls.ts`): the asks and issues, and none of the rest.

import type { AskedOf } from "./now";

export type CallKindView = "drone" | "check" | "judge";

export type CallAskView =
  /**
   * What the Job's Commander cannot do itself, as the one card it puts to the owner. The decisions are
   * the card's question and answers, and an answer names `card` beside the option.
   */
  | { key: string; kind: "commander"; card: string; context?: readonly string[]; decisions: readonly { id: string; question: string; options: readonly { id: string; label: string }[] }[] }
  | { key: string; kind: "plan"; context?: readonly string[]; decisions: readonly { id: string; question: string; options: readonly { id: string; label: string }[] }[] }
  | { key: string; kind: "judge" | "drone"; name: string; text: string; context?: readonly string[]; target?: string; asked?: AskedOf };

/** `context` is the words leading up to it: the Drone's or Judge's own, or a Check's output tail. */
export type CallIssueView = {
  key: string;
  /** `clash` is main broken, or another Job writing the same files. */
  of: CallKindView | "clash";
  name: string;
  text: string;
  said: string;
  context?: readonly string[];
  target?: string;
  /** A Check: the tests that failed by name, the last lines of its output, and how many Checks are not finished. */
  failing?: readonly string[];
  output?: readonly string[];
  unfinished?: number;
  /** A ruling that said failed: which attempt, what brought it, and the Judge's reason. */
  ruling?: { attempt: string; trigger: string; reason: string };
  /** A Drone gone quiet: for how many minutes, and its last few actions, oldest first. */
  quiet?: { minutes: number; last: readonly string[] };
  /** What Fleet's issue says can be done about it: Retry step, and words to a stuck Drone. Absent on the mock's, whose acts follow its kind. */
  acts?: readonly { glyph: "retry" | "skip" | "skip_all" | "redirect" | "retry_step"; said: string }[];
  /** Main broken or another Job in the same files: what it is, the paths, and the other Job where there is one. */
  clash?: { what: string; paths: readonly string[]; other?: { jobId: string; title: string } };
};

export type CallRunningView = {
  key: string;
  of: CallKindView;
  name: string;
  line?: string;
  /** The canvas step it belongs to. */
  step?: { id: string; name: string };
  /** The last few lines of its output, oldest first. */
  tail?: readonly string[];
  state: "running" | "passed" | "failed";
  target?: string;
};

/** A Job or an issue named in `about`, drawn as a chip that opens it. */
export type AboutLink = { kind: "job" | "issue"; number: number; title: string; jobId?: string; url?: string };

/** Files named in `about`, each pressed to read what the Job changed in it. `lines` is the file's patch. */
export type AboutFiles = { files: readonly { path: string; lines: readonly { kind: "hunk" | "added" | "removed" | "context"; text: string }[] }[] };

export type CallView = {
  /** Drawn from what Fleet serves: an issue's acts are not the mock's stubs, and the card opens the Job. */
  served?: true;
  /** What the Job was dispatched with. */
  request?: string;
  /** Where the Job lives, as label and value: repository, area, from, branch, touched. */
  about?: readonly (readonly [string, string | AboutLink | AboutFiles])[];
  asks?: readonly CallAskView[];
  issues?: readonly CallIssueView[];
  running?: readonly CallRunningView[];
};
