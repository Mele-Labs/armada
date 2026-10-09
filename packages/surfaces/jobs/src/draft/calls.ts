// What the Dashboard's calls carry beyond the Board, by Job id: a Plan decision's options, the words
// leading up to a question, the request, and where the Job lives. **Mock only**: Fleet publishes none
// of it yet, so on a real Fleet a call draws from the Board's own fields alone.

export type CallKindView = "drone" | "check" | "judge";

export type CallAskView =
  | { key: string; kind: "plan"; context?: readonly string[]; decisions: readonly { id: string; question: string; options: readonly { id: string; label: string }[] }[] }
  | { key: string; kind: "judge" | "drone"; name: string; text: string; context?: readonly string[]; target?: string };

/** `context` is the words leading up to it: the Drone's or Judge's own, or a Check's output tail. */
export type CallIssueView = { key: string; of: CallKindView; name: string; text: string; said: string; context?: readonly string[]; target?: string };

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
