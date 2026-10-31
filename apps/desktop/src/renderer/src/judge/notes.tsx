// What a judge found, drawn inside the artifact on the part it names. A note is the adversarial review's finding presentation (`FindingNotes`): the heading
// `You should know`, the claim, where it is, and Send to Job or Dismiss. A note settled folds to one muted line. Nothing here counts notes.

import { useEffect, useMemo, type ReactNode } from "react";
import { FindingNotes, useFindingKeys, type AdversarialReviewState, type DiffFile, type Finding } from "@armada/components";

import { useJudgeApi, useJudgeRecord, useJudgeRecords, type JudgeArtifact, type JudgeNote, type JudgeRecord } from "./store";

const NONE: readonly JudgeNote[] = [];

/** A note as the finding row draws it. */
export const findingOf = (note: JudgeNote): Finding => ({
  id: note.id,
  note: true,
  consequence: note.claim,
  ...(note.where === undefined ? {} : { cites: [note.where] }),
  ...(note.about === undefined ? {} : { about: note.about }),
  ...(note.settled === undefined ? {} : { settled: note.settled }),
});

/** A reading as the plan card draws a review: running while it reads, then its notes pinned to the groups and tasks they name. */
export function reviewOf(record: JudgeRecord | undefined): AdversarialReviewState {
  if (record === undefined) return { state: "idle" };
  return record.state === "reading" ? { state: "running" } : { state: "done", findings: record.notes.map(findingOf) };
}

/** The reading of one artifact, with the acts on its notes. The notes are read once something draws them: `markRead` is the caller's to call. */
export function useJudgeNotes(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">) {
  const api = useJudgeApi();
  const record = useJudgeRecord(job, artifact);
  const done = record?.state === "done" ? record : undefined;
  return {
    record: done,
    notes: done?.notes ?? NONE,
    unread: done !== undefined && !done.read,
    send: (note: string) => void (done === undefined ? undefined : api?.send(done.id, note)),
    dismiss: (note: string) => void (done === undefined ? undefined : api?.dismiss(done.id, note)),
    markRead: () => void (done === undefined || done.read ? undefined : api?.markRead(done.id)),
  };
}

/** Keeps the reading's notes read for as long as something draws them. */
export function useNotesRead(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">, drawn: boolean): void {
  const { record, markRead } = useJudgeNotes(job, artifact);
  const unread = record !== undefined && !record.read;
  useEffect(() => {
    if (drawn && unread) markRead();
    // The record is the thing that changes; the act is made fresh each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drawn, unread, record?.id]);
}

/** The notes of a reading that stand under one part, as a list in the artifact. Draws nothing where the part has none. */
export function JudgeNotes({ job, artifact, label, only, draws = true }: { job: string; artifact: Pick<JudgeArtifact, "kind" | "id">; label: string; /** Which notes stand here. Absent, every one. */ only?: (note: JudgeNote) => boolean; draws?: boolean }) {
  const { notes, send, dismiss } = useJudgeNotes(job, artifact);
  const here = notes.filter((one) => only?.(one) ?? true);
  const findings = here.map(findingOf);
  const keys = useFindingKeys({ state: "done", findings }, { onSend: send, onDismiss: dismiss });
  useNotesRead(job, artifact, draws && here.length > 0);
  if (here.length === 0) return null;
  return (
    <div className="judge-notes" {...keys.bind}>
      <FindingNotes findings={findings} label={label} onSend={keys.send} onDismiss={keys.dismiss} />
    </div>
  );
}

/** A note block at the top of a view that has no parts to pin to: the notes whose part is the whole of it. */
export function JudgeNoteBlock({ job, artifact }: { job: string; artifact: Pick<JudgeArtifact, "kind" | "id"> }) {
  return <JudgeNotes job={job} artifact={artifact} label="Notes from the judge" only={(note) => note.about === undefined} />;
}

/** The line a diff's line has in the new file: counted from the hunk headers, since the diff draws no numbers. Absent for a line that is only removed. */
export function newLineAt(file: DiffFile, index: number): number | undefined {
  let at = 0;
  let found: number | undefined;
  for (let i = 0; i <= index && i < file.lines.length; i += 1) {
    const line = file.lines[i]!;
    if (line.kind === "hunk") {
      const start = /\+(\d+)/.exec(line.text)?.[1];
      at = start === undefined ? 0 : Number(start) - 1;
      found = undefined;
    } else if (line.kind === "removed") {
      found = undefined;
    } else {
      at += 1;
      found = at;
    }
  }
  return found;
}

/** The `under` of a diff: the notes whose `path:line` is the line drawn, under it. Absent where the reading has no note on a line, so a diff draws as it did. */
export function useDiffUnder(job: string, artifact: Pick<JudgeArtifact, "kind" | "id">): ((file: DiffFile, index: number) => ReactNode) | undefined {
  const { notes } = useJudgeNotes(job, artifact);
  const { kind, id } = artifact;
  return useMemo(() => {
    if (!notes.some((one) => one.about !== undefined)) return undefined;
    return (file: DiffFile, index: number) => {
      const line = newLineAt(file, index);
      const about = line === undefined ? undefined : `${file.path}:${line}`;
      if (about === undefined || !notes.some((one) => one.about === about)) return null;
      return <JudgeNotes job={job} artifact={{ kind, id }} label={`Notes on ${about}`} only={(note) => note.about === about} />;
    };
  }, [notes, job, kind, id]);
}

/** Kinds that have a view of their own to hold their notes. */
const OWN_VIEW: ReadonlySet<string> = new Set(["plan", "pull_request", "doc"]);

/** The notes of the artifacts that have no view of their own, at the head of the Job's thread: each headed by what it is about. */
export function JudgeLooseNotes({ job }: { job: string }) {
  const records = useJudgeRecords(job).filter((one) => one.state === "done" && !OWN_VIEW.has(one.artifact.kind) && one.notes.length > 0);
  if (records.length === 0) return null;
  return (
    <div className="judge-loose">
      {records.map((one) => (
        <section key={one.id} aria-label={`Notes on ${one.artifact.title}`}>
          <p className="judge-loose__title">{one.artifact.title}</p>
          <JudgeNoteBlock job={job} artifact={one.artifact} />
        </section>
      ))}
    </div>
  );
}
