// What the Now panel beside the Overview canvas draws, as data. **Mock only**:
// Fleet publishes no plan interview, and its asks, issues and live rows are not
// one read yet, so the real Fleet puts nothing here and the panel is absent.
//
// The panel's `onOpen` and `onAnswer` are the host's. `target` names what a row
// opens (a Drone id, a Check's name, a Judge's step); a press with no host to
// open it is a stub.

export type NowKindView = "drone" | "check" | "judge";

export type NowAskView =
  | { key: string; kind: "plan"; question: string; options: readonly { id: string; label: string }[] }
  | { key: string; kind: "judge" | "drone"; name: string; text: string; target?: string };

export type NowIssueView = { key: string; of: NowKindView; name: string; text: string; said: string; target?: string };

export type NowRunningView = {
  key: string;
  of: NowKindView;
  name: string;
  line?: string;
  state: "running" | "passed" | "failed";
  target?: string;
};

export type NowView = {
  asks?: readonly NowAskView[];
  issues?: readonly NowIssueView[];
  running?: readonly NowRunningView[];
};
