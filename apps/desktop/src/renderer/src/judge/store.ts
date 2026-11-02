// A judge on demand: Fleet reads one artifact of a Job, once, when the person presses Judge on it, and answers with notes. A Job's readings are
// `SessionRecord.judge_readings` (`packages/protocol/src/judge.ts`), carried by every `session.changed`; the window's Sessions hand them here
// (`putReadings`), and the components read them as records. A reading arrives whole and replaces the one before it for the same artifact.

import { useSyncExternalStore } from "react";
import type {
  JudgeArtifact,
  JudgeKind,
  JudgeNote as WireNote,
  JudgeReading,
} from "@armada/protocol";
import type { SessionJudgeActs } from "@armada/screens/src/draft/sessions";

export type { JudgeArtifact, JudgeKind };

/** The press: an empty prompt asks what was missed. */
export type JudgeRequest = {
  job: string;
  artifact: JudgeArtifact;
  prompt: string;
};

/** A note as the components read it: `settled` is how the person settled it, and absent while it waits. */
export type JudgeNote = Omit<WireNote, "settled"> & {
  settled?: "sent" | "dismissed";
};

/** A reading as the components read it, with the Job it belongs to. */
export type JudgeRecord = Omit<JudgeReading, "notes"> & {
  job: string;
  notes: readonly JudgeNote[];
};

/** What the components ask: `record` is a reading's id, and the Job it belongs to is looked up from the readings held. */
export type JudgeApi = {
  ask: (request: JudgeRequest) => void;
  send: (record: string, note: string) => void;
  dismiss: (record: string, note: string) => void;
  markRead: (record: string) => void;
};

export const keyOf = (artifact: Pick<JudgeArtifact, "kind" | "id">): string =>
  `${artifact.kind}:${artifact.id}`;

/** What `judge_readings` of one Job are to the components. */
export const recordsOf = (
  job: string,
  readings: readonly JudgeReading[] | undefined,
): readonly JudgeRecord[] =>
  (readings ?? []).map((one) => ({
    ...one,
    job,
    notes: one.notes.map(({ settled, ...note }) =>
      settled === undefined
        ? note
        : {
            ...note,
            settled: settled.how,
          },
    ),
  }));

let api: JudgeApi | undefined;
let records: readonly JudgeRecord[] = [];
/** The `judge_readings` each Job was last handed, so a Session that did not change is not read again. */
const handed = new Map<string, readonly JudgeReading[] | undefined>();
const listeners = new Set<() => void>();
const told = () => listeners.forEach((one) => one());
const subscribe = (hear: () => void) => (listeners.add(hear), () => void listeners.delete(hear));

/** The Job a reading belongs to, from the readings held. */
const jobOf = (reading: string): string | undefined =>
  records.find((one) => one.id === reading)?.job;

/** The acts of the window's Sessions as the components ask them. Fleet needs the Job with each; a reading no longer held is no act. */
export function judgeApiOver(acts: SessionJudgeActs): JudgeApi {
  const on = (reading: string, act: (job: string) => void) => {
    const job = jobOf(reading);
    if (job !== undefined) act(job);
  };
  return {
    ask: ({ job, artifact, prompt }) => acts.ask(job, artifact, prompt),
    send: (reading, note) => on(reading, (job) => acts.send(job, reading, note)),
    dismiss: (reading, note) => on(reading, (job) => acts.dismiss(job, reading, note)),
    markRead: (reading) => on(reading, (job) => acts.markRead(job, reading)),
  };
}

/** The window's Sessions serve a Judge, or none (nothing is drawn, and the readings are let go). */
export function provideJudge(next: JudgeApi | undefined): void {
  api = next;
  if (next === undefined) {
    records = [];
    handed.clear();
  }
  told();
}

/** A Job's readings as its record carries them now: they replace the Job's before. */
export function putReadings(job: string, readings: readonly JudgeReading[] | undefined): void {
  if (handed.has(job) && handed.get(job) === readings) return;
  handed.set(job, readings);
  records = [
    ...records.filter((one) => one.job !== job),
    ...recordsOf(job, readings),
  ];
  told();
}

export const useJudgeApi = (): JudgeApi | undefined => useSyncExternalStore(subscribe, () => api);

const allRecords = () => records;

export const useJudgeRecords = (job: string): readonly JudgeRecord[] => {
  const all = useSyncExternalStore(subscribe, allRecords);
  return all.filter((one) => one.job === job);
};

/** The reading of one artifact, if it was asked. */
export function useJudgeRecord(
  job: string,
  artifact: Pick<JudgeArtifact, "kind" | "id">,
): JudgeRecord | undefined {
  const all = useSyncExternalStore(subscribe, allRecords);
  return all.find((one) => one.job === job && keyOf(one.artifact) === keyOf(artifact));
}

// ---- Bringing a page's notes into view ---------------------------------------------------------------------------------------------------------------

let asked: string | undefined;

/** A press asked for this artifact's notes: its page brings them into view once they are drawn, which may be after it opens. */
export function askForNotes(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">): void {
  asked = `${job}|${keyOf(artifact)}`;
  told();
}

/** Whether this artifact's notes were asked for and not yet brought into view. */
export const useNotesAsked = (job: string, artifact: Pick<JudgeArtifact, "kind" | "id">): boolean =>
  useSyncExternalStore(subscribe, () => asked === `${job}|${keyOf(artifact)}`);

export function notesBrought(): void {
  asked = undefined;
  told();
}

// ---- Opening the popover from a key --------------------------------------------------------------------------------------------------------------------

const openers = new Map<string, () => void>();

/** The Judge act of an artifact stands here so a key can open its popover. */
export function registerOpener(
  job: string,
  artifact: Pick<JudgeArtifact, "kind" | "id">,
  open: () => void,
): () => void {
  const key = `${job}|${keyOf(artifact)}`;
  openers.set(key, open);
  return () => void (openers.get(key) === open && openers.delete(key));
}

export function openJudge(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">): void {
  openers.get(`${job}|${keyOf(artifact)}`)?.();
}
