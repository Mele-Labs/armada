// What the Dashboard's calls carry beyond the Board, by Job id: a Plan decision's options, the words
// leading up to a question, the request, and where the Job lives. **Mock only**: Fleet publishes none
// of it yet, so on a real Fleet a call draws from the Board's own fields alone.

export type CallKindView = "drone" | "check" | "judge";

export type CallAskView =
  | { key: string; kind: "plan"; context?: readonly string[]; decisions: readonly { id: string; question: string; options: readonly { id: string; label: string }[] }[] }
  | { key: string; kind: "judge" | "drone"; name: string; text: string; context?: readonly string[]; target?: string };

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
  /** What the Job was dispatched with. */
  request?: string;
  /** Where the Job lives, as label and value: repository, area, from, branch, touched. */
  about?: readonly (readonly [string, string | AboutLink | AboutFiles])[];
  asks?: readonly CallAskView[];
  issues?: readonly CallIssueView[];
  running?: readonly CallRunningView[];
};
