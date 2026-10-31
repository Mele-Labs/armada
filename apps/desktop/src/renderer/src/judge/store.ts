// A judge on demand: Fleet reads one artifact of a Job, once, when the person presses Judge on it, and answers with notes. **Provisional**: the
// request and the record below are the shape Bridge wants from Fleet, held here until the wire carries them; the mock Fleet (`mock/judge-fleet.ts`)
// answers through `provideJudge`. A record arrives whole and replaces the one before it for the same artifact.

import { useSyncExternalStore } from "react";

export type JudgeKind = "job" | "plan" | "pull_request" | "commit" | "file" | "page" | "doc" | "sketch" | "command" | "subagent" | "issue" | "note";

/** What is read. `id` is the kind's own: a pull request's number, a commit's row, a path, a page's address, the plan's version. `title` names it in a line. */
export type JudgeArtifact = { kind: JudgeKind; id: string; title: string };

/** The press: an empty prompt asks what was missed. */
export type JudgeRequest = { job: string; artifact: JudgeArtifact; prompt: string };

export type JudgeNote = {
  id: string;
  /** The part it is about: a plan group or task id, `path:line` of a diff, a document's paragraph number. Absent, the artifact as a whole. */
  about?: string;
  claim: string;
  /** Where it is, as a person reads it: `auth/middleware.rs:88`. */
  where?: string;
  settled?: "sent" | "dismissed";
};

export type JudgeRecord = {
  id: string;
  job: string;
  artifact: JudgeArtifact;
  prompt: string;
  state: "reading" | "done";
  asked_at: string;
  finished_at?: string;
  notes: readonly JudgeNote[];
  /** The notes were opened. Fleet keeps it with the record. */
  read: boolean;
  /** The id of the thread row it follows, where the thread line stands. */
  after?: string;
};

/** What the renderer asks of Fleet. */
export type JudgeApi = {
  ask: (request: JudgeRequest) => Promise<void>;
  send: (record: string, note: string) => Promise<void>;
  dismiss: (record: string, note: string) => Promise<void>;
  markRead: (record: string) => Promise<void>;
};

export const keyOf = (artifact: Pick<JudgeArtifact, "kind" | "id">): string => `${artifact.kind}:${artifact.id}`;

let api: JudgeApi | undefined;
let records: readonly JudgeRecord[] = [];
const listeners = new Set<() => void>();
const told = () => listeners.forEach((one) => one());
const subscribe = (hear: () => void) => (listeners.add(hear), () => void listeners.delete(hear));

/** Fleet's side, for the mock: the api the renderer asks through, and nothing once it is torn down. */
export function provideJudge(next: JudgeApi | undefined): void {
  api = next;
  if (next === undefined) records = [];
  told();
}

/** A record arrived: it replaces the one for the same Job and artifact. */
export function putRecord(record: JudgeRecord): void {
  records = [...records.filter((one) => one.id !== record.id && !(one.job === record.job && keyOf(one.artifact) === keyOf(record.artifact))), record];
  told();
}

export const useJudgeApi = (): JudgeApi | undefined => useSyncExternalStore(subscribe, () => api);

const allRecords = () => records;

export const useJudgeRecords = (job: string): readonly JudgeRecord[] => {
  const all = useSyncExternalStore(subscribe, allRecords);
  return all.filter((one) => one.job === job);
};

/** The reading of one artifact, if it was asked. */
export function useJudgeRecord(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">): JudgeRecord | undefined {
  const all = useSyncExternalStore(subscribe, allRecords);
  return all.find((one) => one.job === job && keyOf(one.artifact) === keyOf(artifact));
}

// ---- Opening the popover from a key --------------------------------------------------------------------------------------------------------------------

const openers = new Map<string, () => void>();

/** The Judge act of an artifact stands here so a key can open its popover. */
export function registerOpener(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">, open: () => void): () => void {
  const key = `${job}|${keyOf(artifact)}`;
  openers.set(key, open);
  return () => void (openers.get(key) === open && openers.delete(key));
}

export function openJudge(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">): void {
  openers.get(`${job}|${keyOf(artifact)}`)?.();
}
